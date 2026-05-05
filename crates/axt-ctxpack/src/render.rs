use std::io::Write;

use axt_core::ErrorCode;
use axt_output::{
    JsonEnvelope, AgentJsonlWriter, OutputDiagnostic, RenderContext, Renderable,
    Result as RenderResult,
};
use serde::Serialize;
use serde_json::json;

use crate::model::{CtxpackData, CtxpackWarning, WarningCode};

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
    next: &'a [String],
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

    fn render_compact(&self, w: &mut dyn Write, _ctx: &RenderContext<'_>) -> RenderResult<()> {
        writeln!(
            w,
            "ctxpack root={} patterns={} files_scanned={} files_matched={} hits={} warnings={} bytes_scanned={} truncated={}",
            self.root,
            self.patterns.len(),
            self.summary.files_scanned,
            self.summary.files_matched,
            self.summary.hits,
            self.summary.warnings,
            self.summary.bytes_scanned,
            self.summary.truncated
        )?;
        for hit in &self.hits {
            writeln!(
                w,
                "hit pat={} {}:{}:{} kind={} text={:?}",
                hit.pattern,
                hit.path,
                hit.line,
                hit.column,
                hit.kind.as_str(),
                hit.matched_text
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
            "axt.ctxpack.v1",
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
        for hit in &self.hits {
            writer.write_record(&json!({
                "schema": "axt.ctxpack.hit.v1",
                "type": "hit",
                "pat": hit.pattern,
                "p": hit.path,
                "line": hit.line,
                "col": hit.column,
                "range": hit.byte_range,
                "k": hit.kind,
                "src": hit.classification_source,
                "l": hit.language,
                "node": hit.node_kind,
                "sym": hit.enclosing_symbol,
                "text": hit.matched_text,
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
        next: &data.next,
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
