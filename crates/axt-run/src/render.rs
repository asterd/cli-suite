use std::io::Write;

use axt_core::ErrorCode;
use axt_output::{
    AgentJsonlWriter, JsonEnvelope, OutputDiagnostic, RenderContext, Renderable,
    Result as RenderResult,
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
    next: Vec<String>,
}

#[derive(Debug, Serialize)]
struct JsonlFile<'a> {
    schema: &'static str,
    #[serde(rename = "type")]
    kind: &'static str,
    #[serde(rename = "p")]
    path: &'a str,
    #[serde(rename = "a")]
    action: &'static str,
    #[serde(rename = "b")]
    bytes: Option<u64>,
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

    fn render_compact(&self, w: &mut dyn Write, _ctx: &RenderContext<'_>) -> RenderResult<()> {
        match self {
            Self::Run(data) | Self::Show(data) => render_run_compact(w, data),
            Self::Stream { name, stream, text } => {
                writeln!(
                    w,
                    "run stream name={name} stream={stream} bytes={}",
                    text.len()
                )?;
                write!(w, "{text}")?;
                Ok(())
            }
            Self::List { runs } => {
                writeln!(w, "run list count={}", runs.len())?;
                for run in runs {
                    writeln!(
                        w,
                        "run name={} ok={} exit={} ms={} cmd={}",
                        run.name,
                        run.ok,
                        run.exit
                            .map_or_else(|| "timeout".to_owned(), |exit| exit.to_string()),
                        run.duration_ms,
                        run.command
                    )?;
                }
                Ok(())
            }
            Self::Clean { removed } => {
                writeln!(w, "run clean removed={removed}")?;
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

    fn render_agent(&self, w: &mut dyn Write, ctx: &RenderContext<'_>) -> RenderResult<()> {
        let mut writer = AgentJsonlWriter::new(w, ctx.limits);
        match self {
            Self::Run(data) | Self::Show(data) => {
                writer.write_record(&jsonl_summary(data, next_hints_run(data)))?;
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

fn render_run_compact(w: &mut dyn Write, data: &RunData) -> RenderResult<()> {
    writeln!(
        w,
        "run ok={} cmd={} exit={} ms={} stdout_lines={} stderr_lines={} changed={} saved={} truncated={}",
        data.ok(),
        command_string(data),
        data.exit
            .map_or_else(|| "timeout".to_owned(), |exit| exit.to_string()),
        data.duration_ms,
        data.stdout.lines,
        data.stderr.lines,
        data.changed_count,
        data.saved
            .as_ref()
            .map_or_else(|| "-".to_owned(), |saved| saved.name.clone()),
        data.truncated
    )?;
    for change in &data.changed {
        writeln!(
            w,
            "file action={} path={} bytes={}",
            change.action.as_str(),
            change.path,
            change
                .bytes
                .map_or_else(|| "-".to_owned(), |bytes| bytes.to_string())
        )?;
    }
    Ok(())
}

pub fn agent_summary_line(data: &RunData) -> RenderResult<String> {
    let summary = jsonl_summary(data, next_hints_run(data));
    Ok(serde_json::to_string(&summary)?)
}

fn jsonl_summary(data: &RunData, next: Vec<String>) -> JsonlSummaryOwned {
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
        next,
    }
}

fn next_hints_run(data: &RunData) -> Vec<String> {
    let mut hints = Vec::new();
    if let Some(saved) = &data.saved {
        if data.timed_out || data.exit != Some(0) {
            hints.push(format!("axt-run show {} --stderr", saved.name));
        }
    }
    if data.changed_count > 0 {
        hints.push("axt-peek . --changed --agent".to_owned());
    }
    hints
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
