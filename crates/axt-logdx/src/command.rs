use std::{
    collections::{BTreeMap, HashMap},
    fs::File,
    io::{self, BufRead, BufReader},
};

use axt_core::CommandContext;
use camino::{Utf8Path, Utf8PathBuf};
use time::{Date, Month, OffsetDateTime, PrimitiveDateTime, Time};

use crate::{
    cli::Args,
    error::LogdxError,
    model::{
        LogGroup, LogdxData, LogdxSummary, LogdxWarning, Occurrence, Severity, SourceSummary,
        TimelineBucket,
    },
};

const MAX_SNIPPETS_PER_GROUP: usize = 3;
const MAX_STACK_LINES: usize = 24;
const MAX_MESSAGE_CHARS: usize = 240;
const MAX_SNIPPET_CHARS: usize = 500;
const MIN_GROUP_CAPACITY: usize = 512;
const MAX_GROUP_CAPACITY: usize = 8_192;

#[derive(Debug)]
struct Filters {
    min_severity: Severity,
    since: Option<OffsetDateTime>,
    until: Option<OffsetDateTime>,
}

#[derive(Debug, Clone)]
struct PendingRecord {
    source: String,
    line: usize,
    severity: Severity,
    timestamp: Option<OffsetDateTime>,
    message: String,
    snippet: String,
    stack: Vec<String>,
}

#[derive(Debug)]
struct GroupState {
    fingerprint: String,
    severity: Severity,
    count: usize,
    first: Occurrence,
    last: Occurrence,
    message: String,
    stack: Vec<String>,
    snippets: Vec<String>,
}

#[derive(Debug)]
struct Collector {
    filters: Filters,
    year: i32,
    group_capacity: usize,
    groups: HashMap<String, GroupState>,
    timeline: BTreeMap<String, TimelineCounts>,
    warnings: Vec<LogdxWarning>,
    summary_lines: usize,
    summary_errors: usize,
    bytes_scanned: u64,
    truncated: bool,
    evicted_groups: usize,
    time_filter_unparseable: usize,
    invalid_utf8_lines: usize,
}

#[derive(Debug, Default, Clone)]
struct TimelineCounts {
    trace: usize,
    debug: usize,
    info: usize,
    warn: usize,
    error: usize,
    fatal: usize,
}

pub fn run(args: &Args, ctx: &CommandContext) -> Result<LogdxData, LogdxError> {
    if args.paths.is_empty() && !args.stdin {
        return Err(LogdxError::NoInput);
    }

    let filters = Filters {
        min_severity: args.severity.into(),
        since: parse_filter_time("since", args.since.as_deref())?,
        until: parse_filter_time("until", args.until.as_deref())?,
    };
    let retained = retained_group_limit(args.top, ctx.limits.max_records);
    let mut collector = Collector::new(
        filters,
        ctx.clock.now_utc().year(),
        group_capacity(retained),
    );
    let mut sources = Vec::new();

    for path in &args.paths {
        let resolved = resolve_path(&ctx.cwd, path);
        if !resolved.exists() {
            return Err(LogdxError::PathNotFound(resolved));
        }
        let file = File::open(&resolved).map_err(|source| LogdxError::Io {
            path: resolved.to_string(),
            source,
        })?;
        let summary = collector.read_source(path.as_str().to_owned(), BufReader::new(file))?;
        sources.push(summary);
    }

    if args.stdin {
        let stdin = io::stdin();
        let summary = collector.read_source("<stdin>".to_owned(), stdin.lock())?;
        sources.push(summary);
    }

    Ok(collector.finish(sources, args.top, ctx.limits.max_records))
}

#[allow(dead_code)]
pub fn fuzz_parse_bytes(bytes: &[u8]) {
    let filters = Filters {
        min_severity: Severity::Trace,
        since: None,
        until: None,
    };
    let mut collector = Collector::new(filters, 2026, 128);
    let _summary = collector.read_source("fuzz.log".to_owned(), std::io::Cursor::new(bytes));
}

impl Collector {
    fn new(filters: Filters, year: i32, group_capacity: usize) -> Self {
        Self {
            filters,
            year,
            group_capacity,
            groups: HashMap::new(),
            timeline: BTreeMap::new(),
            warnings: Vec::new(),
            summary_lines: 0,
            summary_errors: 0,
            bytes_scanned: 0,
            truncated: false,
            evicted_groups: 0,
            time_filter_unparseable: 0,
            invalid_utf8_lines: 0,
        }
    }

    fn read_source<R: BufRead>(
        &mut self,
        source_name: String,
        mut reader: R,
    ) -> Result<SourceSummary, LogdxError> {
        let mut line = Vec::new();
        let mut line_no = 0usize;
        let mut bytes = 0u64;
        let mut pending: Option<PendingRecord> = None;
        let year = self.year;

        loop {
            line.clear();
            let read = reader
                .read_until(b'\n', &mut line)
                .map_err(|source| LogdxError::Io {
                    path: source_name.clone(),
                    source,
                })?;
            if read == 0 {
                break;
            }
            line_no = line_no.saturating_add(1);
            bytes = bytes.saturating_add(read as u64);
            self.bytes_scanned = self.bytes_scanned.saturating_add(read as u64);
            self.summary_lines = self.summary_lines.saturating_add(1);

            let line_bytes = trim_line_bytes(&line);
            let decoded = axt_fs::decode_text_smart(line_bytes);
            if decoded.lossy || decoded.converted {
                self.invalid_utf8_lines = self.invalid_utf8_lines.saturating_add(1);
            }
            let normalized = strip_ansi(&decoded.text);
            if pending.is_some() && is_stack_continuation(&normalized, year) {
                if let Some(record) = pending.as_mut() {
                    if record.stack.len() < MAX_STACK_LINES {
                        record
                            .stack
                            .push(truncate_chars(&normalized, MAX_SNIPPET_CHARS));
                    }
                }
            } else if let Some(record) = parse_record(&source_name, line_no, &normalized, year) {
                if let Some(previous) = pending.take() {
                    self.add_record(&previous);
                }
                pending = Some(record);
            }
        }

        if let Some(record) = pending {
            self.add_record(&record);
        }

        Ok(SourceSummary {
            path: source_name,
            lines: line_no,
            bytes,
        })
    }

    fn add_record(&mut self, record: &PendingRecord) {
        if !self.record_passes_filters(record) {
            return;
        }
        if record.severity.is_error() {
            self.summary_errors = self.summary_errors.saturating_add(1);
        }
        if let Some(timestamp) = record.timestamp {
            self.add_timeline(timestamp, record.severity);
        }

        let fingerprint = fingerprint(record.severity, &record.message, &record.stack);
        let occurrence = Occurrence {
            source: record.source.clone(),
            line: record.line,
            timestamp: record.timestamp.map(format_time),
        };
        if self.group_capacity == 0 {
            self.truncated = true;
            return;
        }

        let base_count = if !self.groups.contains_key(&fingerprint)
            && self.groups.len() >= self.group_capacity
        {
            self.replace_cold_group(&fingerprint).unwrap_or(0)
        } else {
            0
        };

        let entry = self
            .groups
            .entry(fingerprint.clone())
            .or_insert_with(|| GroupState {
                fingerprint,
                severity: record.severity,
                count: base_count,
                first: occurrence.clone(),
                last: occurrence.clone(),
                message: truncate_chars(&record.message, MAX_MESSAGE_CHARS),
                stack: record.stack.clone(),
                snippets: Vec::new(),
            });
        entry.count = entry.count.saturating_add(1);
        entry.last = occurrence;
        if entry.snippets.len() < MAX_SNIPPETS_PER_GROUP {
            entry
                .snippets
                .push(truncate_chars(&record.snippet, MAX_SNIPPET_CHARS));
        }
    }

    fn record_passes_filters(&mut self, record: &PendingRecord) -> bool {
        if record.severity.rank() < self.filters.min_severity.rank() {
            return false;
        }
        if self.filters.since.is_none() && self.filters.until.is_none() {
            return true;
        }
        let Some(timestamp) = record.timestamp else {
            self.time_filter_unparseable = self.time_filter_unparseable.saturating_add(1);
            return false;
        };
        if let Some(since) = self.filters.since {
            if timestamp < since {
                return false;
            }
        }
        if let Some(until) = self.filters.until {
            if timestamp > until {
                return false;
            }
        }
        true
    }

    fn replace_cold_group(&mut self, replacement: &str) -> Option<usize> {
        let key = self
            .groups
            .iter()
            .min_by(|(_, left), (_, right)| {
                left.count
                    .cmp(&right.count)
                    .then_with(|| left.severity.rank().cmp(&right.severity.rank()))
                    .then_with(|| right.last.source.cmp(&left.last.source))
                    .then_with(|| right.last.line.cmp(&left.last.line))
                    .then_with(|| right.fingerprint.cmp(&left.fingerprint))
            })
            .map(|(key, _)| key.clone())?;
        let removed = self.groups.remove(&key)?;
        if key != replacement {
            self.evicted_groups = self.evicted_groups.saturating_add(1);
            self.truncated = true;
        }
        Some(removed.count)
    }

    fn add_timeline(&mut self, timestamp: OffsetDateTime, severity: Severity) {
        let bucket = format_time(timestamp.replace_second(0).unwrap_or(timestamp));
        let counts = self.timeline.entry(bucket).or_default();
        match severity {
            Severity::Trace => counts.trace = counts.trace.saturating_add(1),
            Severity::Debug => counts.debug = counts.debug.saturating_add(1),
            Severity::Info => counts.info = counts.info.saturating_add(1),
            Severity::Warn => counts.warn = counts.warn.saturating_add(1),
            Severity::Error => counts.error = counts.error.saturating_add(1),
            Severity::Fatal => counts.fatal = counts.fatal.saturating_add(1),
        }
    }

    fn finish(mut self, sources: Vec<SourceSummary>, top: usize, limit: usize) -> LogdxData {
        if self.evicted_groups > 0 {
            self.warnings.push(LogdxWarning {
                code: crate::model::WarningCode::InputTruncated,
                path: None,
                message: format!(
                    "bounded aggregation evicted {} low-frequency groups before final ranking",
                    self.evicted_groups
                ),
            });
        }
        if self.time_filter_unparseable > 0 {
            self.warnings.push(LogdxWarning {
                code: crate::model::WarningCode::TimeUnparseable,
                path: None,
                message: format!(
                    "{} records matched severity but were excluded because their timestamps were unparseable under the active time filter",
                    self.time_filter_unparseable
                ),
            });
        }
        if self.invalid_utf8_lines > 0 {
            self.warnings.push(LogdxWarning {
                code: crate::model::WarningCode::InvalidUtf8,
                path: None,
                message: format!(
                    "{} lines contained invalid UTF-8 and were decoded lossily",
                    self.invalid_utf8_lines
                ),
            });
        }

        let mut groups = self
            .groups
            .into_values()
            .map(|group| LogGroup {
                fingerprint: group.fingerprint,
                severity: group.severity,
                count: group.count,
                first: group.first,
                last: group.last,
                message: group.message,
                stack: group.stack,
                snippets: group.snippets,
            })
            .collect::<Vec<_>>();
        groups.sort_by(|left, right| {
            right
                .count
                .cmp(&left.count)
                .then_with(|| right.severity.rank().cmp(&left.severity.rank()))
                .then_with(|| left.first.source.cmp(&right.first.source))
                .then_with(|| left.first.line.cmp(&right.first.line))
                .then_with(|| left.fingerprint.cmp(&right.fingerprint))
        });
        let retained = retained_group_limit(top, limit);
        if groups.len() > retained {
            groups.truncate(retained);
            self.truncated = true;
        }
        let timeline = self
            .timeline
            .into_iter()
            .map(|(bucket, counts)| TimelineBucket {
                bucket,
                trace: counts.trace,
                debug: counts.debug,
                info: counts.info,
                warn: counts.warn,
                error: counts.error,
                fatal: counts.fatal,
            })
            .collect::<Vec<_>>();
        let next = sources
            .first()
            .map(|source| {
                vec![format!(
                    "axt-logdx {} --severity error --top 20 --agent",
                    source.path
                )]
            })
            .unwrap_or_default();
        let summary = LogdxSummary {
            lines: self.summary_lines,
            groups: groups.len(),
            errors: self.summary_errors,
            warnings: self.warnings.len(),
            bytes_scanned: self.bytes_scanned,
            truncated: self.truncated,
        };
        LogdxData {
            sources,
            summary,
            groups,
            timeline,
            warnings: self.warnings,
            next,
        }
    }
}

fn resolve_path(cwd: &Utf8Path, path: &Utf8Path) -> Utf8PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        cwd.join(path)
    }
}

const fn retained_group_limit(top: usize, limit: usize) -> usize {
    if top < limit {
        top
    } else {
        limit
    }
}

fn group_capacity(retained: usize) -> usize {
    if retained == 0 {
        0
    } else {
        retained
            .saturating_mul(64)
            .clamp(MIN_GROUP_CAPACITY, MAX_GROUP_CAPACITY)
    }
}

fn trim_line_bytes(line: &[u8]) -> &[u8] {
    let without_lf = line.strip_suffix(b"\n").unwrap_or(line);
    without_lf.strip_suffix(b"\r").unwrap_or(without_lf)
}

fn parse_filter_time(
    field: &'static str,
    value: Option<&str>,
) -> Result<Option<OffsetDateTime>, LogdxError> {
    value
        .map(|raw| {
            OffsetDateTime::parse(raw, &time::format_description::well_known::Rfc3339).map_err(
                |_| LogdxError::InvalidTime {
                    field,
                    value: raw.to_owned(),
                },
            )
        })
        .transpose()
}

fn parse_record(source: &str, line: usize, text: &str, year: i32) -> Option<PendingRecord> {
    if text.trim().is_empty() {
        return None;
    }
    if let Some(record) = parse_json_record(source, line, text, year) {
        return Some(record);
    }
    if let Some(record) = parse_key_value_record(source, line, text, year) {
        return Some(record);
    }
    parse_plain_record(source, line, text, year)
}

fn parse_json_record(source: &str, line: usize, text: &str, year: i32) -> Option<PendingRecord> {
    let value = serde_json::from_str::<serde_json::Value>(text).ok()?;
    let object = value.as_object()?;
    if let Some(log_text) = json_string(object, &[&["log"], &["message", "log"]]) {
        if let Some(mut record) = parse_plain_record(source, line, log_text.trim_end(), year) {
            record.timestamp = json_timestamp(object).or(record.timestamp);
            text.clone_into(&mut record.snippet);
            return Some(record);
        }
    }
    let severity = json_string(
        object,
        &[
            &["level"],
            &["severity"],
            &["log.level"],
            &["severity_text"],
            &["severityText"],
            &["log", "level"],
            &["level", "name"],
            &["jsonPayload", "level"],
            &["jsonPayload", "severity"],
            &["jsonPayload", "log", "level"],
            &["attributes", "level"],
            &["attributes", "log.level"],
        ],
    )
    .and_then(parse_severity)
    .or_else(|| {
        json_string(object, &[&["stream"]])
            .filter(|stream| stream.eq_ignore_ascii_case("stderr"))
            .map(|_| Severity::Error)
    })?;
    let message = normalize_message(
        json_string(
            object,
            &[
                &["message"],
                &["msg"],
                &["error"],
                &["err"],
                &["event"],
                &["body"],
                &["log"],
                &["textPayload"],
                &["jsonPayload", "message"],
                &["jsonPayload", "msg"],
                &["jsonPayload", "error"],
                &["attributes", "message"],
                &["event", "original"],
            ],
        )
        .unwrap_or(text),
    );
    let timestamp = json_timestamp(object);
    Some(PendingRecord {
        source: source.to_owned(),
        line,
        severity,
        timestamp,
        message,
        snippet: text.to_owned(),
        stack: json_stack_lines(object),
    })
}

fn json_string<'a>(
    object: &'a serde_json::Map<String, serde_json::Value>,
    paths: &[&[&str]],
) -> Option<&'a str> {
    paths
        .iter()
        .find_map(|path| json_value_at_path(object, path).and_then(serde_json::Value::as_str))
}

fn json_value_at_path<'a>(
    object: &'a serde_json::Map<String, serde_json::Value>,
    path: &[&str],
) -> Option<&'a serde_json::Value> {
    let (first, rest) = path.split_first()?;
    let mut value = object.get(*first)?;
    for key in rest {
        value = value.as_object()?.get(*key)?;
    }
    Some(value)
}

fn json_timestamp(object: &serde_json::Map<String, serde_json::Value>) -> Option<OffsetDateTime> {
    if let Some(timestamp) = json_string(
        object,
        &[
            &["timestamp"],
            &["time"],
            &["ts"],
            &["@timestamp"],
            &["datetime"],
            &["date"],
            &["jsonPayload", "timestamp"],
            &["jsonPayload", "time"],
            &["attributes", "timestamp"],
            &["attributes", "time"],
        ],
    )
    .and_then(parse_flexible_timestamp)
    {
        return Some(timestamp);
    }
    json_number(
        object,
        &[
            &["timestamp"],
            &["time"],
            &["ts"],
            &["@timestamp"],
            &["epoch"],
            &["epoch_ms"],
            &["jsonPayload", "timestamp"],
            &["jsonPayload", "time"],
            &["attributes", "timestamp"],
            &["attributes", "time"],
        ],
    )
    .and_then(parse_epoch_number)
}

fn json_stack_lines(object: &serde_json::Map<String, serde_json::Value>) -> Vec<String> {
    let paths = [
        &["stack"][..],
        &["stacktrace"],
        &["stack_trace"],
        &["error.stack_trace"],
        &["exception", "stacktrace"],
        &["exception", "stack_trace"],
        &["error", "stack"],
        &["error", "stack_trace"],
        &["jsonPayload", "stack"],
        &["jsonPayload", "stack_trace"],
        &["jsonPayload", "error", "stack_trace"],
    ];
    for path in paths {
        if let Some(value) = json_value_at_path(object, path) {
            if let Some(stack) = value.as_str() {
                return stack.lines().map(ToOwned::to_owned).collect();
            }
            if let Some(lines) = stack_lines_from_array(value) {
                return lines;
            }
        }
    }
    Vec::new()
}

fn stack_lines_from_array(value: &serde_json::Value) -> Option<Vec<String>> {
    let array = value.as_array()?;
    let lines = array
        .iter()
        .filter_map(|item| {
            item.as_str().map(ToOwned::to_owned).or_else(|| {
                item.as_object().and_then(|object| {
                    json_string(
                        object,
                        &[&["line"], &["frame"], &["function"], &["method"], &["file"]],
                    )
                    .map(ToOwned::to_owned)
                })
            })
        })
        .collect::<Vec<_>>();
    if lines.is_empty() {
        None
    } else {
        Some(lines)
    }
}

fn json_number(
    object: &serde_json::Map<String, serde_json::Value>,
    paths: &[&[&str]],
) -> Option<i64> {
    paths
        .iter()
        .find_map(|path| json_value_at_path(object, path).and_then(serde_json::Value::as_i64))
}

fn parse_key_value_record(
    source: &str,
    line: usize,
    text: &str,
    year: i32,
) -> Option<PendingRecord> {
    if !text.contains('=') {
        return None;
    }
    let severity = key_value(text, &["level", "severity", "log.level"])
        .and_then(parse_severity)
        .or_else(|| detect_severity(text))?;
    let message = key_value(text, &["msg", "message", "error", "err"])
        .map_or_else(|| normalize_message(text), normalize_message);
    let timestamp = key_value(text, &["ts", "time", "timestamp"])
        .and_then(parse_flexible_timestamp)
        .or_else(|| detect_timestamp(text, year));
    Some(PendingRecord {
        source: source.to_owned(),
        line,
        severity,
        timestamp,
        message,
        snippet: text.to_owned(),
        stack: Vec::new(),
    })
}

fn key_value<'a>(text: &'a str, keys: &[&str]) -> Option<&'a str> {
    let bytes = text.as_bytes();
    let mut start = 0usize;
    while start < bytes.len() {
        start = next_char_boundary(text, start);
        while start < bytes.len() && bytes[start].is_ascii_whitespace() {
            start += 1;
        }
        let Some(eq_offset) = text[start..].find('=') else {
            break;
        };
        let eq = start + eq_offset;
        let key = text[start..eq].trim();
        let mut value_start = eq.saturating_add(1);
        if value_start >= bytes.len() {
            return None;
        }
        let value;
        if bytes[value_start] == b'"' {
            value_start += 1;
            let end = text[value_start..]
                .find('"')
                .map_or(bytes.len(), |offset| value_start + offset);
            value = &text[value_start..end];
            start = next_char_boundary(text, end.saturating_add(1));
        } else {
            let end = text[value_start..]
                .find(char::is_whitespace)
                .map_or(bytes.len(), |offset| value_start + offset);
            value = &text[value_start..end];
            start = next_char_boundary(text, end.saturating_add(1));
        }
        if keys.iter().any(|wanted| key.eq_ignore_ascii_case(wanted)) {
            return Some(value);
        }
    }
    None
}

fn next_char_boundary(text: &str, mut index: usize) -> usize {
    while index < text.len() && !text.is_char_boundary(index) {
        index = index.saturating_add(1);
    }
    index
}

fn parse_plain_record(source: &str, line: usize, text: &str, year: i32) -> Option<PendingRecord> {
    let severity = detect_severity(text)?;
    let timestamp = detect_timestamp(text, year);
    let message = normalize_message(text);
    Some(PendingRecord {
        source: source.to_owned(),
        line,
        severity,
        timestamp,
        message,
        snippet: text.to_owned(),
        stack: Vec::new(),
    })
}

fn detect_severity(text: &str) -> Option<Severity> {
    let lowered = text.to_ascii_lowercase();
    for token in lowered
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|token| !token.is_empty())
    {
        if let Some(severity) = parse_severity(token) {
            return Some(severity);
        }
    }
    if lowered.contains("panic") || lowered.contains("exception") || lowered.contains("failed") {
        return Some(Severity::Error);
    }
    None
}

fn parse_severity(raw: &str) -> Option<Severity> {
    match raw.to_ascii_lowercase().as_str() {
        "trace" => Some(Severity::Trace),
        "debug" => Some(Severity::Debug),
        "info" | "information" | "notice" => Some(Severity::Info),
        "warn" | "warning" => Some(Severity::Warn),
        "error" | "err" => Some(Severity::Error),
        "fatal" | "critical" | "crit" | "panic" => Some(Severity::Fatal),
        _ => None,
    }
}

fn detect_timestamp(text: &str, year: i32) -> Option<OffsetDateTime> {
    text.split_whitespace()
        .find_map(parse_flexible_timestamp)
        .or_else(|| parse_syslog_timestamp(text, year))
        .or_else(|| parse_slash_timestamp(text))
        .or_else(|| parse_apache_timestamp(text))
}

fn parse_flexible_timestamp(raw: &str) -> Option<OffsetDateTime> {
    parse_rfc3339(raw).or_else(|| parse_epoch_timestamp(raw))
}

fn parse_rfc3339(raw: &str) -> Option<OffsetDateTime> {
    let trimmed = trim_timestamp_token(raw);
    OffsetDateTime::parse(trimmed, &time::format_description::well_known::Rfc3339).ok()
}

fn parse_epoch_timestamp(raw: &str) -> Option<OffsetDateTime> {
    let trimmed = trim_timestamp_token(raw);
    if let Some((whole, fraction)) = trimmed.split_once('.') {
        if !matches!(whole.len(), 10 | 13)
            || !whole.bytes().all(|byte| byte.is_ascii_digit())
            || fraction.is_empty()
            || !fraction.bytes().all(|byte| byte.is_ascii_digit())
        {
            return None;
        }
        return parse_fractional_epoch(whole, fraction);
    }
    if !matches!(trimmed.len(), 10 | 13) || !trimmed.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    parse_epoch_number(trimmed.parse::<i64>().ok()?)
}

fn trim_timestamp_token(raw: &str) -> &str {
    let trimmed = raw.trim_end_matches([',', ';']);
    if trimmed.len() >= 2 {
        let bytes = trimmed.as_bytes();
        let paired = matches!(
            (bytes[0], bytes[trimmed.len() - 1]),
            (b'[', b']') | (b'"', b'"') | (b'\'', b'\'')
        );
        if paired {
            return &trimmed[1..trimmed.len() - 1];
        }
    }
    trimmed
}

fn parse_fractional_epoch(whole: &str, fraction: &str) -> Option<OffsetDateTime> {
    let whole = whole.parse::<i128>().ok()?;
    let unit_nanos = if whole >= 10_000_000_000 {
        1_000_000
    } else {
        1_000_000_000
    };
    let mut nanos = whole.checked_mul(unit_nanos)?;
    let mut fractional_nanos = 0_i128;
    let mut scale = unit_nanos / 10;
    for digit in fraction.bytes().take(9) {
        fractional_nanos += i128::from(digit - b'0') * scale;
        scale /= 10;
    }
    nanos = nanos.checked_add(fractional_nanos)?;
    OffsetDateTime::from_unix_timestamp_nanos(nanos).ok()
}

fn parse_epoch_number(raw: i64) -> Option<OffsetDateTime> {
    if raw >= 10_000_000_000 {
        OffsetDateTime::from_unix_timestamp_nanos(i128::from(raw) * 1_000_000).ok()
    } else {
        OffsetDateTime::from_unix_timestamp(raw).ok()
    }
}

fn parse_syslog_timestamp(text: &str, year: i32) -> Option<OffsetDateTime> {
    let mut parts = text.split_whitespace();
    let month = parse_month(parts.next()?)?;
    let day = parts.next()?.parse::<u8>().ok()?;
    let time_text = parts.next()?;
    let mut time_parts = time_text.split(':');
    let hour = time_parts.next()?.parse::<u8>().ok()?;
    let minute = time_parts.next()?.parse::<u8>().ok()?;
    let second = time_parts.next()?.parse::<u8>().ok()?;
    let date = Date::from_calendar_date(year, month, day).ok()?;
    let time = Time::from_hms(hour, minute, second).ok()?;
    Some(PrimitiveDateTime::new(date, time).assume_utc())
}

fn parse_slash_timestamp(text: &str) -> Option<OffsetDateTime> {
    let mut parts = text.split_whitespace();
    let date_text = parts.next()?;
    let time_text = parts.next()?;
    let mut date_parts = date_text.split('/');
    let year = date_parts.next()?.parse::<i32>().ok()?;
    let month = month_from_number(date_parts.next()?.parse::<u8>().ok()?)?;
    let day = date_parts.next()?.parse::<u8>().ok()?;
    let time = parse_hms(time_text)?;
    let date = Date::from_calendar_date(year, month, day).ok()?;
    Some(PrimitiveDateTime::new(date, time).assume_utc())
}

fn parse_apache_timestamp(text: &str) -> Option<OffsetDateTime> {
    let start = text.find('[')?;
    let end = text[start..].find(']').map(|offset| start + offset)?;
    let inner = &text[start + 1..end];
    let mut parts = inner.split_whitespace();
    let _weekday = parts.next()?;
    let month = parse_month(parts.next()?)?;
    let day = parts.next()?.parse::<u8>().ok()?;
    let time_text = parts.next()?;
    let year = parts.next()?.parse::<i32>().ok()?;
    let time = parse_hms(time_text.split('.').next().unwrap_or(time_text))?;
    let date = Date::from_calendar_date(year, month, day).ok()?;
    Some(PrimitiveDateTime::new(date, time).assume_utc())
}

fn parse_hms(time_text: &str) -> Option<Time> {
    let mut time_parts = time_text.split(':');
    let hour = time_parts.next()?.parse::<u8>().ok()?;
    let minute = time_parts.next()?.parse::<u8>().ok()?;
    let second = time_parts.next()?.parse::<u8>().ok()?;
    Time::from_hms(hour, minute, second).ok()
}

const fn month_from_number(month: u8) -> Option<Month> {
    match month {
        1 => Some(Month::January),
        2 => Some(Month::February),
        3 => Some(Month::March),
        4 => Some(Month::April),
        5 => Some(Month::May),
        6 => Some(Month::June),
        7 => Some(Month::July),
        8 => Some(Month::August),
        9 => Some(Month::September),
        10 => Some(Month::October),
        11 => Some(Month::November),
        12 => Some(Month::December),
        _ => None,
    }
}

fn parse_month(raw: &str) -> Option<Month> {
    match raw {
        "Jan" => Some(Month::January),
        "Feb" => Some(Month::February),
        "Mar" => Some(Month::March),
        "Apr" => Some(Month::April),
        "May" => Some(Month::May),
        "Jun" => Some(Month::June),
        "Jul" => Some(Month::July),
        "Aug" => Some(Month::August),
        "Sep" => Some(Month::September),
        "Oct" => Some(Month::October),
        "Nov" => Some(Month::November),
        "Dec" => Some(Month::December),
        _ => None,
    }
}

fn is_stack_line(text: &str) -> bool {
    let trimmed = text.trim_start();
    trimmed.starts_with("at ")
        || trimmed.starts_with("File \"")
        || trimmed.starts_with("Traceback ")
        || trimmed.starts_with("stack backtrace:")
        || trimmed.starts_with("goroutine ")
        || trimmed.starts_with("Caused by:")
        || trimmed.starts_with("Suppressed:")
        || trimmed.starts_with("Error:")
        || trimmed.starts_with("Exception:")
        || trimmed.starts_with("RuntimeError:")
        || trimmed.starts_with("TypeError:")
        || trimmed.starts_with("ValueError:")
        || trimmed.starts_with("java.")
        || trimmed.starts_with("javax.")
        || trimmed.starts_with("org.")
        || trimmed.starts_with("com.")
        || trimmed.starts_with("...")
        || trimmed.contains(".rs:")
        || trimmed.contains(".go:")
        || trimmed.contains(".js:")
        || trimmed.contains(".py:")
        || trimmed.contains(".java:")
}

fn is_stack_continuation(text: &str, year: i32) -> bool {
    let trimmed = text.trim_start();
    !trimmed.starts_with('{')
        && !trimmed.starts_with('[')
        && !text.contains('=')
        && detect_timestamp(text, year).is_none()
        && is_stack_line(text)
}

fn normalize_message(text: &str) -> String {
    let mut out = String::new();
    let mut previous_digit = false;
    for ch in text.chars() {
        if ch.is_ascii_digit() {
            if !previous_digit {
                out.push('#');
            }
            previous_digit = true;
        } else {
            previous_digit = false;
            out.push(ch);
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn fingerprint(severity: Severity, message: &str, stack: &[String]) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(severity.as_str().as_bytes());
    hasher.update(b"\0");
    hasher.update(message.as_bytes());
    for line in stack.iter().take(3) {
        hasher.update(b"\0");
        hasher.update(line.as_bytes());
    }
    let hex = hasher.finalize().to_hex().to_string();
    let short = hex.get(..16).unwrap_or(hex.as_str());
    format!("blake3:{short}")
}

fn strip_ansi(text: &str) -> String {
    let mut output = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' && chars.peek().is_some_and(|next| *next == '[') {
            let _discarded = chars.next();
            for seq in chars.by_ref() {
                if seq.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            output.push(ch);
        }
    }
    output
}

fn truncate_chars(text: &str, max_chars: usize) -> String {
    let mut chars = text.chars();
    let truncated = chars.by_ref().take(max_chars).collect::<String>();
    if chars.next().is_some() {
        format!("{truncated}...")
    } else {
        truncated
    }
}

fn format_time(value: OffsetDateTime) -> String {
    value
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| value.unix_timestamp().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn normalizes_digits_for_dedup() {
        assert_eq!(
            normalize_message("ERROR user 123 failed at line 45"),
            "ERROR user # failed at line #"
        );
    }

    #[test]
    fn strips_ansi_sequences() {
        assert_eq!(
            strip_ansi("\u{1b}[31mERROR\u{1b}[0m failed"),
            "ERROR failed"
        );
    }

    #[test]
    fn parses_syslog_timestamp() {
        let parsed = detect_timestamp("Apr 28 10:00:01 host app ERROR failed", 2026);
        assert!(parsed.is_some());
        assert_eq!(parsed.map(OffsetDateTime::year), Some(2026));
    }

    proptest! {
        #[test]
        fn parser_accepts_arbitrary_bytes_without_panicking(bytes in proptest::collection::vec(any::<u8>(), 0..4096)) {
            let filters = Filters {
                min_severity: Severity::Trace,
                since: None,
                until: None,
            };
            let mut collector = Collector::new(filters, 2026, 128);
            let _summary = collector.read_source("fuzz.log".to_owned(), std::io::Cursor::new(bytes));
        }
    }
}
