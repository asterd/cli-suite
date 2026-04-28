use std::io::Write;

use axt_core::ErrorCode;
use axt_output::{
    format_agent_fields, AgentCompactWriter, AgentField, JsonEnvelope, JsonlWriter,
    OutputDiagnostic, RenderContext, Renderable, Result as RenderResult,
};
use serde::Serialize;
use serde_json::json;

use crate::{
    model::{FileChange, RunData},
    output::RunOutput,
};

#[derive(Debug, Serialize)]
struct JsonlSummaryOwned {
    schema: &'static str,
    #[serde(rename = "type")]
    kind: &'static str,
    ok: bool,
    cmd: String,
    exit: Option<i32>,
    ms: u64,
    stdout_lines: usize,
    stderr_lines: usize,
    changed: usize,
    saved: Option<String>,
    truncated: bool,
}

#[derive(Debug, Serialize)]
struct JsonlFile<'a> {
    schema: &'static str,
    #[serde(rename = "type")]
    kind: &'static str,
    path: &'a str,
    action: &'static str,
    bytes: Option<u64>,
}

impl RunOutput {
    pub fn render_json_data(&self, w: &mut dyn Write) -> RenderResult<()> {
        match self {
            Self::Run(data) | Self::Show(data) => serde_json::to_writer(&mut *w, data)?,
            Self::Stream { name, stream, text } => {
                serde_json::to_writer(
                    &mut *w,
                    &json!({ "name": name, "stream": stream, "text": text }),
                )?;
            }
            Self::List { runs } => {
                serde_json::to_writer(&mut *w, &json!({ "runs": runs }))?;
            }
            Self::Clean { removed } => {
                serde_json::to_writer(&mut *w, &json!({ "removed": removed }))?;
            }
        }
        writeln!(w)?;
        Ok(())
    }
}

impl Renderable for RunOutput {
    fn render_human(&self, w: &mut dyn Write, _ctx: &RenderContext<'_>) -> RenderResult<()> {
        match self {
            Self::Run(data) | Self::Show(data) => render_run_human(w, data),
            Self::Stream { text, .. } => {
                write!(w, "{text}")?;
                Ok(())
            }
            Self::List { runs } => {
                for run in runs {
                    writeln!(
                        w,
                        "{}  exit={}  {}ms  {}",
                        run.name,
                        run.exit
                            .map_or_else(|| "timeout".to_owned(), |exit| exit.to_string()),
                        run.duration_ms,
                        run.command
                    )?;
                }
                Ok(())
            }
            Self::Clean { removed } => {
                writeln!(w, "removed {removed} run(s)")?;
                Ok(())
            }
        }
    }

    fn render_json(&self, w: &mut dyn Write, _ctx: &RenderContext<'_>) -> RenderResult<()> {
        match self {
            Self::Run(data) | Self::Show(data) => {
                let envelope = JsonEnvelope::with_status(
                    "axt.run.v1",
                    data.ok(),
                    data,
                    Vec::new(),
                    errors(self),
                );
                serde_json::to_writer(&mut *w, &envelope)?;
            }
            Self::Stream { .. } | Self::List { .. } | Self::Clean { .. } => {
                let envelope = JsonEnvelope::with_status(
                    "axt.run.v1",
                    self.ok(),
                    self,
                    Vec::new(),
                    errors(self),
                );
                serde_json::to_writer(&mut *w, &envelope)?;
            }
        }
        writeln!(w)?;
        Ok(())
    }

    fn render_jsonl(&self, w: &mut dyn Write, ctx: &RenderContext<'_>) -> RenderResult<()> {
        let mut writer = JsonlWriter::new(w, ctx.limits);
        match self {
            Self::Run(data) | Self::Show(data) => {
                writer.write_record(&jsonl_summary(data))?;
                for change in &data.changed {
                    writer.write_record(&jsonl_file(change))?;
                }
            }
            Self::Stream { name, stream, text } => {
                writer.write_record(&json!({
                    "schema": "axt.run.stream.v1",
                    "type": "stream",
                    "name": name,
                    "stream": stream,
                    "text": text
                }))?;
            }
            Self::List { runs } => {
                writer.write_record(&json!({
                    "schema": "axt.run.list.v1",
                    "type": "summary",
                    "runs": runs.len()
                }))?;
                for run in runs {
                    writer.write_record(&json!({
                        "schema": "axt.run.list.entry.v1",
                        "type": "run",
                        "name": run.name,
                        "path": run.path,
                        "created_at": run.created_at,
                        "ok": run.ok,
                        "exit": run.exit,
                        "duration_ms": run.duration_ms,
                        "command": run.command
                    }))?;
                }
            }
            Self::Clean { removed } => {
                writer.write_record(&json!({
                    "schema": "axt.run.clean.v1",
                    "type": "summary",
                    "removed": removed
                }))?;
            }
        }
        let _summary = writer.finish("axt.run.warn.v1")?;
        Ok(())
    }

    fn render_agent(&self, w: &mut dyn Write, ctx: &RenderContext<'_>) -> RenderResult<()> {
        let mut writer = AgentCompactWriter::new(w, ctx.limits);
        match self {
            Self::Run(data) | Self::Show(data) => {
                writer.write_line(&agent_summary_line(data)?)?;
                if data.timed_out {
                    writer.write_line("X code=timeout")?;
                } else if data.exit != Some(0) {
                    writer.write_line(&format!(
                        "X code=command_failed exit={}",
                        data.exit.unwrap_or(-1)
                    ))?;
                }
                for (index, line) in data.stderr.tail.iter().enumerate() {
                    writer.write_line(&agent_prefixed_line(
                        "E",
                        &[
                            AgentField::str("stream", "stderr"),
                            AgentField::usize("line", index + 1),
                            AgentField::str("text", line),
                        ],
                    )?)?;
                }
                for change in &data.changed {
                    writer.write_line(&agent_file_line(change)?)?;
                }
                if let Some(saved) = &data.saved {
                    writer.write_line(&agent_prefixed_line(
                        "S",
                        &[AgentField::str(
                            "run",
                            &format!("axt-run show {}", saved.name),
                        )],
                    )?)?;
                }
            }
            Self::Stream { name, stream, text } => {
                writer.write_fields(&[
                    AgentField::str("schema", "axt.run.agent.v1"),
                    AgentField::bool("ok", true),
                    AgentField::str("mode", "records"),
                    AgentField::str("name", name),
                    AgentField::str("stream", stream),
                    AgentField::bool("truncated", false),
                ])?;
                for line in text.lines() {
                    writer.write_line(&agent_prefixed_line(
                        "D",
                        &[
                            AgentField::str("stream", stream),
                            AgentField::str("text", line),
                        ],
                    )?)?;
                }
            }
            Self::List { runs } => {
                writer.write_fields(&[
                    AgentField::str("schema", "axt.run.agent.v1"),
                    AgentField::bool("ok", true),
                    AgentField::str("mode", "records"),
                    AgentField::usize("runs", runs.len()),
                    AgentField::bool("truncated", false),
                ])?;
                for run in runs {
                    writer.write_line(&agent_prefixed_line(
                        "R",
                        &[
                            AgentField::str("name", &run.name),
                            AgentField::str("cmd", &run.command),
                            AgentField::i64("exit", i64::from(run.exit.unwrap_or(-1))),
                            AgentField::u64("ms", run.duration_ms),
                        ],
                    )?)?;
                }
            }
            Self::Clean { removed } => {
                writer.write_fields(&[
                    AgentField::str("schema", "axt.run.agent.v1"),
                    AgentField::bool("ok", true),
                    AgentField::str("mode", "records"),
                    AgentField::usize("removed", *removed),
                    AgentField::bool("truncated", false),
                ])?;
            }
        }
        let _summary = writer.finish()?;
        Ok(())
    }
}

pub fn agent_summary_line(data: &RunData) -> RenderResult<String> {
    format_agent_fields(&[
        AgentField::str("schema", "axt.run.agent.v1"),
        AgentField::bool("ok", data.ok()),
        AgentField::str("mode", "records"),
        AgentField::str("cmd", &command_string(data)),
        AgentField::str(
            "exit",
            &data
                .exit
                .map_or_else(|| "timeout".to_owned(), |exit| exit.to_string()),
        ),
        AgentField::u64("ms", data.duration_ms),
        AgentField::usize("stdout_lines", data.stdout.lines),
        AgentField::usize("stderr_lines", data.stderr.lines),
        AgentField::usize("changed", data.changed_count),
        AgentField::str(
            "saved",
            data.saved
                .as_ref()
                .map_or("none", |saved| saved.name.as_str()),
        ),
        AgentField::bool("truncated", data.truncated),
    ])
}

fn render_run_human(w: &mut dyn Write, data: &RunData) -> RenderResult<()> {
    writeln!(
        w,
        "{} exit={} {}ms",
        command_string(data),
        data.exit
            .map_or_else(|| "timeout".to_owned(), |exit| exit.to_string()),
        data.duration_ms
    )?;
    writeln!(
        w,
        "stdout: {} bytes, {} lines{}",
        data.stdout.bytes,
        data.stdout.lines,
        log_suffix(&data.stdout.log)
    )?;
    writeln!(
        w,
        "stderr: {} bytes, {} lines{}",
        data.stderr.bytes,
        data.stderr.lines,
        log_suffix(&data.stderr.log)
    )?;
    if !data.stderr.tail.is_empty() {
        writeln!(w, "stderr tail:")?;
        for line in &data.stderr.tail {
            writeln!(w, "  {line}")?;
        }
    }
    if !data.changed.is_empty() {
        writeln!(w, "changed files:")?;
        for change in &data.changed {
            writeln!(
                w,
                "  {} {} {}",
                change.action.as_str(),
                change.path,
                change
                    .bytes
                    .map_or_else(|| "-".to_owned(), |bytes| bytes.to_string())
            )?;
        }
    }
    if let Some(saved) = &data.saved {
        writeln!(w, "saved: {}", saved.path)?;
    }
    Ok(())
}

fn jsonl_summary(data: &RunData) -> JsonlSummaryOwned {
    JsonlSummaryOwned {
        schema: "axt.run.summary.v1",
        kind: "summary",
        ok: data.ok(),
        cmd: command_string(data),
        exit: data.exit,
        ms: data.duration_ms,
        stdout_lines: data.stdout.lines,
        stderr_lines: data.stderr.lines,
        changed: data.changed_count,
        saved: data.saved.as_ref().map(|saved| saved.name.clone()),
        truncated: data.truncated,
    }
}

fn jsonl_file(change: &FileChange) -> JsonlFile<'_> {
    JsonlFile {
        schema: "axt.run.file.v1",
        kind: "file",
        path: &change.path,
        action: change.action.as_str(),
        bytes: change.bytes,
    }
}

fn agent_file_line(change: &FileChange) -> RenderResult<String> {
    agent_prefixed_line(
        "F",
        &[
            AgentField::str("path", &change.path),
            AgentField::str("action", change.action.as_str()),
            AgentField::str(
                "bytes",
                &change
                    .bytes
                    .map_or_else(|| "none".to_owned(), |bytes| bytes.to_string()),
            ),
        ],
    )
}

fn agent_prefixed_line(prefix: &str, fields: &[AgentField<'_>]) -> RenderResult<String> {
    let mut line = prefix.to_owned();
    let formatted = format_agent_fields(fields)?;
    if !formatted.is_empty() {
        line.push(' ');
        line.push_str(&formatted);
    }
    Ok(line)
}

fn command_string(data: &RunData) -> String {
    let mut parts = Vec::with_capacity(1 + data.command.args.len());
    parts.push(data.command.program.as_str());
    parts.extend(data.command.args.iter().map(String::as_str));
    parts.join(" ")
}

fn log_suffix(log: &Option<String>) -> String {
    log.as_ref()
        .map_or_else(String::new, |path| format!(" ({path})"))
}

fn errors(output: &RunOutput) -> Vec<OutputDiagnostic> {
    let data = match output {
        RunOutput::Run(data) | RunOutput::Show(data) => data,
        RunOutput::Stream { .. } | RunOutput::List { .. } | RunOutput::Clean { .. } => {
            return Vec::new();
        }
    };
    if data.timed_out {
        vec![OutputDiagnostic {
            code: ErrorCode::Timeout,
            message: "command timed out".to_owned(),
            context: json!({ "command": command_string(data) }),
        }]
    } else if data.exit != Some(0) {
        vec![OutputDiagnostic {
            code: ErrorCode::CommandFailed,
            message: "command exited non-zero".to_owned(),
            context: json!({ "exit": data.exit, "command": command_string(data) }),
        }]
    } else {
        Vec::new()
    }
}
