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

use std::io::{self, Write};

use anstyle::{AnsiColor, Style};
use axt_core::{Clock, ColorChoice, ErrorCode, OutputLimits, OutputMode};
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
///
/// Three primary modes are required:
/// - `render_human` for terminals,
/// - `render_json` for the canonical envelope,
/// - `render_agent` for JSONL agent output.
///
/// `render_agent` is the schema-versioned, summary-first JSONL stream that
/// AI agents consume.
pub trait Renderable {
    /// Render human-readable output.
    fn render_human(&self, w: &mut dyn Write, ctx: &RenderContext<'_>) -> Result<()>;

    /// Render the canonical JSON envelope.
    fn render_json(&self, w: &mut dyn Write, ctx: &RenderContext<'_>) -> Result<()>;

    /// Render JSONL agent output (summary-first, schema-versioned).
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
pub struct AgentJsonlWriter<'a, W: Write + ?Sized> {
    inner: &'a mut W,
    limits: OutputLimits,
    records: usize,
    bytes: usize,
    truncated: Option<TruncationReason>,
}

impl<'a, W: Write + ?Sized> AgentJsonlWriter<'a, W> {
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

    /// Flush the underlying writer.
    pub fn flush(&mut self) -> Result<()> {
        self.inner.flush()?;
        Ok(())
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
    use axt_core::SystemClock;
    use serde_json::json;

    #[test]
    fn json_envelope_serializes_in_standard_shape() -> Result<()> {
        let envelope =
            JsonEnvelope::new("axt.peek.v1", json!({ "status": "ready" }), vec![], vec![]);
        let value = serde_json::to_value(envelope)?;
        assert_eq!(
            value,
            json!({
                "schema": "axt.peek.v1",
                "ok": true,
                "data": { "status": "ready" },
                "warnings": [],
                "errors": []
            })
        );
        Ok(())
    }

    #[test]
    fn jsonl_writer_minifies_records() -> Result<()> {
        let mut output = Vec::new();
        let mut writer = AgentJsonlWriter::new(&mut output, OutputLimits::default());
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
        let mut writer = AgentJsonlWriter::new(&mut output, limits);
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
        let mut writer = AgentJsonlWriter::new(&mut output, limits);
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
        let mut writer = AgentJsonlWriter::new(&mut output, limits);
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
    fn render_context_holds_shared_settings() {
        let clock = SystemClock;
        let limits = OutputLimits::default();
        let ctx = RenderContext::new(OutputMode::Json, limits, ColorChoice::Never, &clock);
        assert_eq!(ctx.mode, OutputMode::Json);
        assert_eq!(ctx.limits, limits);
        assert_eq!(ctx.color, ColorChoice::Never);
    }
}
