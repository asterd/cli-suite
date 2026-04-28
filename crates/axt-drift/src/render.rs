use std::io::Write;

use axt_core::{ErrorCode, OutputLimits};
use axt_output::{
    format_agent_fields, AgentCompactWriter, AgentField, JsonEnvelope, JsonlWriter,
    OutputDiagnostic, RenderContext, Renderable, Result as RenderResult,
};
use serde::Serialize;
use serde_json::{json, Value};

use crate::{
    model::{DriftData, FileChange},
    output::DriftOutput,
};

#[derive(Debug, Serialize)]
struct JsonlSummary<'a> {
    schema: &'static str,
    #[serde(rename = "type")]
    kind: &'static str,
    ok: bool,
    operation: &'static str,
    name: Option<&'a str>,
    files: usize,
    changes: usize,
    marks: usize,
    removed: usize,
    exit: Option<i32>,
    truncated: bool,
}

impl DriftOutput {
    pub fn render_json_data(&self, w: &mut dyn Write) -> RenderResult<()> {
        serde_json::to_writer(&mut *w, self.data())?;
        writeln!(w)?;
        Ok(())
    }
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

    fn render_jsonl(&self, w: &mut dyn Write, ctx: &RenderContext<'_>) -> RenderResult<()> {
        let data = self.data();
        let records = jsonl_detail_records(data);
        let truncated = jsonl_would_truncate(data, &records, ctx.limits)?;
        let mut writer = JsonlWriter::new(w, ctx.limits);
        writer.write_record(&jsonl_summary(data, truncated))?;
        for record in &records {
            writer.write_record(record)?;
        }
        let _summary = writer.finish("axt.drift.warn.v1")?;
        Ok(())
    }

    fn render_agent(&self, w: &mut dyn Write, ctx: &RenderContext<'_>) -> RenderResult<()> {
        let data = self.data();
        let mut lines = Vec::new();
        if let Some(exit) = data.exit {
            if exit != 0 {
                lines.push(format!("X code=command_failed exit={exit}"));
            }
        }
        for change in &data.changes {
            lines.push(agent_file_line(change)?);
        }
        for mark in &data.marks {
            lines.push(prefixed_line(
                "D",
                &[
                    AgentField::str("kind", "mark"),
                    AgentField::str("name", &mark.name),
                    AgentField::usize("files", mark.files),
                    AgentField::str("path", &mark.path),
                ],
            )?);
        }
        let truncated = agent_would_truncate(data, &lines, ctx.limits)?;
        let mut writer = AgentCompactWriter::new(w, ctx.limits);
        writer.write_fields(&[
            AgentField::str("schema", "axt.drift.agent.v1"),
            AgentField::bool("ok", data.ok()),
            AgentField::str("mode", "records"),
            AgentField::str("operation", data.operation.as_str()),
            AgentField::str("name", data.name.as_deref().unwrap_or("none")),
            AgentField::usize("files", data.files),
            AgentField::usize("changed", data.changes.len()),
            AgentField::usize("marks", data.marks.len()),
            AgentField::usize("removed", data.removed),
            AgentField::bool("truncated", truncated),
        ])?;
        for line in &lines {
            writer.write_line(line)?;
        }
        let _summary = writer.finish()?;
        Ok(())
    }
}

fn jsonl_summary(data: &DriftData, truncated: bool) -> JsonlSummary<'_> {
    JsonlSummary {
        schema: "axt.drift.summary.v1",
        kind: "summary",
        ok: data.ok(),
        operation: data.operation.as_str(),
        name: data.name.as_deref(),
        files: data.files,
        changes: data.changes.len(),
        marks: data.marks.len(),
        removed: data.removed,
        exit: data.exit,
        truncated,
    }
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
            "hash": change.hash
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
    let summary_len = serialized_jsonl_len(&jsonl_summary(data, false))?;
    line_output_would_truncate(
        summary_len,
        records.iter().map(serialized_jsonl_len),
        limits,
    )
}

fn serialized_jsonl_len<T: Serialize>(record: &T) -> RenderResult<usize> {
    Ok(serde_json::to_vec(record)?.len() + 1)
}

fn agent_would_truncate(
    data: &DriftData,
    lines: &[String],
    limits: OutputLimits,
) -> RenderResult<bool> {
    let summary = format_agent_fields(&[
        AgentField::str("schema", "axt.drift.agent.v1"),
        AgentField::bool("ok", data.ok()),
        AgentField::str("mode", "records"),
        AgentField::str("operation", data.operation.as_str()),
        AgentField::str("name", data.name.as_deref().unwrap_or("none")),
        AgentField::usize("files", data.files),
        AgentField::usize("changed", data.changes.len()),
        AgentField::usize("marks", data.marks.len()),
        AgentField::usize("removed", data.removed),
        AgentField::bool("truncated", false),
    ])?;
    line_output_would_truncate(
        summary.len() + 1,
        lines.iter().map(|line| Ok(line.len() + 1)),
        limits,
    )
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
    for (records, detail_len) in (1..).zip(detail_lens.into_iter()) {
        let detail_len = detail_len?;
        if records >= limits.max_records || bytes + detail_len > limits.max_bytes {
            return Ok(true);
        }
        bytes += detail_len;
    }
    Ok(false)
}

fn agent_file_line(change: &FileChange) -> RenderResult<String> {
    prefixed_line(
        "F",
        &[
            AgentField::str("path", &change.path),
            AgentField::str("action", change.action.as_str()),
            AgentField::i64("size_delta", change.size_delta),
        ],
    )
}

fn prefixed_line(prefix: &str, fields: &[AgentField<'_>]) -> RenderResult<String> {
    let formatted = format_agent_fields(fields)?;
    if formatted.is_empty() {
        Ok(prefix.to_owned())
    } else {
        Ok(format!("{prefix} {formatted}"))
    }
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
