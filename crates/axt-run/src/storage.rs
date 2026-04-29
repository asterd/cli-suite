use std::{fs, time::Duration};

use camino::{Utf8Path, Utf8PathBuf};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

use crate::{
    error::{Result, RunError},
    model::{RunData, StoredRun},
};

pub const RUNS_DIR: &str = ".axt/runs";

#[derive(Debug, Clone)]
pub struct RunPaths {
    pub name: String,
    pub dir: Utf8PathBuf,
    pub stdout: Utf8PathBuf,
    pub stderr: Utf8PathBuf,
    pub meta: Utf8PathBuf,
    pub changed: Utf8PathBuf,
    pub summary: Utf8PathBuf,
}

pub fn prepare(root: &Utf8Path, requested: Option<&str>, command: &[String]) -> Result<RunPaths> {
    let name = requested.map_or_else(|| default_name(command), sanitize_name);
    let dir = root.join(RUNS_DIR).join(&name);
    fs::create_dir_all(&dir).map_err(|source| RunError::Io {
        path: dir.clone(),
        source,
    })?;
    Ok(RunPaths {
        name,
        stdout: dir.join("stdout.log"),
        stderr: dir.join("stderr.log"),
        meta: dir.join("meta.json"),
        changed: dir.join("changed.json"),
        summary: dir.join("summary.agent.jsonl"),
        dir,
    })
}

pub fn write_artifacts(paths: &RunPaths, data: &RunData, agent_summary: &str) -> Result<()> {
    let created_at = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .map_err(|err| RunError::Io {
            path: paths.meta.clone(),
            source: std::io::Error::other(err.to_string()),
        })?;
    let stored = StoredRun {
        schema: "axt.run.meta.v1".to_owned(),
        created_at,
        data: data.clone(),
    };
    write_json(&paths.meta, &stored)?;
    write_json(&paths.changed, &data.changed)?;
    fs::write(&paths.summary, agent_summary).map_err(|source| RunError::Io {
        path: paths.summary.clone(),
        source,
    })?;
    Ok(())
}

pub fn read_saved(root: &Utf8Path, name: &str) -> Result<(Utf8PathBuf, StoredRun)> {
    let dir = resolve_run_dir(root, name)?;
    let bytes = fs::read(dir.join("meta.json")).map_err(|source| RunError::Io {
        path: dir.join("meta.json"),
        source,
    })?;
    let run = serde_json::from_slice(&bytes)?;
    Ok((dir, run))
}

pub fn list_saved(root: &Utf8Path) -> Result<Vec<(Utf8PathBuf, StoredRun)>> {
    let runs_dir = root.join(RUNS_DIR);
    if !runs_dir.exists() {
        return Ok(Vec::new());
    }
    let mut runs = Vec::new();
    for entry in fs::read_dir(&runs_dir).map_err(|source| RunError::Io {
        path: runs_dir.clone(),
        source,
    })? {
        let entry = entry.map_err(|source| RunError::Io {
            path: runs_dir.clone(),
            source,
        })?;
        let dir = Utf8PathBuf::from_path_buf(entry.path()).map_err(RunError::PathNotUtf8)?;
        if dir.join("meta.json").is_file() {
            let bytes = fs::read(dir.join("meta.json")).map_err(|source| RunError::Io {
                path: dir.join("meta.json"),
                source,
            })?;
            let run: StoredRun = serde_json::from_slice(&bytes)?;
            runs.push((dir, run));
        }
    }
    runs.sort_by(|left, right| left.1.created_at.cmp(&right.1.created_at));
    Ok(runs)
}

pub fn clean(root: &Utf8Path, older_than: Option<Duration>) -> Result<usize> {
    let cutoff = older_than
        .unwrap_or_else(|| config_retention(root).unwrap_or(Duration::from_secs(30 * 86_400)));
    let runs_dir = root.join(RUNS_DIR);
    if !runs_dir.exists() {
        return Ok(0);
    }
    let now = std::time::SystemTime::now();
    let mut removed = 0;
    for entry in fs::read_dir(&runs_dir).map_err(|source| RunError::Io {
        path: runs_dir.clone(),
        source,
    })? {
        let entry = entry.map_err(|source| RunError::Io {
            path: runs_dir.clone(),
            source,
        })?;
        let path = Utf8PathBuf::from_path_buf(entry.path()).map_err(RunError::PathNotUtf8)?;
        let metadata = fs::metadata(&path).map_err(|source| RunError::Io {
            path: path.clone(),
            source,
        })?;
        let Ok(age) = now.duration_since(metadata.modified().unwrap_or(now)) else {
            continue;
        };
        if age >= cutoff {
            fs::remove_dir_all(&path).map_err(|source| RunError::Io {
                path: path.clone(),
                source,
            })?;
            removed += 1;
        }
    }
    Ok(removed)
}

fn resolve_run_dir(root: &Utf8Path, name: &str) -> Result<Utf8PathBuf> {
    if name != "last" {
        let dir = root.join(RUNS_DIR).join(sanitize_name(name));
        if dir.join("meta.json").is_file() {
            return Ok(dir);
        }
        return Err(RunError::RunNotFound(name.to_owned()));
    }
    let Some((dir, _run)) = list_saved(root)?.pop() else {
        return Err(RunError::RunNotFound(name.to_owned()));
    };
    Ok(dir)
}

fn write_json<T: serde::Serialize>(path: &Utf8Path, value: &T) -> Result<()> {
    let mut bytes = serde_json::to_vec_pretty(value)?;
    bytes.push(b'\n');
    fs::write(path, bytes).map_err(|source| RunError::Io {
        path: path.to_owned(),
        source,
    })
}

fn default_name(command: &[String]) -> String {
    let now = OffsetDateTime::now_utc();
    let stamp = format!(
        "{:04}-{:02}-{:02}T{:02}-{:02}-{:02}Z",
        now.year(),
        u8::from(now.month()),
        now.day(),
        now.hour(),
        now.minute(),
        now.second()
    );
    let slug = command
        .iter()
        .take(2)
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join("-");
    format!("{stamp}-{}", sanitize_name(&slug))
}

fn sanitize_name(value: &str) -> String {
    let sanitized = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
                ch
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_owned();
    if sanitized.is_empty() {
        "run".to_owned()
    } else {
        sanitized
    }
}

fn config_retention(root: &Utf8Path) -> Option<Duration> {
    let text = fs::read_to_string(root.join(".axt/config.toml")).ok()?;
    for line in text.lines() {
        let line = line.trim();
        if let Some(value) = line.strip_prefix("retention_days") {
            let days = value.trim().strip_prefix('=')?.trim().parse::<u64>().ok()?;
            return Some(Duration::from_secs(days.saturating_mul(86_400)));
        }
    }
    None
}
