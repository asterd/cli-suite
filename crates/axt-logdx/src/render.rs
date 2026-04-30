use std::io::Write;

use axt_core::ErrorCode;
use axt_output::{
    AgentJsonlWriter, JsonEnvelope, OutputDiagnostic, RenderContext, Renderable,
    Result as RenderResult,
};
use serde::Serialize;
use serde_json::json;

use crate::model::{LogdxData, LogdxWarning, WarningCode};

#[derive(Debug, Serialize)]
struct JsonlSummary<'a> {
    schema: &'static str,
    #[serde(rename = "type")]
    kind: &'static str,
    ok: bool,
    sources: usize,
    lines: usize,
    groups: usize,
    errors: usize,
    warnings: usize,
    bytes_scanned: u64,
    truncated: bool,
    next: &'a [String],
}

impl Renderable for LogdxData {
    fn render_human(&self, w: &mut dyn Write, _ctx: &RenderContext<'_>) -> RenderResult<()> {
        writeln!(
            w,
            "sources={} lines={} groups={} errors={} warnings={} bytes_scanned={} truncated={}",
            self.sources.len(),
            self.summary.lines,
            self.summary.groups,
            self.summary.errors,
            self.summary.warnings,
            self.summary.bytes_scanned,
            self.summary.truncated
        )?;
        for group in &self.groups {
            writeln!(
                w,
                "{} count={} severity={} first={}:{} last={}:{}",
                group.fingerprint,
                group.count,
                group.severity.as_str(),
                group.first.source,
                group.first.line,
                group.last.source,
                group.last.line
            )?;
            writeln!(w, "  message: {}", group.message)?;
            for line in &group.stack {
                writeln!(w, "  stack: {line}")?;
            }
            for snippet in &group.snippets {
                writeln!(w, "  sample: {snippet}")?;
            }
        }
        for bucket in &self.timeline {
            writeln!(
                w,
                "timeline {} trace={} debug={} info={} warn={} error={} fatal={}",
                bucket.bucket,
                bucket.trace,
                bucket.debug,
                bucket.info,
                bucket.warn,
                bucket.error,
                bucket.fatal
            )?;
        }
        for warning in &self.warnings {
            writeln!(
                w,
                "warning {} {}",
                warning.code.as_str(),
                warning.path.as_deref().unwrap_or("-")
            )?;
        }
        Ok(())
    }

    fn render_json(&self, w: &mut dyn Write, _ctx: &RenderContext<'_>) -> RenderResult<()> {
        let envelope = JsonEnvelope::new(
            "axt.logdx.v1",
            self,
            json_warnings(&self.warnings),
            Vec::new(),
        );
        serde_json::to_writer(&mut *w, &envelope)?;
        writeln!(w)?;
        Ok(())
    }

    fn render_agent(&self, w: &mut dyn Write, ctx: &RenderContext<'_>) -> RenderResult<()> {
        let mut writer = AgentJsonlWriter::new(w, ctx.limits);
        writer.write_record(&jsonl_summary(self))?;
        for group in &self.groups {
            writer.write_record(&json!({
                "schema": "axt.logdx.group.v1",
                "type": "group",
                "fp": group.fingerprint,
                "sev": group.severity,
                "count": group.count,
                "first": {"p": group.first.source, "line": group.first.line, "ts": group.first.timestamp},
                "last": {"p": group.last.source, "line": group.last.line, "ts": group.last.timestamp},
                "msg": group.message,
                "stack": group.stack,
                "snip": group.snippets
            }))?;
        }
        for bucket in &self.timeline {
            writer.write_record(&json!({
                "schema": "axt.logdx.timeline.v1",
                "type": "timeline",
                "bucket": bucket.bucket,
                "trace": bucket.trace,
                "debug": bucket.debug,
                "info": bucket.info,
                "warn": bucket.warn,
                "error": bucket.error,
                "fatal": bucket.fatal
            }))?;
        }
        for warning in &self.warnings {
            writer.write_record(&json!({
                "schema": "axt.logdx.warn.v1",
                "type": "warn",
                "code": warning.code,
                "path": warning.path,
                "message": warning.message
            }))?;
        }
        let _summary = writer.finish("axt.logdx.warn.v1")?;
        Ok(())
    }
}

fn jsonl_summary(data: &LogdxData) -> JsonlSummary<'_> {
    JsonlSummary {
        schema: "axt.logdx.summary.v1",
        kind: "summary",
        ok: true,
        sources: data.sources.len(),
        lines: data.summary.lines,
        groups: data.summary.groups,
        errors: data.summary.errors,
        warnings: data.summary.warnings,
        bytes_scanned: data.summary.bytes_scanned,
        truncated: data.summary.truncated,
        next: &data.next,
    }
}

fn json_warnings(warnings: &[LogdxWarning]) -> Vec<OutputDiagnostic> {
    warnings
        .iter()
        .map(|warning| OutputDiagnostic {
            code: warning_error_code(warning.code),
            message: warning.message.clone(),
            context: json!({
                "code": warning.code.as_str(),
                "path": warning.path,
            }),
        })
        .collect()
}

const fn warning_error_code(code: WarningCode) -> ErrorCode {
    match code {
        WarningCode::TimeUnparseable | WarningCode::InputTruncated | WarningCode::InvalidUtf8 => {
            ErrorCode::IoError
        }
    }
}
