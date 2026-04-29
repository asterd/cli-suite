use std::io::Write;

use axt_core::ErrorCode;
use axt_output::{
    format_agent_fields, AgentCompactWriter, AgentField, JsonEnvelope, JsonlWriter,
    OutputDiagnostic, RenderContext, Renderable, Result as RenderResult,
};
use serde::Serialize;
use serde_json::json;

use crate::model::{CtxpackData, CtxpackSummary, CtxpackWarning, SearchHit, WarningCode};

#[derive(Debug, Serialize)]
struct JsonlSummary<'a> {
    schema: &'static str,
    #[serde(rename = "type")]
    kind: &'static str,
    ok: bool,
    root: &'a str,
    patterns: usize,
    files_scanned: usize,
    files_matched: usize,
    hits: usize,
    warnings: usize,
    bytes_scanned: u64,
    truncated: bool,
}

impl CtxpackData {
    pub fn render_json_data(&self, w: &mut dyn Write) -> RenderResult<()> {
        serde_json::to_writer(&mut *w, self)?;
        writeln!(w)?;
        Ok(())
    }
}

impl Renderable for CtxpackData {
    fn render_human(&self, w: &mut dyn Write, _ctx: &RenderContext<'_>) -> RenderResult<()> {
        writeln!(
            w,
            "root={} patterns={} files={} hits={} warnings={} bytes_scanned={} truncated={}",
            self.root,
            self.patterns.len(),
            self.summary.files_scanned,
            self.summary.hits,
            self.summary.warnings,
            self.summary.bytes_scanned,
            self.summary.truncated
        )?;
        for hit in &self.hits {
            writeln!(
                w,
                "{}:{}:{} {} {} {:?}",
                hit.path,
                hit.line,
                hit.column,
                hit.pattern,
                hit.kind.as_str(),
                hit.matched_text
            )?;
            if let Some(node_kind) = &hit.node_kind {
                writeln!(
                    w,
                    "  ast source={} lang={} node={} symbol={}",
                    hit.classification_source.as_str(),
                    hit.language.as_deref().unwrap_or("-"),
                    node_kind,
                    hit.enclosing_symbol.as_deref().unwrap_or("-")
                )?;
            }
            for line in hit.snippet.lines() {
                writeln!(w, "  {line}")?;
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
            "axt.ctxpack.v1",
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
        writer.write_record(&jsonl_summary(self))?;
        for hit in &self.hits {
            writer.write_record(&json!({
                "schema": "axt.ctxpack.hit.v1",
                "type": "hit",
                "pattern": hit.pattern,
                "path": hit.path,
                "line": hit.line,
                "column": hit.column,
                "byte_range": hit.byte_range,
                "kind": hit.kind,
                "classification_source": hit.classification_source,
                "language": hit.language,
                "node_kind": hit.node_kind,
                "enclosing_symbol": hit.enclosing_symbol,
                "ast_path": hit.ast_path,
                "matched_text": hit.matched_text,
                "snippet": hit.snippet
            }))?;
        }
        for warning in &self.warnings {
            writer.write_record(&json!({
                "schema": "axt.ctxpack.warn.v1",
                "type": "warn",
                "code": warning.code,
                "path": warning.path,
                "message": warning.message
            }))?;
        }
        let _summary = writer.finish("axt.ctxpack.warn.v1")?;
        Ok(())
    }

    fn render_agent(&self, w: &mut dyn Write, ctx: &RenderContext<'_>) -> RenderResult<()> {
        let mut writer = AgentCompactWriter::new(w, ctx.limits);
        writer.write_line(&agent_summary(self)?)?;
        for hit in &self.hits {
            writer.write_line(&agent_hit(hit)?)?;
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

fn jsonl_summary(data: &CtxpackData) -> JsonlSummary<'_> {
    JsonlSummary {
        schema: "axt.ctxpack.summary.v1",
        kind: "summary",
        ok: true,
        root: data.root.as_str(),
        patterns: data.patterns.len(),
        files_scanned: data.summary.files_scanned,
        files_matched: data.summary.files_matched,
        hits: data.summary.hits,
        warnings: data.summary.warnings,
        bytes_scanned: data.summary.bytes_scanned,
        truncated: data.summary.truncated,
    }
}

fn json_warnings(warnings: &[CtxpackWarning]) -> Vec<OutputDiagnostic> {
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
        WarningCode::PermissionDenied => ErrorCode::PermissionDenied,
        WarningCode::BinarySkipped
        | WarningCode::NonUtf8Skipped
        | WarningCode::PathNotUtf8
        | WarningCode::Walk => ErrorCode::IoError,
    }
}

fn agent_summary(data: &CtxpackData) -> RenderResult<String> {
    let summary: &CtxpackSummary = &data.summary;
    format_agent_fields(&[
        AgentField::str("schema", "axt.ctxpack.agent.v1"),
        AgentField::bool("ok", true),
        AgentField::str("mode", "records"),
        AgentField::usize("patterns", data.patterns.len()),
        AgentField::usize("files", summary.files_scanned),
        AgentField::usize("matched", summary.files_matched),
        AgentField::usize("hits", summary.hits),
        AgentField::usize("warnings", summary.warnings),
        AgentField::u64("bytes", summary.bytes_scanned),
        AgentField::bool("truncated", summary.truncated),
    ])
}

fn agent_hit(hit: &SearchHit) -> RenderResult<String> {
    prefixed_line("H", &[
        AgentField::str("pattern", &hit.pattern),
        AgentField::str("path", hit.path.as_str()),
        AgentField::usize("line", hit.line),
        AgentField::usize("col", hit.column),
        AgentField::usize("start", hit.byte_range.start),
        AgentField::usize("end", hit.byte_range.end),
        AgentField::str("kind", hit.kind.as_str()),
        AgentField::str("src", hit.classification_source.as_str()),
        AgentField::str("lang", hit.language.as_deref().unwrap_or("-")),
        AgentField::str("node", hit.node_kind.as_deref().unwrap_or("-")),
        AgentField::str("symbol", hit.enclosing_symbol.as_deref().unwrap_or("-")),
        AgentField::str("text", &hit.matched_text),
        AgentField::str("snippet", &hit.snippet),
    ])
}

fn agent_warning(warning: &CtxpackWarning) -> RenderResult<String> {
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
