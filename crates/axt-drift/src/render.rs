use std::io::Write;

use axt_core::{ErrorCode, OutputLimits};
use axt_output::{
    AgentJsonlWriter, JsonEnvelope, OutputDiagnostic, RenderContext, Renderable,
    Result as RenderResult,
};
use serde::Serialize;
use serde_json::{json, Value};

use crate::{model::DriftData, output::DriftOutput};

#[derive(Debug, Serialize)]
struct JsonlSummary<'a> {
    schema: &'static str,
    #[serde(rename = "type")]
    kind: &'static str,
    ok: bool,
    operation: &'static str,
    name: Option<&'a str>,
    files: usize,
    hash_skipped_size: usize,
    changes: usize,
    marks: usize,
    removed: usize,
    exit: Option<i32>,
    truncated: bool,
    next: Vec<String>,
}

impl Renderable for DriftOutput {
    fn render_human(&self, w: &mut dyn Write, _ctx: &RenderContext<'_>) -> RenderResult<()> {
        let data = self.data();
        match data.operation.as_str() {
            "mark" => {
                writeln!(
                    w,
                    "marked {} file(s) at {}",
                    data.files,
                    data.mark_path.as_deref().unwrap_or("unknown")
                )?;
            }
            "diff" | "run" => {
                if let Some(command) = &data.command {
                    writeln!(
                        w,
                        "{} exit={} {}ms",
                        command_string(command),
                        data.exit
                            .map_or_else(|| "unknown".to_owned(), |exit| exit.to_string()),
                        data.duration_ms.unwrap_or(0)
                    )?;
                }
                writeln!(w, "changed files: {}", data.changes.len())?;
                for change in &data.changes {
                    writeln!(
                        w,
                        "  {} {} {}",
                        change.action.as_str(),
                        change.path,
                        change.size_delta
                    )?;
                }
            }
            "list" => {
                for mark in &data.marks {
                    writeln!(w, "{}  files={}  {}", mark.name, mark.files, mark.path)?;
                }
            }
            "reset" => {
                writeln!(w, "removed {} mark(s)", data.removed)?;
            }
            _ => {}
        }
        Ok(())
    }

    fn render_json(&self, w: &mut dyn Write, _ctx: &RenderContext<'_>) -> RenderResult<()> {
        let envelope = JsonEnvelope::with_status(
            "axt.drift.v1",
            self.ok(),
            self.data(),
            Vec::new(),
            errors(self.data()),
        );
        serde_json::to_writer(&mut *w, &envelope)?;
        writeln!(w)?;
        Ok(())
    }

    fn render_agent(&self, w: &mut dyn Write, ctx: &RenderContext<'_>) -> RenderResult<()> {
        let data = self.data();
        let records = jsonl_detail_records(data);
        let truncated = jsonl_would_truncate(data, &records, ctx.limits)?;
        let mut writer = AgentJsonlWriter::new(w, ctx.limits);
        writer.write_record(&jsonl_summary(data, truncated, next_hints(data)))?;
        for record in &records {
            writer.write_record(record)?;
        }
        let _summary = writer.finish("axt.drift.warn.v1")?;
        Ok(())
    }
}

fn jsonl_summary<'a>(data: &'a DriftData, truncated: bool, next: Vec<String>) -> JsonlSummary<'a> {
    JsonlSummary {
        schema: "axt.drift.summary.v1",
        kind: "summary",
        ok: data.ok(),
        operation: data.operation.as_str(),
        name: data.name.as_deref(),
        files: data.files,
        hash_skipped_size: data.hash_skipped_size,
        changes: data.changes.len(),
        marks: data.marks.len(),
        removed: data.removed,
        exit: data.exit,
        truncated,
        next,
    }
}

fn next_hints(data: &DriftData) -> Vec<String> {
    let mut hints = Vec::new();
    if data.operation.as_str() == "mark" {
        if let Some(name) = data.name.as_deref() {
            hints.push(format!("axt-drift diff --since {name} --agent"));
        }
    }
    if !data.changes.is_empty() {
        hints.push("axt-peek . --changed --agent".to_owned());
    }
    hints
}

fn jsonl_detail_records(data: &DriftData) -> Vec<Value> {
    let mut records = Vec::with_capacity(data.changes.len() + data.marks.len());
    for change in &data.changes {
        records.push(json!({
            "schema": "axt.drift.file.v1",
            "type": "file",
            "path": change.path,
            "action": change.action.as_str(),
            "size_before": change.size_before,
            "size_after": change.size_after,
            "size_delta": change.size_delta,
            "hash": change.hash,
            "hash_skipped_size": change.hash_skipped_size
        }));
    }
    for mark in &data.marks {
        records.push(json!({
            "schema": "axt.drift.mark.v1",
            "type": "mark",
            "name": mark.name,
            "path": mark.path,
            "files": mark.files
        }));
    }
    records
}

fn jsonl_would_truncate(
    data: &DriftData,
    records: &[Value],
    limits: OutputLimits,
) -> RenderResult<bool> {
    let summary_len = serialized_jsonl_len(&jsonl_summary(data, false, Vec::new()))?;
    line_output_would_truncate(
        summary_len,
        records.iter().map(serialized_jsonl_len),
        limits,
    )
}

fn serialized_jsonl_len<T: Serialize>(record: &T) -> RenderResult<usize> {
    Ok(serde_json::to_vec(record)?.len() + 1)
}

fn line_output_would_truncate<I>(
    summary_len: usize,
    detail_lens: I,
    limits: OutputLimits,
) -> RenderResult<bool>
where
    I: IntoIterator<Item = RenderResult<usize>>,
{
    if limits.max_records == 0 || summary_len > limits.max_bytes {
        return Ok(true);
    }

    let mut bytes = summary_len;
    for (records, detail_len) in (1..).zip(detail_lens) {
        let detail_len = detail_len?;
        if records >= limits.max_records || bytes + detail_len > limits.max_bytes {
            return Ok(true);
        }
        bytes += detail_len;
    }
    Ok(false)
}

fn command_string(command: &crate::model::RunCommand) -> String {
    let mut parts = Vec::with_capacity(1 + command.args.len());
    parts.push(command.program.as_str());
    parts.extend(command.args.iter().map(String::as_str));
    parts.join(" ")
}

fn errors(data: &DriftData) -> Vec<OutputDiagnostic> {
    if data.exit.is_some_and(|exit| exit != 0) {
        vec![OutputDiagnostic {
            code: ErrorCode::CommandFailed,
            message: "command exited non-zero".to_owned(),
            context: json!({ "exit": data.exit }),
        }]
    } else {
        Vec::new()
    }
}
