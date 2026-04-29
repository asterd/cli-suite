use std::io::Write;

use axt_core::ErrorCode;
use axt_output::{
    AgentJsonlWriter, JsonEnvelope, OutputDiagnostic, RenderContext, Renderable,
    Result as RenderResult,
};
use serde::Serialize;
use serde_json::json;

use crate::model::{SliceData, SliceStatus, SliceSummary, SliceWarning, WarningCode};

#[derive(Debug, Serialize)]
struct JsonlSummary<'a> {
    schema: &'static str,
    #[serde(rename = "type")]
    kind: &'static str,
    ok: bool,
    p: &'a str,
    l: &'a crate::model::Language,
    status: SliceStatus,
    matches: usize,
    candidates: usize,
    source_bytes: usize,
    truncated: bool,
    next: &'a [String],
}

impl Renderable for SliceData {
    fn render_human(&self, w: &mut dyn Write, _ctx: &RenderContext<'_>) -> RenderResult<()> {
        writeln!(
            w,
            "path={} language={:?} status={:?} matches={} candidates={} source_bytes={} truncated={}",
            self.path,
            self.language,
            self.status,
            self.summary.matches,
            self.summary.candidates,
            self.summary.source_bytes,
            self.summary.truncated
        )?;
        if let (Some(symbol), Some(range), Some(source)) = (&self.symbol, &self.range, &self.source)
        {
            writeln!(
                w,
                "{}:{}-{} {} {} {}",
                self.path,
                range.start_line,
                range.end_line,
                symbol.kind.as_str(),
                symbol.visibility.as_str(),
                symbol.qualified_name
            )?;
            write!(w, "{source}")?;
            if !source.ends_with('\n') {
                writeln!(w)?;
            }
        }
        for candidate in &self.candidates {
            writeln!(
                w,
                "candidate {}:{}-{} {} {} {}",
                self.path,
                candidate.range.start_line,
                candidate.range.end_line,
                candidate.kind.as_str(),
                candidate.visibility.as_str(),
                candidate.qualified_name
            )?;
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
            "axt.slice.v1",
            self,
            json_warnings(&self.warnings),
            Vec::new(),
        );
        serde_json::to_writer(&mut *w, &envelope)?;
        writeln!(w)?;
        Ok(())
    }

    fn render_agent(&self, w: &mut dyn Write, ctx: &RenderContext<'_>) -> RenderResult<()> {
        let detail_records = agent_detail_records(self);
        let mut summary = self.summary.clone();
        summary.truncated = agent_would_truncate(self, &summary, &detail_records, ctx)?;
        let mut writer = AgentJsonlWriter::new(w, ctx.limits);
        writer.write_record(&jsonl_summary(self, &summary))?;
        for record in &detail_records {
            writer.write_record(record)?;
        }
        let _summary = writer.finish("axt.slice.warn.v1")?;
        Ok(())
    }
}

fn agent_detail_records(data: &SliceData) -> Vec<serde_json::Value> {
    let mut records = Vec::new();
    if let (Some(symbol), Some(range), Some(source)) = (&data.symbol, &data.range, &data.source) {
        records.push(json!({
            "schema": "axt.slice.source.v1",
            "type": "source",
            "p": data.path,
            "l": data.language,
            "k": symbol.kind,
            "n": symbol.name,
            "qn": symbol.qualified_name,
            "range": range,
            "spans": data.spans,
            "symbol_range": symbol.range,
            "source": source
        }));
    }
    for candidate in &data.candidates {
        records.push(json!({
            "schema": "axt.slice.candidate.v1",
            "type": "candidate",
            "p": data.path,
            "l": data.language,
            "k": candidate.kind,
            "n": candidate.name,
            "qn": candidate.qualified_name,
            "range": candidate.range,
            "parent": candidate.parent
        }));
    }
    for warning in &data.warnings {
        records.push(json!({
            "schema": "axt.slice.warn.v1",
            "type": "warn",
            "code": warning.code,
            "path": warning.path,
            "message": warning.message
        }));
    }
    records
}

fn agent_would_truncate(
    data: &SliceData,
    summary: &SliceSummary,
    detail_records: &[serde_json::Value],
    ctx: &RenderContext<'_>,
) -> RenderResult<bool> {
    let mut bytes = json_record_len(&jsonl_summary(data, summary))?;
    if ctx.limits.max_records == 0 || bytes > ctx.limits.max_bytes {
        return Ok(true);
    }
    for (index, record) in detail_records.iter().enumerate() {
        let records = index + 1;
        let len = json_record_len(record)?;
        if records >= ctx.limits.max_records || bytes + len > ctx.limits.max_bytes {
            return Ok(true);
        }
        bytes += len;
    }
    Ok(false)
}

fn json_record_len(record: &impl Serialize) -> RenderResult<usize> {
    Ok(serde_json::to_vec(record)?.len() + 1)
}

fn jsonl_summary<'a>(data: &'a SliceData, summary: &SliceSummary) -> JsonlSummary<'a> {
    JsonlSummary {
        schema: "axt.slice.summary.v1",
        kind: "summary",
        ok: true,
        p: data.path.as_str(),
        l: &data.language,
        status: data.status,
        matches: summary.matches,
        candidates: summary.candidates,
        source_bytes: summary.source_bytes,
        truncated: summary.truncated,
        next: &data.next,
    }
}

fn json_warnings(warnings: &[SliceWarning]) -> Vec<OutputDiagnostic> {
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
        WarningCode::Truncated => ErrorCode::OutputTruncatedStrict,
    }
}
