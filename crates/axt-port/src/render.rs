use std::io::Write;

use axt_core::{ErrorCode, OutputLimits};
use axt_output::{
    format_agent_fields, AgentCompactWriter, AgentField, JsonEnvelope, JsonlWriter,
    OutputDiagnostic, RenderContext, Renderable, Result as RenderResult,
};
use serde::Serialize;
use serde_json::{json, Value};

use crate::{
    model::{FreeAttempt, PortData, PortHolder},
    output::PortOutput,
};

#[derive(Debug, Serialize)]
struct JsonlSummary<'a> {
    schema: &'static str,
    #[serde(rename = "type")]
    kind: &'static str,
    ok: bool,
    action: &'static str,
    ports: &'a [u16],
    sockets: usize,
    holders: usize,
    held: bool,
    freed: bool,
    timed_out: bool,
    duration_ms: u64,
    truncated: bool,
}

impl PortOutput {
    pub fn render_json_data(&self, w: &mut dyn Write) -> RenderResult<()> {
        serde_json::to_writer(&mut *w, self.data())?;
        writeln!(w)?;
        Ok(())
    }
}

impl Renderable for PortOutput {
    fn render_human(&self, w: &mut dyn Write, _ctx: &RenderContext<'_>) -> RenderResult<()> {
        let data = self.data();
        match data.action.as_str() {
            "list" => {
                writeln!(
                    w,
                    "Port    Proto  PID    Process       Bound          State"
                )?;
                for socket in &data.sockets {
                    writeln!(
                        w,
                        "{:<7} {:<6} {:<6} {:<13} {:<14} {}",
                        socket.port,
                        socket.proto.as_str(),
                        socket
                            .pid
                            .map_or_else(|| "-".to_owned(), |pid| pid.to_string()),
                        socket.process.as_deref().unwrap_or("-"),
                        socket.bound,
                        socket.state
                    )?;
                }
            }
            "who" | "watch" => {
                if data.holders.is_empty() {
                    writeln!(w, "No holder found.")?;
                }
                for holder in &data.holders {
                    writeln!(
                        w,
                        "Port {} ({}, listening)",
                        holder.port,
                        holder.proto.as_str()
                    )?;
                    writeln!(
                        w,
                        "  PID {:<8} {}    {}",
                        holder.pid,
                        holder.name,
                        holder.command.as_deref().unwrap_or("")
                    )?;
                    if let Some(cwd) = &holder.cwd {
                        writeln!(w, "  Cwd:         {cwd}")?;
                    }
                    writeln!(w, "  Bound:       {}", holder.bound.join("  "))?;
                    if let Some(owner) = &holder.owner {
                        writeln!(w, "  Owner:       {owner}")?;
                    }
                    if let Some(memory) = holder.memory_bytes {
                        writeln!(w, "  Memory:      {memory} bytes")?;
                    }
                }
                if data.timed_out {
                    writeln!(w, "Timed out after {}ms.", data.duration_ms)?;
                }
            }
            "free" => {
                if data.attempts.is_empty() {
                    writeln!(w, "No holder found.")?;
                }
                for attempt in &data.attempts {
                    writeln!(
                        w,
                        "Port {} held by PID {} ({})",
                        attempt.port, attempt.pid, attempt.name
                    )?;
                    writeln!(
                        w,
                        "{} {}: {}",
                        attempt.action.as_str(),
                        attempt.signal,
                        attempt.result.as_str()
                    )?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn render_json(&self, w: &mut dyn Write, _ctx: &RenderContext<'_>) -> RenderResult<()> {
        let envelope = JsonEnvelope::with_status(
            "axt.port.v1",
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
        let truncated = line_output_would_truncate(
            serialized_jsonl_len(&jsonl_summary(data, false))?,
            records.iter().map(serialized_jsonl_len),
            ctx.limits,
        )?;
        let mut writer = JsonlWriter::new(w, ctx.limits);
        writer.write_record(&jsonl_summary(data, truncated))?;
        for record in &records {
            writer.write_record(record)?;
        }
        let _summary = writer.finish("axt.port.warn.v1")?;
        Ok(())
    }

    fn render_agent(&self, w: &mut dyn Write, ctx: &RenderContext<'_>) -> RenderResult<()> {
        let data = self.data();
        let mut lines = Vec::new();
        for holder in &data.holders {
            lines.push(agent_holder_line(holder)?);
        }
        for attempt in &data.attempts {
            lines.push(agent_attempt_line(attempt)?);
        }
        for error in errors(data) {
            lines.push(agent_error_line(&error)?);
        }
        let truncated = line_output_would_truncate(
            agent_summary_line(data, false)?.len() + 1,
            lines.iter().map(|line| Ok(line.len() + 1)),
            ctx.limits,
        )?;
        let mut writer = AgentCompactWriter::new(w, ctx.limits);
        writer.write_line(&agent_summary_line(data, truncated)?)?;
        for line in &lines {
            writer.write_line(line)?;
        }
        let _summary = writer.finish()?;
        Ok(())
    }
}

fn jsonl_summary(data: &PortData, truncated: bool) -> JsonlSummary<'_> {
    JsonlSummary {
        schema: "axt.port.summary.v1",
        kind: "summary",
        ok: data.ok(),
        action: data.action.as_str(),
        ports: &data.ports,
        sockets: data.sockets.len(),
        holders: data.holders.len(),
        held: data.held,
        freed: data.freed,
        timed_out: data.timed_out,
        duration_ms: data.duration_ms,
        truncated,
    }
}

fn jsonl_detail_records(data: &PortData) -> Vec<Value> {
    let mut records = Vec::new();
    records.extend(data.sockets.iter().map(|socket| {
        json!({
            "schema": "axt.port.socket.v1",
            "type": "socket",
            "port": socket.port,
            "proto": socket.proto,
            "pid": socket.pid,
            "process": socket.process,
            "bound": socket.bound,
            "state": socket.state
        })
    }));
    records.extend(data.holders.iter().map(|holder| {
        json!({
            "schema": "axt.port.holder.v1",
            "type": "holder",
            "port": holder.port,
            "proto": holder.proto,
            "pid": holder.pid,
            "name": holder.name,
            "cmd": holder.command,
            "cwd": holder.cwd,
            "bound": holder.bound,
            "owner": holder.owner,
            "mem": holder.memory_bytes,
            "started": holder.started
        })
    }));
    records.extend(data.attempts.iter().map(|attempt| {
        json!({
            "schema": "axt.port.action.v1",
            "type": "action",
            "port": attempt.port,
            "pid": attempt.pid,
            "name": attempt.name,
            "signal": attempt.signal,
            "action": attempt.action,
            "result": attempt.result,
            "ok": attempt.ok,
            "escalated": attempt.escalated,
            "ms": attempt.ms
        })
    }));
    records
}

fn serialized_jsonl_len<T: Serialize>(record: &T) -> RenderResult<usize> {
    Ok(serde_json::to_vec(record)?.len() + 1)
}

fn agent_summary_line(data: &PortData, truncated: bool) -> RenderResult<String> {
    let port = data
        .ports
        .first()
        .map_or_else(|| "all".to_owned(), u16::to_string);
    format_agent_fields(&[
        AgentField::str("schema", "axt.port.agent.v1"),
        AgentField::bool("ok", data.ok()),
        AgentField::str("mode", "records"),
        AgentField::str("action", data.action.as_str()),
        AgentField::str("port", &port),
        AgentField::bool("held", data.held),
        AgentField::usize("holders", data.holders.len()),
        AgentField::bool("freed", data.freed),
        AgentField::bool("timed_out", data.timed_out),
        AgentField::u64("ms", data.duration_ms),
        AgentField::bool("truncated", truncated),
    ])
}

fn agent_holder_line(holder: &PortHolder) -> RenderResult<String> {
    let bound = holder.bound.join(",");
    let mut fields = vec![
        AgentField::u64("port", u64::from(holder.port)),
        AgentField::str("proto", holder.proto.as_str()),
        AgentField::u64("pid", u64::from(holder.pid)),
        AgentField::str("name", &holder.name),
        AgentField::str("bound", &bound),
    ];
    if let Some(command) = &holder.command {
        fields.push(AgentField::str("cmd", command));
    }
    if let Some(cwd) = &holder.cwd {
        fields.push(AgentField::str("cwd", cwd));
    }
    if let Some(owner) = &holder.owner {
        fields.push(AgentField::str("owner", owner));
    }
    if let Some(memory) = holder.memory_bytes {
        fields.push(AgentField::u64("mem", memory));
    }
    if let Some(started) = &holder.started {
        fields.push(AgentField::str("started", started));
    }
    prefixed_line("H", &fields)
}

fn agent_attempt_line(attempt: &FreeAttempt) -> RenderResult<String> {
    let mut fields = vec![
        AgentField::u64("port", u64::from(attempt.port)),
        AgentField::u64("pid", u64::from(attempt.pid)),
        AgentField::str("name", &attempt.name),
        AgentField::str("signal", &attempt.signal),
        AgentField::str("action", attempt.action.as_str()),
        AgentField::str("result", attempt.result.as_str()),
        AgentField::bool("ok", attempt.ok),
        AgentField::bool("escalated", attempt.escalated),
        AgentField::u64("ms", attempt.ms),
    ];
    if let Some(message) = &attempt.message {
        fields.push(AgentField::str("message", message));
    }
    prefixed_line("A", &fields)
}

fn agent_error_line(error: &OutputDiagnostic) -> RenderResult<String> {
    let context = error.context.to_string();
    prefixed_line(
        "X",
        &[
            AgentField::str("code", error.code.as_str()),
            AgentField::str("message", &error.message),
            AgentField::str("context", &context),
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

fn errors(data: &PortData) -> Vec<OutputDiagnostic> {
    let mut errors = Vec::new();
    if data.timed_out {
        errors.push(OutputDiagnostic {
            code: ErrorCode::Timeout,
            message: "watch timed out".to_owned(),
            context: json!({ "ports": data.ports }),
        });
    }
    for attempt in &data.attempts {
        if !attempt.ok {
            let code = if attempt.error_code.as_deref() == Some("permission_denied") {
                ErrorCode::PermissionDenied
            } else {
                ErrorCode::CommandFailed
            };
            errors.push(OutputDiagnostic {
                code,
                message: attempt
                    .message
                    .clone()
                    .unwrap_or_else(|| attempt.result.as_str().to_owned()),
                context: json!({ "port": attempt.port, "pid": attempt.pid }),
            });
        }
    }
    errors
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
