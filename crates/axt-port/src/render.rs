use std::io::Write;

use axt_core::{ErrorCode, OutputLimits};
use axt_output::{
    AgentJsonlWriter, JsonEnvelope, OutputDiagnostic, RenderContext, Renderable,
    Result as RenderResult,
};
use serde::Serialize;
use serde_json::{json, Value};

use crate::{model::PortData, output::PortOutput};

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
    next: Vec<String>,
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

    fn render_compact(&self, w: &mut dyn Write, _ctx: &RenderContext<'_>) -> RenderResult<()> {
        let data = self.data();
        writeln!(
            w,
            "port action={} ok={} ports={:?} sockets={} holders={} held={} freed={} timed_out={} ms={}",
            data.action.as_str(),
            data.ok(),
            data.ports,
            data.sockets.len(),
            data.holders.len(),
            data.held,
            data.freed,
            data.timed_out,
            data.duration_ms
        )?;
        for socket in &data.sockets {
            writeln!(
                w,
                "socket port={} proto={} pid={} process={} bound={} state={}",
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
        for holder in &data.holders {
            writeln!(
                w,
                "holder port={} proto={} pid={} name={} bound={}",
                holder.port,
                holder.proto.as_str(),
                holder.pid,
                holder.name,
                holder.bound.join(",")
            )?;
        }
        for attempt in &data.attempts {
            writeln!(
                w,
                "action port={} pid={} signal={} result={} ok={}",
                attempt.port,
                attempt.pid,
                attempt.signal,
                attempt.result.as_str(),
                attempt.ok
            )?;
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

    fn render_agent(&self, w: &mut dyn Write, ctx: &RenderContext<'_>) -> RenderResult<()> {
        let data = self.data();
        let records = jsonl_detail_records(data);
        let truncated = line_output_would_truncate(
            serialized_jsonl_len(&jsonl_summary(data, false, Vec::new()))?,
            records.iter().map(serialized_jsonl_len),
            ctx.limits,
        )?;
        let mut writer = AgentJsonlWriter::new(w, ctx.limits);
        writer.write_record(&jsonl_summary(data, truncated, next_hints(data)))?;
        for record in &records {
            writer.write_record(record)?;
        }
        let _summary = writer.finish("axt.port.warn.v1")?;
        Ok(())
    }
}

fn jsonl_summary(data: &PortData, truncated: bool, next: Vec<String>) -> JsonlSummary<'_> {
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
        next,
    }
}

fn next_hints(data: &PortData) -> Vec<String> {
    let mut hints = Vec::new();
    if data.held && !data.freed {
        if let Some(port) = data.ports.first() {
            hints.push(format!("axt-port free {port} --dry-run --agent"));
        }
    }
    hints
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
