//! Internal use only, no stability guarantees.

#![deny(unsafe_code)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![allow(
    clippy::missing_errors_doc,
    clippy::module_name_repetitions,
    clippy::option_if_let_else
)]

use std::{
    fmt::Write as FmtWrite,
    io::{self, Write},
};

use anstyle::{AnsiColor, Style};
use ax_core::{Clock, ColorChoice, ErrorCode, OutputLimits, OutputMode};
use serde::Serialize;
use serde_json::Value;
use thiserror::Error;

/// Output helper error.
#[derive(Debug, Error)]
pub enum OutputError {
    /// An IO write failed.
    #[error("failed to write output: {0}")]
    Io(#[from] io::Error),

    /// JSON serialization failed.
    #[error("failed to serialize output: {0}")]
    Json(#[from] serde_json::Error),

    /// Output was truncated while strict mode was active.
    #[error("output was truncated while --strict was active")]
    TruncatedStrict,
}

/// Output helper result type.
pub type Result<T> = std::result::Result<T, OutputError>;

/// Rendering context shared by command-specific renderers.
#[derive(Debug, Clone, Copy)]
pub struct RenderContext<'a> {
    /// Selected output mode.
    pub mode: OutputMode,
    /// Output limits.
    pub limits: OutputLimits,
    /// Resolved color choice.
    pub color: ColorChoice,
    /// Clock for timestamp-producing output.
    pub clock: &'a dyn Clock,
}

impl<'a> RenderContext<'a> {
    /// Create a render context.
    #[must_use]
    pub const fn new(
        mode: OutputMode,
        limits: OutputLimits,
        color: ColorChoice,
        clock: &'a dyn Clock,
    ) -> Self {
        Self {
            mode,
            limits,
            color,
            clock,
        }
    }
}

/// Command output rendering contract.
pub trait Renderable {
    /// Render human or plain output.
    fn render_human(&self, w: &mut dyn Write, ctx: &RenderContext<'_>) -> Result<()>;

    /// Render JSON output.
    fn render_json(&self, w: &mut dyn Write, ctx: &RenderContext<'_>) -> Result<()>;

    /// Render JSONL output.
    fn render_jsonl(&self, w: &mut dyn Write, ctx: &RenderContext<'_>) -> Result<()>;

    /// Render agent ACF output.
    fn render_agent(&self, w: &mut dyn Write, ctx: &RenderContext<'_>) -> Result<()>;
}

/// Diagnostic item used by JSON envelopes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OutputDiagnostic {
    /// Standard catalog error or warning code.
    pub code: ErrorCode,
    /// Human-readable message.
    pub message: String,
    /// Echo of the relevant failing input.
    pub context: Value,
}

/// Standard JSON envelope for command output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct JsonEnvelope<T> {
    /// Versioned command schema.
    pub schema: String,
    /// Success marker.
    pub ok: bool,
    /// Command-specific payload.
    pub data: T,
    /// Non-fatal diagnostics.
    pub warnings: Vec<OutputDiagnostic>,
    /// Fatal diagnostics.
    pub errors: Vec<OutputDiagnostic>,
}

impl<T> JsonEnvelope<T> {
    /// Build a successful envelope.
    #[must_use]
    pub fn new(
        schema: impl Into<String>,
        data: T,
        warnings: Vec<OutputDiagnostic>,
        errors: Vec<OutputDiagnostic>,
    ) -> Self {
        Self {
            schema: schema.into(),
            ok: true,
            data,
            warnings,
            errors,
        }
    }

    /// Build an envelope with an explicit `ok` value.
    #[must_use]
    pub fn with_status(
        schema: impl Into<String>,
        ok: bool,
        data: T,
        warnings: Vec<OutputDiagnostic>,
        errors: Vec<OutputDiagnostic>,
    ) -> Self {
        Self {
            schema: schema.into(),
            ok,
            data,
            warnings,
            errors,
        }
    }
}

/// Why line-oriented output was truncated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TruncationReason {
    /// Record count limit was reached.
    MaxRecords,
    /// Byte limit was reached.
    MaxBytes,
}

impl TruncationReason {
    #[must_use]
    const fn as_str(self) -> &'static str {
        match self {
            Self::MaxRecords => "max_records",
            Self::MaxBytes => "max_bytes",
        }
    }
}

/// Result summary from a limited line writer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineWriteSummary {
    /// Number of normal records written.
    pub records: usize,
    /// Number of normal record bytes written.
    pub bytes: usize,
    /// Truncation reason, if any.
    pub truncated: Option<TruncationReason>,
}

/// Writes minified JSONL records and enforces output limits.
#[derive(Debug)]
pub struct JsonlWriter<'a, W: Write + ?Sized> {
    inner: &'a mut W,
    limits: OutputLimits,
    records: usize,
    bytes: usize,
    truncated: Option<TruncationReason>,
}

impl<'a, W: Write + ?Sized> JsonlWriter<'a, W> {
    /// Create a JSONL writer.
    #[must_use]
    pub fn new(inner: &'a mut W, limits: OutputLimits) -> Self {
        Self {
            inner,
            limits,
            records: 0,
            bytes: 0,
            truncated: None,
        }
    }

    /// Write one minified JSON record followed by a newline.
    ///
    /// The first record is written even when limits are already exceeded so
    /// JSONL output can preserve the required summary-first contract.
    pub fn write_record<T: Serialize>(&mut self, record: &T) -> Result<bool> {
        if self.truncated.is_some() {
            return Ok(false);
        }

        let mut line = serde_json::to_vec(record)?;
        line.push(b'\n');

        let is_first_record = self.records == 0;
        if !is_first_record && self.records >= self.limits.max_records {
            self.truncated = Some(TruncationReason::MaxRecords);
            return Ok(false);
        }

        if !is_first_record && self.bytes + line.len() > self.limits.max_bytes {
            self.truncated = Some(TruncationReason::MaxBytes);
            return Ok(false);
        }

        self.inner.write_all(&line)?;
        self.records += 1;
        self.bytes += line.len();

        if is_first_record && self.limits.max_records == 0 {
            self.truncated = Some(TruncationReason::MaxRecords);
        } else if is_first_record && self.bytes > self.limits.max_bytes {
            self.truncated = Some(TruncationReason::MaxBytes);
        }

        Ok(true)
    }

    /// Finish the stream and emit a truncation warning when required.
    pub fn finish(self, warn_schema: &str) -> Result<LineWriteSummary> {
        let summary = LineWriteSummary {
            records: self.records,
            bytes: self.bytes,
            truncated: self.truncated,
        };

        if let Some(reason) = self.truncated {
            write_truncation_warning(self.inner, warn_schema, reason)?;
            if self.limits.strict {
                return Err(OutputError::TruncatedStrict);
            }
        }

        Ok(summary)
    }
}

#[derive(Debug, Serialize)]
struct TruncationWarning<'a> {
    schema: &'a str,
    #[serde(rename = "type")]
    kind: &'static str,
    code: &'static str,
    reason: &'static str,
    truncated: bool,
}

fn write_truncation_warning<W: Write + ?Sized>(
    w: &mut W,
    warn_schema: &str,
    reason: TruncationReason,
) -> Result<()> {
    let warning = TruncationWarning {
        schema: warn_schema,
        kind: "warn",
        code: "truncated",
        reason: reason.as_str(),
        truncated: true,
    };
    serde_json::to_writer(&mut *w, &warning)?;
    writeln!(w)?;
    Ok(())
}

/// Value for one ACF `key=value` field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentValue<'a> {
    /// String value.
    Str(&'a str),
    /// Boolean value.
    Bool(bool),
    /// Unsigned integer value.
    Usize(usize),
    /// Unsigned 64-bit integer value.
    U64(u64),
    /// Signed integer value.
    I64(i64),
}

/// One ACF `key=value` field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AgentField<'a> {
    key: &'a str,
    value: AgentValue<'a>,
}

impl<'a> AgentField<'a> {
    /// Create a string field.
    #[must_use]
    pub const fn str(key: &'a str, value: &'a str) -> Self {
        Self {
            key,
            value: AgentValue::Str(value),
        }
    }

    /// Create a boolean field.
    #[must_use]
    pub const fn bool(key: &'a str, value: bool) -> Self {
        Self {
            key,
            value: AgentValue::Bool(value),
        }
    }

    /// Create a `usize` field.
    #[must_use]
    pub const fn usize(key: &'a str, value: usize) -> Self {
        Self {
            key,
            value: AgentValue::Usize(value),
        }
    }

    /// Create a `u64` field.
    #[must_use]
    pub const fn u64(key: &'a str, value: u64) -> Self {
        Self {
            key,
            value: AgentValue::U64(value),
        }
    }

    /// Create an `i64` field.
    #[must_use]
    pub const fn i64(key: &'a str, value: i64) -> Self {
        Self {
            key,
            value: AgentValue::I64(value),
        }
    }
}

/// Format ACF fields into one line.
pub fn format_agent_fields(fields: &[AgentField<'_>]) -> Result<String> {
    let mut line = String::new();
    for (index, field) in fields.iter().enumerate() {
        if index > 0 {
            line.push(' ');
        }
        line.push_str(field.key);
        line.push('=');
        write_agent_value(&mut line, field.value)?;
    }
    Ok(line)
}

fn write_agent_value(line: &mut String, value: AgentValue<'_>) -> Result<()> {
    match value {
        AgentValue::Str(value) => {
            if is_raw_agent_value(value) {
                line.push_str(value);
            } else {
                line.push_str(&serde_json::to_string(value)?);
            }
        }
        AgentValue::Bool(value) => line.push_str(if value { "true" } else { "false" }),
        AgentValue::Usize(value) => {
            write!(line, "{value}").map_err(|err| io::Error::other(err.to_string()))?;
        }
        AgentValue::U64(value) => {
            write!(line, "{value}").map_err(|err| io::Error::other(err.to_string()))?;
        }
        AgentValue::I64(value) => {
            write!(line, "{value}").map_err(|err| io::Error::other(err.to_string()))?;
        }
    }
    Ok(())
}

fn is_raw_agent_value(value: &str) -> bool {
    !value.is_empty()
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric()
                || matches!(byte, b'.' | b'_' | b'-' | b'/' | b':' | b'+' | b'@')
        })
}

/// Writes ACF lines and enforces output limits.
#[derive(Debug)]
pub struct AgentCompactWriter<'a, W: Write + ?Sized> {
    inner: &'a mut W,
    limits: OutputLimits,
    records: usize,
    bytes: usize,
    truncated: Option<TruncationReason>,
}

impl<'a, W: Write + ?Sized> AgentCompactWriter<'a, W> {
    /// Create an ACF writer.
    #[must_use]
    pub fn new(inner: &'a mut W, limits: OutputLimits) -> Self {
        Self {
            inner,
            limits,
            records: 0,
            bytes: 0,
            truncated: None,
        }
    }

    /// Write one already-formatted ACF line.
    ///
    /// The first line is written even when limits are already exceeded so ACF
    /// output can preserve the required summary/schema-first contract.
    pub fn write_line(&mut self, line: &str) -> Result<bool> {
        if self.truncated.is_some() {
            return Ok(false);
        }

        let line_len = line.len() + 1;
        let is_first_record = self.records == 0;
        if !is_first_record && self.records >= self.limits.max_records {
            self.truncated = Some(TruncationReason::MaxRecords);
            return Ok(false);
        }

        if !is_first_record && self.bytes + line_len > self.limits.max_bytes {
            self.truncated = Some(TruncationReason::MaxBytes);
            return Ok(false);
        }

        self.inner.write_all(line.as_bytes())?;
        writeln!(self.inner)?;
        self.records += 1;
        self.bytes += line_len;

        if is_first_record && self.limits.max_records == 0 {
            self.truncated = Some(TruncationReason::MaxRecords);
        } else if is_first_record && self.bytes > self.limits.max_bytes {
            self.truncated = Some(TruncationReason::MaxBytes);
        }

        Ok(true)
    }

    /// Format and write one ACF field line.
    pub fn write_fields(&mut self, fields: &[AgentField<'_>]) -> Result<bool> {
        let line = format_agent_fields(fields)?;
        self.write_line(&line)
    }

    /// Finish the stream and emit a truncation warning when required.
    pub fn finish(self) -> Result<LineWriteSummary> {
        let summary = LineWriteSummary {
            records: self.records,
            bytes: self.bytes,
            truncated: self.truncated,
        };

        if let Some(reason) = self.truncated {
            writeln!(
                self.inner,
                "W code=truncated reason={} truncated=true",
                reason.as_str()
            )?;
            if self.limits.strict {
                return Err(OutputError::TruncatedStrict);
            }
        }

        Ok(summary)
    }
}

/// Create an auto-coloring stdout stream.
#[must_use]
pub fn stdout_stream(choice: ColorChoice) -> anstream::AutoStream<io::Stdout> {
    anstream::AutoStream::new(io::stdout(), choice)
}

/// Create an auto-coloring stderr stream.
#[must_use]
pub fn stderr_stream(choice: ColorChoice) -> anstream::AutoStream<io::Stderr> {
    anstream::AutoStream::new(io::stderr(), choice)
}

/// Style for important human output.
#[must_use]
pub fn strong_style(choice: ColorChoice) -> Style {
    if choice == ColorChoice::Never {
        Style::new()
    } else {
        Style::new().bold()
    }
}

/// Style for muted human output.
#[must_use]
pub fn muted_style(choice: ColorChoice) -> Style {
    if choice == ColorChoice::Never {
        Style::new()
    } else {
        Style::new().fg_color(Some(AnsiColor::BrightBlack.into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ax_core::SystemClock;
    use serde_json::json;

    #[test]
    fn json_envelope_serializes_in_standard_shape() -> Result<()> {
        let envelope = JsonEnvelope::new("ax.peek.v1", json!({ "status": "stub" }), vec![], vec![]);
        let value = serde_json::to_value(envelope)?;
        assert_eq!(
            value,
            json!({
                "schema": "ax.peek.v1",
                "ok": true,
                "data": { "status": "stub" },
                "warnings": [],
                "errors": []
            })
        );
        Ok(())
    }

    #[test]
    fn jsonl_writer_minifies_records() -> Result<()> {
        let mut output = Vec::new();
        let mut writer = JsonlWriter::new(&mut output, OutputLimits::default());
        assert!(writer.write_record(&json!({ "schema": "test.v1", "type": "summary" }))?);
        let summary = writer.finish("test.warn.v1")?;
        assert_eq!(summary.truncated, None);
        assert_eq!(
            String::from_utf8_lossy(&output),
            "{\"schema\":\"test.v1\",\"type\":\"summary\"}\n"
        );
        Ok(())
    }

    #[test]
    fn jsonl_writer_truncates_by_record_limit() -> Result<()> {
        let mut output = Vec::new();
        let limits = OutputLimits {
            max_records: 1,
            max_bytes: 1_000,
            strict: false,
        };
        let mut writer = JsonlWriter::new(&mut output, limits);
        assert!(writer.write_record(&json!({ "schema": "test.v1", "type": "summary" }))?);
        assert!(!writer.write_record(&json!({ "schema": "test.v1", "type": "detail" }))?);
        let summary = writer.finish("test.warn.v1")?;
        assert_eq!(summary.truncated, Some(TruncationReason::MaxRecords));
        assert_eq!(
            String::from_utf8_lossy(&output),
            "{\"schema\":\"test.v1\",\"type\":\"summary\"}\n{\"schema\":\"test.warn.v1\",\"type\":\"warn\",\"code\":\"truncated\",\"reason\":\"max_records\",\"truncated\":true}\n"
        );
        Ok(())
    }

    #[test]
    fn jsonl_writer_truncates_by_byte_limit() -> Result<()> {
        let mut output = Vec::new();
        let limits = OutputLimits {
            max_records: 10,
            max_bytes: 3,
            strict: false,
        };
        let mut writer = JsonlWriter::new(&mut output, limits);
        assert!(writer.write_record(&json!({ "schema": "test.v1", "type": "summary" }))?);
        let summary = writer.finish("test.warn.v1")?;
        assert_eq!(summary.truncated, Some(TruncationReason::MaxBytes));
        assert_eq!(
            String::from_utf8_lossy(&output),
            "{\"schema\":\"test.v1\",\"type\":\"summary\"}\n{\"schema\":\"test.warn.v1\",\"type\":\"warn\",\"code\":\"truncated\",\"reason\":\"max_bytes\",\"truncated\":true}\n"
        );
        Ok(())
    }

    #[test]
    fn jsonl_writer_strict_truncation_errors_after_warning() -> Result<()> {
        let mut output = Vec::new();
        let limits = OutputLimits {
            max_records: 0,
            max_bytes: 1_000,
            strict: true,
        };
        let mut writer = JsonlWriter::new(&mut output, limits);
        assert!(writer.write_record(&json!({ "schema": "test.v1", "type": "summary" }))?);
        let result = writer.finish("test.warn.v1");
        assert!(matches!(result, Err(OutputError::TruncatedStrict)));
        assert_eq!(
            String::from_utf8_lossy(&output),
            "{\"schema\":\"test.v1\",\"type\":\"summary\"}\n{\"schema\":\"test.warn.v1\",\"type\":\"warn\",\"code\":\"truncated\",\"reason\":\"max_records\",\"truncated\":true}\n"
        );
        Ok(())
    }

    #[test]
    fn acf_fields_quote_only_when_needed() -> Result<()> {
        let line = format_agent_fields(&[
            AgentField::str("schema", "ax.run.agent.v1"),
            AgentField::bool("ok", false),
            AgentField::str("cmd", "npm test"),
            AgentField::usize("ms", 42),
            AgentField::i64("exit", -1),
        ])?;
        assert_eq!(
            line,
            "schema=ax.run.agent.v1 ok=false cmd=\"npm test\" ms=42 exit=-1"
        );
        Ok(())
    }

    #[test]
    fn agent_compact_writer_truncates_by_record_limit() -> Result<()> {
        let mut output = Vec::new();
        let limits = OutputLimits {
            max_records: 1,
            max_bytes: 1_000,
            strict: false,
        };
        let mut writer = AgentCompactWriter::new(&mut output, limits);
        assert!(writer.write_line("schema=test.agent.v1 ok=true mode=records truncated=false")?);
        assert!(!writer.write_line("F path=src/main.rs bytes=12")?);
        let summary = writer.finish()?;
        assert_eq!(summary.truncated, Some(TruncationReason::MaxRecords));
        assert_eq!(
            String::from_utf8_lossy(&output),
            "schema=test.agent.v1 ok=true mode=records truncated=false\nW code=truncated reason=max_records truncated=true\n"
        );
        Ok(())
    }

    #[test]
    fn render_context_holds_shared_settings() {
        let clock = SystemClock;
        let limits = OutputLimits::default();
        let ctx = RenderContext::new(OutputMode::Json, limits, ColorChoice::Never, &clock);
        assert_eq!(ctx.mode, OutputMode::Json);
        assert_eq!(ctx.limits, limits);
        assert_eq!(ctx.color, ColorChoice::Never);
    }
}
