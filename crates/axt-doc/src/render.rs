use std::io::Write;

use axt_output::{
    format_agent_fields, AgentCompactWriter, AgentField, JsonEnvelope, JsonlWriter, RenderContext,
    Renderable, Result as RenderResult,
};
use serde::Serialize;
use serde_json::json;

use crate::{
    model::{DocData, EnvReport, PathReport, WhichReport},
    output::DocOutput,
};

#[derive(Debug, Serialize)]
struct JsonlSummary {
    schema: &'static str,
    #[serde(rename = "type")]
    kind: &'static str,
    ok: bool,
    which_found: Option<bool>,
    path_entries: Option<usize>,
    env_vars: Option<usize>,
    secret_like: Option<usize>,
    truncated: bool,
}

impl DocOutput {
    pub fn render_json_data(&self, w: &mut dyn Write) -> RenderResult<()> {
        serde_json::to_writer(&mut *w, self.data())?;
        writeln!(w)?;
        Ok(())
    }
}

impl Renderable for DocOutput {
    fn render_human(&self, w: &mut dyn Write, _ctx: &RenderContext<'_>) -> RenderResult<()> {
        let data = self.data();
        if let Some(which) = &data.which {
            render_which_human(w, which)?;
        }
        if let Some(path) = &data.path {
            render_path_human(w, path)?;
        }
        if let Some(env) = &data.env {
            render_env_human(w, env)?;
        }
        Ok(())
    }

    fn render_json(&self, w: &mut dyn Write, _ctx: &RenderContext<'_>) -> RenderResult<()> {
        let envelope = JsonEnvelope::new("axt.doc.v1", self.data(), Vec::new(), Vec::new());
        serde_json::to_writer(&mut *w, &envelope)?;
        writeln!(w)?;
        Ok(())
    }

    fn render_jsonl(&self, w: &mut dyn Write, ctx: &RenderContext<'_>) -> RenderResult<()> {
        let data = self.data();
        let mut writer = JsonlWriter::new(w, ctx.limits);
        writer.write_record(&jsonl_summary(data))?;
        if let Some(which) = &data.which {
            writer.write_record(&json!({
                "schema": "axt.doc.which.v1",
                "type": "which",
                "cmd": which.cmd,
                "found": which.found,
                "primary": which.primary,
                "matches": which.matches.len(),
                "version_ok": which.version.ok,
                "version_timed_out": which.version.timed_out
            }))?;
            for item in &which.matches {
                writer.write_record(&json!({
                    "schema": "axt.doc.which.match.v1",
                    "type": "match",
                    "path": item.path,
                    "manager": item.manager,
                    "source": item.source,
                    "executable": item.executable
                }))?;
            }
        }
        if let Some(path) = &data.path {
            for item in &path.entries {
                writer.write_record(&json!({
                    "schema": "axt.doc.path.entry.v1",
                    "type": "path_entry",
                    "index": item.index,
                    "path": item.path,
                    "exists": item.exists,
                    "is_dir": item.is_dir,
                    "is_symlink": item.is_symlink,
                    "canonical": item.canonical,
                    "manager": item.manager
                }))?;
            }
        }
        if let Some(env) = &data.env {
            for item in &env.secret_like {
                writer.write_record(&json!({
                    "schema": "axt.doc.env.secret.v1",
                    "type": "secret_like",
                    "name": item.name,
                    "value": item.value,
                    "redacted": item.secret_like && !env.show_secrets
                }))?;
            }
            for item in &env.suspicious {
                writer.write_record(&json!({
                    "schema": "axt.doc.env.suspicion.v1",
                    "type": "suspicion",
                    "name": item.name,
                    "reason": item.reason
                }))?;
            }
        }
        let _summary = writer.finish("axt.doc.warn.v1")?;
        Ok(())
    }

    fn render_agent(&self, w: &mut dyn Write, ctx: &RenderContext<'_>) -> RenderResult<()> {
        let data = self.data();
        let mut writer = AgentCompactWriter::new(w, ctx.limits);
        writer.write_fields(&[
            AgentField::str("schema", "axt.doc.agent.v1"),
            AgentField::bool("ok", true),
            AgentField::str("mode", "records"),
            AgentField::str(
                "which",
                optional_bool(data.which.as_ref().map(|item| item.found)),
            ),
            AgentField::usize(
                "path_entries",
                data.path.as_ref().map_or(0, |item| item.entries.len()),
            ),
            AgentField::usize("env_vars", data.env.as_ref().map_or(0, |item| item.total)),
            AgentField::bool("truncated", false),
        ])?;
        if let Some(which) = &data.which {
            writer.write_line(&prefixed_line(
                "D",
                &[
                    AgentField::str("kind", "which"),
                    AgentField::str("cmd", &which.cmd),
                    AgentField::bool("found", which.found),
                    AgentField::str("path", which.primary.as_deref().unwrap_or("none")),
                ],
            )?)?;
        }
        if let Some(path) = &data.path {
            for issue in &path.ordering_issues {
                writer.write_line(&prefixed_line(
                    "W",
                    &[
                        AgentField::str("code", &issue.kind),
                        AgentField::str("path", &issue.path),
                        AgentField::str("hint", &issue.message),
                    ],
                )?)?;
            }
            for missing in &path.missing {
                writer.write_line(&prefixed_line(
                    "W",
                    &[
                        AgentField::str("code", "path_not_found"),
                        AgentField::str("path", missing),
                    ],
                )?)?;
            }
        }
        if let Some(env) = &data.env {
            for secret in &env.secret_like {
                writer.write_line(&prefixed_line(
                    "W",
                    &[
                        AgentField::str("code", "secret_like_env"),
                        AgentField::str("name", &secret.name),
                    ],
                )?)?;
            }
            for suspicion in &env.suspicious {
                writer.write_line(&prefixed_line(
                    "W",
                    &[
                        AgentField::str("code", "suspicious_env"),
                        AgentField::str("name", &suspicion.name),
                        AgentField::str("hint", &suspicion.reason),
                    ],
                )?)?;
            }
        }
        let _summary = writer.finish()?;
        Ok(())
    }
}

fn render_which_human(w: &mut dyn Write, which: &WhichReport) -> RenderResult<()> {
    writeln!(w, "which {}", which.cmd)?;
    if let Some(primary) = &which.primary {
        writeln!(w, "  primary: {primary}")?;
    } else {
        writeln!(w, "  primary: not found")?;
    }
    for item in &which.matches {
        writeln!(
            w,
            "  match: {}{}",
            item.path,
            item.manager
                .as_ref()
                .map_or_else(String::new, |manager| format!(" ({manager})"))
        )?;
    }
    if which.version.attempted {
        writeln!(
            w,
            "  version: {}",
            which
                .version
                .output
                .as_deref()
                .or(which.version.error.as_deref())
                .unwrap_or("no output")
        )?;
    }
    Ok(())
}

fn render_path_human(w: &mut dyn Write, path: &PathReport) -> RenderResult<()> {
    writeln!(w, "path")?;
    writeln!(w, "  entries: {}", path.entries.len())?;
    writeln!(w, "  duplicates: {}", path.duplicates.len())?;
    writeln!(w, "  missing: {}", path.missing.len())?;
    writeln!(w, "  broken symlinks: {}", path.broken_symlinks.len())?;
    for item in &path.ordering_issues {
        writeln!(w, "  warning: {}", item.message)?;
    }
    Ok(())
}

fn render_env_human(w: &mut dyn Write, env: &EnvReport) -> RenderResult<()> {
    writeln!(w, "env")?;
    writeln!(w, "  vars: {}", env.total)?;
    writeln!(w, "  secret-like: {}", env.secret_like.len())?;
    writeln!(w, "  suspicious: {}", env.suspicious.len())?;
    for item in &env.secret_like {
        writeln!(w, "  secret-like: {}={}", item.name, item.value)?;
    }
    Ok(())
}

fn jsonl_summary(data: &DocData) -> JsonlSummary {
    JsonlSummary {
        schema: "axt.doc.summary.v1",
        kind: "summary",
        ok: true,
        which_found: data.which.as_ref().map(|item| item.found),
        path_entries: data.path.as_ref().map(|item| item.entries.len()),
        env_vars: data.env.as_ref().map(|item| item.total),
        secret_like: data.env.as_ref().map(|item| item.secret_like.len()),
        truncated: false,
    }
}

fn optional_bool(value: Option<bool>) -> &'static str {
    match value {
        Some(true) => "true",
        Some(false) => "false",
        None => "none",
    }
}

fn prefixed_line(prefix: &str, fields: &[AgentField<'_>]) -> RenderResult<String> {
    let mut line = prefix.to_owned();
    let formatted = format_agent_fields(fields)?;
    if !formatted.is_empty() {
        line.push(' ');
        line.push_str(&formatted);
    }
    Ok(line)
}
