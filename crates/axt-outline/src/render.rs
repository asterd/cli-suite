use std::io::Write;

use axt_core::ErrorCode;
use axt_output::{
    format_agent_fields, AgentCompactWriter, AgentField, JsonEnvelope, JsonlWriter,
    OutputDiagnostic, RenderContext, Renderable, Result as RenderResult,
};
use serde::Serialize;
use serde_json::json;

use crate::model::{OutlineData, OutlineSummary, OutlineWarning, Symbol, WarningCode};

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
}

impl OutlineData {
    pub fn render_json_data(&self, w: &mut dyn Write) -> RenderResult<()> {
        serde_json::to_writer(&mut *w, self)?;
        writeln!(w)?;
        Ok(())
    }
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

    fn render_jsonl(&self, w: &mut dyn Write, ctx: &RenderContext<'_>) -> RenderResult<()> {
        let mut writer = JsonlWriter::new(w, ctx.limits);
        writer.write_record(&jsonl_summary(&self.summary, self.root.as_str()))?;
        for symbol in &self.symbols {
            writer.write_record(&json!({
                "schema": "axt.outline.symbol.v1",
                "type": "symbol",
                "path": symbol.path,
                "language": symbol.language,
                "kind": symbol.kind,
                "visibility": symbol.visibility,
                "name": symbol.name,
                "signature": symbol.signature,
                "docs": symbol.docs,
                "range": symbol.range,
                "parent": symbol.parent
            }))?;
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

    fn render_agent(&self, w: &mut dyn Write, ctx: &RenderContext<'_>) -> RenderResult<()> {
        let mut writer = AgentCompactWriter::new(w, ctx.limits);
        writer.write_line(&agent_summary(&self.summary)?)?;
        for symbol in &self.symbols {
            writer.write_line(&agent_symbol(symbol)?)?;
        }
        for warning in &self.warnings {
            writer.write_line(&agent_warning(warning)?)?;
        }
        for next in &self.next {
            writer.write_line(&prefixed_line("S", &[AgentField::str("run", next)])?)?;
        }
        let _summary = writer.finish()?;
        Ok(())
    }
}

const fn jsonl_summary<'a>(summary: &OutlineSummary, root: &'a str) -> JsonlSummary<'a> {
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

fn agent_summary(summary: &OutlineSummary) -> RenderResult<String> {
    format_agent_fields(&[
        AgentField::str("schema", "axt.outline.agent.v1"),
        AgentField::bool("ok", true),
        AgentField::str("mode", "records"),
        AgentField::usize("files", summary.files),
        AgentField::usize("symbols", summary.symbols),
        AgentField::usize("warnings", summary.warnings),
        AgentField::usize("source_bytes", summary.source_bytes),
        AgentField::usize("signature_bytes", summary.signature_bytes),
        AgentField::bool("truncated", summary.truncated),
    ])
}

fn agent_symbol(symbol: &Symbol) -> RenderResult<String> {
    prefixed_line("Y", &[
        AgentField::str("path", symbol.path.as_str()),
        AgentField::str("lang", symbol.language.as_str()),
        AgentField::str("kind", symbol.kind.as_str()),
        AgentField::str("visibility", symbol.visibility.as_str()),
        AgentField::str("name", &symbol.name),
        AgentField::usize("line", symbol.range.start_line),
        AgentField::usize("end_line", symbol.range.end_line),
        AgentField::str("parent", symbol.parent.as_deref().unwrap_or("-")),
        AgentField::str("signature", &symbol.signature),
        AgentField::str("docs", symbol.docs.as_deref().unwrap_or("-")),
    ])
}

fn agent_warning(warning: &OutlineWarning) -> RenderResult<String> {
    prefixed_line("W", &[
        AgentField::str("code", warning.code.as_str()),
        AgentField::str("path", warning.path.as_ref().map_or("-", |path| path.as_str())),
        AgentField::str("message", &warning.message),
    ])
}

fn prefixed_line(prefix: &str, fields: &[AgentField<'_>]) -> RenderResult<String> {
    let rest = format_agent_fields(fields)?;
    if rest.is_empty() {
        Ok(prefix.to_owned())
    } else {
        Ok(format!("{prefix} {rest}"))
    }
}
