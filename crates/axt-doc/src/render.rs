use std::io::Write;

use axt_output::{
    AgentJsonlWriter, JsonEnvelope, RenderContext, Renderable, Result as RenderResult,
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
    next: Vec<String>,
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

    fn render_agent(&self, w: &mut dyn Write, ctx: &RenderContext<'_>) -> RenderResult<()> {
        let data = self.data();
        let mut writer = AgentJsonlWriter::new(w, ctx.limits);
        writer.write_record(&jsonl_summary(data, next_hints(data)))?;
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
            for issue in &path.ordering_issues {
                writer.write_record(&json!({
                    "schema": "axt.doc.warn.v1",
                    "type": "warn",
                    "code": issue.kind,
                    "path": issue.path,
                    "hint": issue.message
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

fn jsonl_summary(data: &DocData, next: Vec<String>) -> JsonlSummary {
    JsonlSummary {
        schema: "axt.doc.summary.v1",
        kind: "summary",
        ok: true,
        which_found: data.which.as_ref().map(|item| item.found),
        path_entries: data.path.as_ref().map(|item| item.entries.len()),
        env_vars: data.env.as_ref().map(|item| item.total),
        secret_like: data.env.as_ref().map(|item| item.secret_like.len()),
        truncated: false,
        next,
    }
}

fn next_hints(data: &DocData) -> Vec<String> {
    let mut hints = Vec::new();
    if let Some(which) = &data.which {
        if !which.found {
            hints.push(format!("axt-doc which {} --json", which.cmd));
        }
    }
    if let Some(env) = &data.env {
        if !env.secret_like.is_empty() {
            hints.push("axt-doc env --json".to_owned());
        }
    }
    hints
}
