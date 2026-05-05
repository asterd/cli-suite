use std::io::Write;

use axt_core::ErrorCode;
use axt_output::{
    JsonEnvelope, AgentJsonlWriter, OutputDiagnostic, RenderContext, Renderable,
    Result as RenderResult,
};
use serde::Serialize;
use serde_json::json;

use crate::model::{OutlineData, OutlineSummary, OutlineWarning, WarningCode};

#[derive(Debug, Serialize)]
struct JsonlSummary<'a> {
    schema: &'static str,
    #[serde(rename = "type")]
    kind: &'static str,
    ok: bool,
    root: &'a str,
    files: usize,
    symbols: usize,
    warnings: usize,
    source_bytes: usize,
    signature_bytes: usize,
    truncated: bool,
    next: &'a [String],
}

impl Renderable for OutlineData {
    fn render_human(&self, w: &mut dyn Write, _ctx: &RenderContext<'_>) -> RenderResult<()> {
        writeln!(
            w,
            "root={} files={} symbols={} warnings={} source_bytes={} signature_bytes={} truncated={}",
            self.root,
            self.summary.files,
            self.summary.symbols,
            self.summary.warnings,
            self.summary.source_bytes,
            self.summary.signature_bytes,
            self.summary.truncated
        )?;
        for symbol in &self.symbols {
            writeln!(
                w,
                "{}:{}-{} {} {} {}",
                symbol.path,
                symbol.range.start_line,
                symbol.range.end_line,
                symbol.kind.as_str(),
                symbol.visibility.as_str(),
                symbol.name
            )?;
            writeln!(w, "  {}", symbol.signature)?;
            if let Some(docs) = &symbol.docs {
                writeln!(w, "  docs: {docs}")?;
            }
        }
        for warning in &self.warnings {
            writeln!(
                w,
                "warning {} {}",
                warning.code.as_str(),
                warning.path.as_ref().map_or("-", |path| path.as_str())
            )?;
        }
        Ok(())
    }

    fn render_compact(&self, w: &mut dyn Write, _ctx: &RenderContext<'_>) -> RenderResult<()> {
        writeln!(
            w,
            "outline root={} files={} symbols={} warnings={} source_bytes={} signature_bytes={} truncated={}",
            self.root,
            self.summary.files,
            self.summary.symbols,
            self.summary.warnings,
            self.summary.source_bytes,
            self.summary.signature_bytes,
            self.summary.truncated
        )?;
        for symbol in &self.symbols {
            writeln!(
                w,
                "symbol {}:{}-{} kind={} vis={} name={} sig={}",
                symbol.path,
                symbol.range.start_line,
                symbol.range.end_line,
                symbol.kind.as_str(),
                symbol.visibility.as_str(),
                symbol.name,
                symbol.signature
            )?;
        }
        for warning in &self.warnings {
            writeln!(
                w,
                "warn code={} path={} message={}",
                warning.code.as_str(),
                warning.path.as_ref().map_or("-", |path| path.as_str()),
                warning.message
            )?;
        }
        Ok(())
    }

    fn render_json(&self, w: &mut dyn Write, _ctx: &RenderContext<'_>) -> RenderResult<()> {
        let envelope = JsonEnvelope::new(
            "axt.outline.v1",
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
        writer.write_record(&jsonl_summary(&self.summary, self.root.as_str(), &self.next))?;
        for symbol in &self.symbols {
            if self.symbols_only {
                writer.write_record(&json!({
                    "schema": "axt.outline.symbol.v1",
                    "type": "symbol",
                    "name": symbol.name,
                    "kind": symbol.kind,
                    "line": symbol.range.start_line
                }))?;
            } else {
                writer.write_record(&json!({
                    "schema": "axt.outline.symbol.v1",
                    "type": "symbol",
                    "p": symbol.path,
                    "l": symbol.language,
                    "k": symbol.kind,
                    "vis": symbol.visibility,
                    "n": symbol.name,
                    "sig": symbol.signature,
                    "docs": symbol.docs,
                    "range": symbol.range,
                    "parent": symbol.parent
                }))?;
            }
        }
        for warning in &self.warnings {
            writer.write_record(&json!({
                "schema": "axt.outline.warn.v1",
                "type": "warn",
                "code": warning.code,
                "path": warning.path,
                "message": warning.message
            }))?;
        }
        let _summary = writer.finish("axt.outline.warn.v1")?;
        Ok(())
    }
}

const fn jsonl_summary<'a>(
    summary: &OutlineSummary,
    root: &'a str,
    next: &'a [String],
) -> JsonlSummary<'a> {
    JsonlSummary {
        schema: "axt.outline.summary.v1",
        kind: "summary",
        ok: true,
        root,
        files: summary.files,
        symbols: summary.symbols,
        warnings: summary.warnings,
        source_bytes: summary.source_bytes,
        signature_bytes: summary.signature_bytes,
        truncated: summary.truncated,
        next,
    }
}

fn json_warnings(warnings: &[OutlineWarning]) -> Vec<OutputDiagnostic> {
    warnings
        .iter()
        .map(|warning| OutputDiagnostic {
            code: warning_error_code(warning.code),
            message: warning.message.clone(),
            context: json!({
                "code": warning.code.as_str(),
                "path": warning.path.as_deref(),
            }),
        })
        .collect()
}

const fn warning_error_code(code: WarningCode) -> ErrorCode {
    match code {
        WarningCode::ParseError => ErrorCode::RuntimeError,
        WarningCode::UnsupportedLanguage => ErrorCode::FeatureUnsupported,
    }
}
