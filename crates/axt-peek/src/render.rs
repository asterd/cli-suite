use std::io::Write;

use axt_core::ErrorCode;
use axt_output::{
    format_agent_fields, AgentCompactWriter, AgentField, JsonEnvelope, JsonlWriter,
    OutputDiagnostic, RenderContext, Renderable, Result as RenderResult,
};
use serde::Serialize;
use serde_json::json;

use crate::model::{Entry, PeekData, PeekWarning, Summary, WarningCode};

#[derive(Debug, Serialize)]
struct JsonlSummary<'a> {
    schema: &'static str,
    #[serde(rename = "type")]
    kind: &'static str,
    ok: bool,
    root: &'a str,
    files: usize,
    dirs: usize,
    bytes: u64,
    git: &'static str,
    modified: usize,
    untracked: usize,
    ignored: usize,
    truncated: bool,
}

#[derive(Debug, Serialize)]
struct JsonlEntry<'a> {
    schema: &'static str,
    #[serde(rename = "type")]
    kind: &'static str,
    path: &'a str,
    bytes: u64,
    lang: Option<&'a str>,
    git: &'static str,
    mtime: Option<&'a str>,
}

#[derive(Debug, Serialize)]
struct JsonlWarning<'a> {
    schema: &'static str,
    #[serde(rename = "type")]
    kind: &'static str,
    code: &'static str,
    path: Option<&'a str>,
    reason: &'a str,
}

impl PeekData {
    pub fn render_json_data(&self, w: &mut dyn Write) -> RenderResult<()> {
        serde_json::to_writer(&mut *w, self)?;
        writeln!(w)?;
        Ok(())
    }
}

impl Renderable for PeekData {
    fn render_human(&self, w: &mut dyn Write, _ctx: &RenderContext<'_>) -> RenderResult<()> {
        writeln!(w, "{}/", self.root)?;
        for entry in &self.entries {
            writeln!(
                w,
                "  {:<32} {:>8}  {:<10} {}",
                display_path(entry),
                human_bytes(entry.bytes),
                entry.language.as_deref().unwrap_or(""),
                entry.git.as_str()
            )?;
        }
        writeln!(w)?;
        writeln!(w, "Summary")?;
        writeln!(
            w,
            "  files     {:<8} modified   {}",
            self.summary.files, self.summary.modified
        )?;
        writeln!(
            w,
            "  dirs      {:<8} untracked  {}",
            self.summary.dirs, self.summary.untracked
        )?;
        writeln!(
            w,
            "  bytes     {:<8} ignored    {}",
            human_bytes(self.summary.bytes),
            self.summary.ignored
        )?;
        writeln!(
            w,
            "  git       {:<8} truncated  {}",
            self.summary.git_state.as_str(),
            yes_no(self.summary.truncated)
        )?;
        Ok(())
    }

    fn render_json(&self, w: &mut dyn Write, _ctx: &RenderContext<'_>) -> RenderResult<()> {
        let envelope = JsonEnvelope::new(
            "axt.peek.v1",
            self,
            json_warnings(&self.warnings),
            Vec::new(),
        );
        serde_json::to_writer(&mut *w, &envelope)?;
        writeln!(w)?;
        Ok(())
    }

    fn render_jsonl(&self, w: &mut dyn Write, ctx: &RenderContext<'_>) -> RenderResult<()> {
        let records = jsonl_records(self);
        let truncated = jsonl_output_truncated(&records, ctx);
        let mut writer = JsonlWriter::new(w, ctx.limits);
        writer.write_record(&jsonl_summary(&self.root, &self.summary, truncated))?;
        for entry in &self.entries {
            writer.write_record(&jsonl_entry(entry))?;
        }
        for warning in &self.warnings {
            writer.write_record(&jsonl_warning(warning))?;
        }
        let _summary = writer.finish("axt.peek.warn.v1")?;
        Ok(())
    }

    fn render_agent(&self, w: &mut dyn Write, ctx: &RenderContext<'_>) -> RenderResult<()> {
        let mut writer = AgentCompactWriter::new(w, ctx.limits);
        let total = self.entries.len();
        let rows = emitted_agent_rows(self, ctx)?;
        let truncated = agent_output_truncated(self, ctx)?;
        writer.write_line(&format!(
            "schema=axt.peek.agent.v1 ok=true mode=table root={} cols=path,kind,bytes,lang,git,mtime rows={} total={} truncated={}",
            agent_value(&self.root),
            rows,
            total,
            truncated
        ))?;
        for entry in &self.entries {
            writer.write_line(&agent_row(entry))?;
        }
        for warning in &self.warnings {
            writer.write_line(&agent_warning(warning)?)?;
        }
        let _summary = writer.finish()?;
        Ok(())
    }
}

fn json_warnings(warnings: &[PeekWarning]) -> Vec<OutputDiagnostic> {
    warnings
        .iter()
        .map(|warning| OutputDiagnostic {
            code: warning_error_code(warning.code),
            message: warning.reason.clone(),
            context: json!({
                "code": warning.code.as_str(),
                "path": warning.path.as_deref(),
            }),
        })
        .collect()
}

const fn warning_error_code(code: WarningCode) -> ErrorCode {
    match code {
        WarningCode::PermissionDenied => ErrorCode::PermissionDenied,
        WarningCode::SymlinkLoop | WarningCode::PathNotUtf8 | WarningCode::GitCapped => {
            ErrorCode::IoError
        }
    }
}

fn jsonl_summary<'a>(root: &'a str, summary: &Summary, truncated: bool) -> JsonlSummary<'a> {
    JsonlSummary {
        schema: "axt.peek.summary.v1",
        kind: "summary",
        ok: true,
        root,
        files: summary.files,
        dirs: summary.dirs,
        bytes: summary.bytes,
        git: summary.git_state.as_str(),
        modified: summary.modified,
        untracked: summary.untracked,
        ignored: summary.ignored,
        truncated,
    }
}

fn emitted_agent_rows(data: &PeekData, ctx: &RenderContext<'_>) -> RenderResult<usize> {
    let header_len = agent_header(data, false, data.entries.len())?.len() + 1;
    let mut bytes = header_len;
    let mut rows = 0;
    for (records, entry) in (1..).zip(data.entries.iter()) {
        let len = agent_row(entry).len() + 1;
        if records >= ctx.limits.max_records || bytes + len > ctx.limits.max_bytes {
            break;
        }
        bytes += len;
        rows += 1;
    }
    Ok(rows)
}

fn agent_output_truncated(data: &PeekData, ctx: &RenderContext<'_>) -> RenderResult<bool> {
    let rows = emitted_agent_rows(data, ctx)?;
    if rows < data.entries.len() {
        return Ok(true);
    }
    let mut bytes = agent_header(data, false, data.entries.len())?.len()
        + 1
        + data
            .entries
            .iter()
            .map(|entry| agent_row(entry).len() + 1)
            .sum::<usize>();
    for (records, warning) in (1 + data.entries.len()..).zip(data.warnings.iter()) {
        let len = agent_warning(warning)?.len() + 1;
        if records >= ctx.limits.max_records || bytes + len > ctx.limits.max_bytes {
            return Ok(true);
        }
        bytes += len;
    }
    Ok(data.entries.is_empty() && data.warnings.is_empty() && bytes > ctx.limits.max_bytes)
}

fn agent_header(data: &PeekData, truncated: bool, rows: usize) -> RenderResult<String> {
    Ok(format!(
        "schema=axt.peek.agent.v1 ok=true mode=table root={} cols=path,kind,bytes,lang,git,mtime rows={} total={} truncated={}",
        agent_value(&data.root),
        rows,
        data.entries.len(),
        truncated
    ))
}

fn jsonl_records(data: &PeekData) -> Vec<Vec<u8>> {
    let mut records = Vec::with_capacity(1 + data.entries.len() + data.warnings.len());
    if let Ok(record) = serde_json::to_vec(&jsonl_summary(&data.root, &data.summary, false)) {
        records.push(record);
    }
    records.extend(
        data.entries
            .iter()
            .map(jsonl_entry)
            .filter_map(|record| serde_json::to_vec(&record).ok()),
    );
    records.extend(
        data.warnings
            .iter()
            .map(jsonl_warning)
            .filter_map(|record| serde_json::to_vec(&record).ok()),
    );
    records
}

fn jsonl_output_truncated(records: &[Vec<u8>], ctx: &RenderContext<'_>) -> bool {
    let mut bytes = 0;
    for (index, record) in records.iter().enumerate() {
        let len = record.len() + 1;
        let first = index == 0;
        if !first && index >= ctx.limits.max_records {
            return true;
        }
        if !first && bytes + len > ctx.limits.max_bytes {
            return true;
        }
        bytes += len;
        if first && (ctx.limits.max_records == 0 || bytes > ctx.limits.max_bytes) {
            return true;
        }
    }
    false
}

fn agent_value(value: &str) -> String {
    let raw = !value.is_empty()
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric()
                || matches!(byte, b'.' | b'_' | b'-' | b'/' | b':' | b'+' | b'@')
        });
    if raw {
        value.to_owned()
    } else {
        serde_json::to_string(value).unwrap_or_else(|_err| "\"\"".to_owned())
    }
}

fn jsonl_entry(entry: &Entry) -> JsonlEntry<'_> {
    JsonlEntry {
        schema: "axt.peek.entry.v1",
        kind: entry.kind.as_str(),
        path: &entry.path,
        bytes: entry.bytes,
        lang: entry.language.as_deref(),
        git: entry.git.as_str(),
        mtime: entry.mtime.as_deref(),
    }
}

fn jsonl_warning(warning: &PeekWarning) -> JsonlWarning<'_> {
    JsonlWarning {
        schema: "axt.peek.warn.v1",
        kind: "warn",
        code: warning.code.as_str(),
        path: warning.path.as_deref(),
        reason: &warning.reason,
    }
}

fn agent_row(entry: &Entry) -> String {
    format!(
        "{},{},{},{},{},{}",
        csv_cell(&entry.path),
        entry.kind.as_str(),
        entry.bytes,
        entry.language.as_deref().unwrap_or(""),
        entry.git.as_str(),
        entry.mtime.as_deref().unwrap_or("")
    )
}

fn agent_warning(warning: &PeekWarning) -> RenderResult<String> {
    let mut fields = vec![
        AgentField::str("code", warning.code.as_str()),
        AgentField::str("reason", &warning.reason),
    ];
    if let Some(path) = warning.path.as_deref() {
        fields.push(AgentField::str("path", path));
    }
    Ok(format!("W {}", format_agent_fields(&fields)?))
}

fn csv_cell(value: &str) -> String {
    if value
        .bytes()
        .any(|byte| matches!(byte, b',' | b'"' | b'\n' | b'\r'))
    {
        let escaped = value.replace('"', "\"\"");
        format!("\"{escaped}\"")
    } else {
        value.to_owned()
    }
}

fn display_path(entry: &Entry) -> String {
    if entry.kind == crate::model::EntryKind::Dir {
        format!("{}/", entry.path)
    } else {
        entry.path.clone()
    }
}

fn human_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

const fn yes_no(value: bool) -> &'static str {
    if value {
        "yes"
    } else {
        "no"
    }
}
