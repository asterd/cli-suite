use std::io::Write;

use axt_core::ErrorCode;
use axt_output::{
    AgentJsonlWriter, JsonEnvelope, OutputDiagnostic, RenderContext, Renderable,
    Result as RenderResult,
};
use serde::Serialize;
use serde_json::json;

use crate::model::{Entry, PeekData, PeekWarning, Summary, WarningCode};

#[derive(Debug, Serialize)]
struct JsonlSummary<'a> {
    schema: &'static str,
    #[serde(rename = "type")]
    kind: &'static str,
    ok: bool,
    root: &'a str,
    files: usize,
    dirs: usize,
    bytes: u64,
    git: &'static str,
    modified: usize,
    untracked: usize,
    ignored: usize,
    truncated: bool,
    next: Vec<String>,
}

#[derive(Debug, Serialize)]
struct JsonlEntry<'a> {
    schema: &'static str,
    #[serde(rename = "type")]
    kind: &'static str,
    #[serde(rename = "p")]
    path: &'a str,
    #[serde(rename = "b")]
    bytes: u64,
    #[serde(rename = "l")]
    lang: Option<&'a str>,
    #[serde(rename = "g")]
    git: &'static str,
    #[serde(rename = "ts")]
    mtime: Option<&'a str>,
}

#[derive(Debug, Serialize)]
struct JsonlWarning<'a> {
    schema: &'static str,
    #[serde(rename = "type")]
    kind: &'static str,
    code: &'static str,
    path: Option<&'a str>,
    reason: &'a str,
}

impl Renderable for PeekData {
    fn render_human(&self, w: &mut dyn Write, _ctx: &RenderContext<'_>) -> RenderResult<()> {
        writeln!(w, "{}/", self.root)?;
        for entry in &self.entries {
            writeln!(
                w,
                "  {:<32} {:>8}  {:<10} {}",
                display_path(entry),
                human_bytes(entry.bytes),
                entry.language.as_deref().unwrap_or(""),
                entry.git.as_str()
            )?;
        }
        writeln!(w)?;
        writeln!(w, "Summary")?;
        writeln!(
            w,
            "  files     {:<8} modified   {}",
            self.summary.files, self.summary.modified
        )?;
        writeln!(
            w,
            "  dirs      {:<8} untracked  {}",
            self.summary.dirs, self.summary.untracked
        )?;
        writeln!(
            w,
            "  bytes     {:<8} ignored    {}",
            human_bytes(self.summary.bytes),
            self.summary.ignored
        )?;
        writeln!(
            w,
            "  git       {:<8} truncated  {}",
            self.summary.git_state.as_str(),
            yes_no(self.summary.truncated)
        )?;
        Ok(())
    }

    fn render_json(&self, w: &mut dyn Write, _ctx: &RenderContext<'_>) -> RenderResult<()> {
        let envelope = JsonEnvelope::new(
            "axt.peek.v1",
            self,
            json_warnings(&self.warnings),
            Vec::new(),
        );
        serde_json::to_writer(&mut *w, &envelope)?;
        writeln!(w)?;
        Ok(())
    }

    fn render_agent(&self, w: &mut dyn Write, ctx: &RenderContext<'_>) -> RenderResult<()> {
        let records = jsonl_records(self);
        let truncated = jsonl_output_truncated(&records, ctx);
        let mut writer = AgentJsonlWriter::new(w, ctx.limits);
        writer.write_record(&jsonl_summary(
            &self.root,
            &self.summary,
            truncated,
            next_hints(self),
        ))?;
        for entry in &self.entries {
            writer.write_record(&jsonl_entry(entry))?;
        }
        for warning in &self.warnings {
            writer.write_record(&jsonl_warning(warning))?;
        }
        let _summary = writer.finish("axt.peek.warn.v1")?;
        Ok(())
    }
}

fn json_warnings(warnings: &[PeekWarning]) -> Vec<OutputDiagnostic> {
    warnings
        .iter()
        .map(|warning| OutputDiagnostic {
            code: warning_error_code(warning.code),
            message: warning.reason.clone(),
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
        WarningCode::SymlinkLoop | WarningCode::PathNotUtf8 | WarningCode::GitCapped => {
            ErrorCode::IoError
        }
    }
}

fn jsonl_summary<'a>(
    root: &'a str,
    summary: &Summary,
    truncated: bool,
    next: Vec<String>,
) -> JsonlSummary<'a> {
    JsonlSummary {
        schema: "axt.peek.summary.v1",
        kind: "summary",
        ok: true,
        root,
        files: summary.files,
        dirs: summary.dirs,
        bytes: summary.bytes,
        git: summary.git_state.as_str(),
        modified: summary.modified,
        untracked: summary.untracked,
        ignored: summary.ignored,
        truncated,
        next,
    }
}

fn next_hints(data: &PeekData) -> Vec<String> {
    let mut hints = Vec::new();
    if let Some(largest) = data
        .entries
        .iter()
        .filter(|entry| entry.kind == crate::model::EntryKind::File)
        .max_by_key(|entry| entry.bytes)
    {
        hints.push(format!("axt-outline {} --agent", largest.path));
    }
    if data.summary.modified > 0 || data.summary.untracked > 0 {
        hints.push("axt-peek . --changed --agent".to_owned());
    }
    hints
}

fn jsonl_records(data: &PeekData) -> Vec<Vec<u8>> {
    let mut records = Vec::with_capacity(1 + data.entries.len() + data.warnings.len());
    if let Ok(record) = serde_json::to_vec(&jsonl_summary(
        &data.root,
        &data.summary,
        false,
        next_hints(data),
    )) {
        records.push(record);
    }
    records.extend(
        data.entries
            .iter()
            .map(jsonl_entry)
            .filter_map(|record| serde_json::to_vec(&record).ok()),
    );
    records.extend(
        data.warnings
            .iter()
            .map(jsonl_warning)
            .filter_map(|record| serde_json::to_vec(&record).ok()),
    );
    records
}

fn jsonl_output_truncated(records: &[Vec<u8>], ctx: &RenderContext<'_>) -> bool {
    let mut bytes = 0;
    for (index, record) in records.iter().enumerate() {
        let len = record.len() + 1;
        let first = index == 0;
        if !first && index >= ctx.limits.max_records {
            return true;
        }
        if !first && bytes + len > ctx.limits.max_bytes {
            return true;
        }
        bytes += len;
        if first && (ctx.limits.max_records == 0 || bytes > ctx.limits.max_bytes) {
            return true;
        }
    }
    false
}

fn jsonl_entry(entry: &Entry) -> JsonlEntry<'_> {
    JsonlEntry {
        schema: "axt.peek.entry.v1",
        kind: entry.kind.as_str(),
        path: &entry.path,
        bytes: entry.bytes,
        lang: entry.language.as_deref(),
        git: entry.git.as_str(),
        mtime: entry.mtime.as_deref(),
    }
}

fn jsonl_warning(warning: &PeekWarning) -> JsonlWarning<'_> {
    JsonlWarning {
        schema: "axt.peek.warn.v1",
        kind: "warn",
        code: warning.code.as_str(),
        path: warning.path.as_deref(),
        reason: &warning.reason,
    }
}

fn display_path(entry: &Entry) -> String {
    if entry.kind == crate::model::EntryKind::Dir {
        format!("{}/", entry.path)
    } else {
        entry.path.clone()
    }
}

fn human_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

const fn yes_no(value: bool) -> &'static str {
    if value {
        "yes"
    } else {
        "no"
    }
}
