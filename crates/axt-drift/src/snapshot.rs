use std::{
    collections::BTreeMap,
    fs,
    io::{BufRead, BufReader, Read, Write},
    time::SystemTime,
};

use camino::{Utf8Path, Utf8PathBuf};
use walkdir::WalkDir;

use crate::{
    error::{DriftError, Result},
    model::{ChangeAction, FileChange, MarkEntry, SnapshotRecord},
};

#[derive(Debug, Clone)]
pub struct Snapshot {
    records: BTreeMap<String, SnapshotRecord>,
}

impl Snapshot {
    pub fn capture(root: &Utf8Path, hash: bool) -> Result<Self> {
        let mut records = BTreeMap::new();
        for entry in WalkDir::new(root).follow_links(false) {
            let entry = entry.map_err(|err| DriftError::Io {
                path: root.to_owned(),
                source: err
                    .into_io_error()
                    .unwrap_or_else(|| std::io::Error::other("failed to walk directory")),
            })?;
            if entry.depth() == 0 {
                continue;
            }
            let path = Utf8PathBuf::from_path_buf(entry.path().to_path_buf())
                .map_err(DriftError::PathNotUtf8)?;
            let rel = path.strip_prefix(root).map_err(|err| DriftError::Io {
                path: path.clone(),
                source: std::io::Error::other(err.to_string()),
            })?;
            if is_internal_axt_path(rel) {
                continue;
            }
            if !entry.file_type().is_file() {
                continue;
            }
            let metadata = entry.metadata().map_err(|err| DriftError::Io {
                path: path.clone(),
                source: err
                    .into_io_error()
                    .unwrap_or_else(|| std::io::Error::other("failed to read entry metadata")),
            })?;
            let record = SnapshotRecord {
                path: rel.to_string(),
                size: metadata.len(),
                mtime_ns: metadata.modified().ok().and_then(system_time_ns),
                hash: if hash { Some(hash_file(&path)?) } else { None },
            };
            records.insert(record.path.clone(), record);
        }
        Ok(Self { records })
    }

    pub fn read(path: &Utf8Path) -> Result<Self> {
        let file = fs::File::open(path).map_err(|source| DriftError::Io {
            path: path.to_owned(),
            source,
        })?;
        let mut records = BTreeMap::new();
        for (index, line) in BufReader::new(file).lines().enumerate() {
            let line = line.map_err(|source| DriftError::Io {
                path: path.to_owned(),
                source,
            })?;
            if line.trim().is_empty() {
                continue;
            }
            let record: SnapshotRecord =
                serde_json::from_str(&line).map_err(|source| DriftError::SnapshotParse {
                    path: path.to_owned(),
                    line: index + 1,
                    source,
                })?;
            records.insert(record.path.clone(), record);
        }
        Ok(Self { records })
    }

    pub fn write(&self, path: &Utf8Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|source| DriftError::Io {
                path: parent.to_owned(),
                source,
            })?;
        }
        let mut file = fs::File::create(path).map_err(|source| DriftError::Io {
            path: path.to_owned(),
            source,
        })?;
        for record in self.records.values() {
            serde_json::to_writer(&mut file, record)?;
            writeln!(file).map_err(|source| DriftError::Io {
                path: path.to_owned(),
                source,
            })?;
        }
        Ok(())
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.records.len()
    }

    #[must_use]
    pub fn diff(&self, after: &Self) -> Vec<FileChange> {
        let mut changes = Vec::new();
        for (path, before) in &self.records {
            match after.records.get(path) {
                Some(current) if record_changed(before, current) => changes.push(FileChange {
                    path: path.clone(),
                    action: ChangeAction::Modified,
                    size_before: Some(before.size),
                    size_after: Some(current.size),
                    size_delta: size_delta(Some(before.size), Some(current.size)),
                    hash: current.hash.clone(),
                }),
                Some(_) => {}
                None => changes.push(FileChange {
                    path: path.clone(),
                    action: ChangeAction::Deleted,
                    size_before: Some(before.size),
                    size_after: None,
                    size_delta: size_delta(Some(before.size), None),
                    hash: None,
                }),
            }
        }
        for (path, current) in &after.records {
            if !self.records.contains_key(path) {
                changes.push(FileChange {
                    path: path.clone(),
                    action: ChangeAction::Created,
                    size_before: None,
                    size_after: Some(current.size),
                    size_delta: size_delta(None, Some(current.size)),
                    hash: current.hash.clone(),
                });
            }
        }
        changes.sort_by(|left, right| {
            right
                .size_delta
                .abs()
                .cmp(&left.size_delta.abs())
                .then_with(|| left.path.cmp(&right.path))
        });
        changes
    }
}

pub fn mark_path(root: &Utf8Path, name: &str) -> Result<Utf8PathBuf> {
    validate_name(name)?;
    Ok(root
        .join(".axt")
        .join("drift")
        .join(format!("{name}.jsonl")))
}

pub fn list_marks(root: &Utf8Path) -> Result<Vec<MarkEntry>> {
    let dir = root.join(".axt").join("drift");
    let entries = match fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(source) => return Err(DriftError::Io { path: dir, source }),
    };
    let mut marks = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|source| DriftError::Io {
            path: dir.clone(),
            source,
        })?;
        let path = Utf8PathBuf::from_path_buf(entry.path()).map_err(DriftError::PathNotUtf8)?;
        if path.extension() != Some("jsonl") {
            continue;
        }
        let Some(stem) = path.file_stem() else {
            continue;
        };
        let files = Snapshot::read(&path)?.len();
        marks.push(MarkEntry {
            name: stem.to_owned(),
            path: path.to_string(),
            files,
        });
    }
    marks.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(marks)
}

pub fn reset_marks(root: &Utf8Path) -> Result<usize> {
    let dir = root.join(".axt").join("drift");
    let marks = list_marks(root)?;
    if marks.is_empty() {
        return Ok(0);
    }
    fs::remove_dir_all(&dir).map_err(|source| DriftError::Io { path: dir, source })?;
    Ok(marks.len())
}

fn validate_name(name: &str) -> Result<()> {
    let valid = !name.is_empty()
        && name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'));
    if valid {
        Ok(())
    } else {
        Err(DriftError::InvalidName(name.to_owned()))
    }
}

fn size_delta(before: Option<u64>, after: Option<u64>) -> i64 {
    let before = before
        .and_then(|value| i64::try_from(value).ok())
        .unwrap_or(0);
    let after = after
        .and_then(|value| i64::try_from(value).ok())
        .unwrap_or(0);
    after.saturating_sub(before)
}

fn is_internal_axt_path(path: &Utf8Path) -> bool {
    path.components()
        .next()
        .is_some_and(|component| component.as_str() == ".axt")
}

fn record_changed(before: &SnapshotRecord, after: &SnapshotRecord) -> bool {
    before.size != after.size
        || before.mtime_ns != after.mtime_ns
        || match (&before.hash, &after.hash) {
            (Some(before_hash), Some(after_hash)) => before_hash != after_hash,
            _ => false,
        }
}

fn system_time_ns(value: SystemTime) -> Option<u128> {
    value
        .duration_since(SystemTime::UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_nanos())
}

fn hash_file(path: &Utf8Path) -> Result<String> {
    let mut file = fs::File::open(path).map_err(|source| DriftError::Io {
        path: path.to_owned(),
        source,
    })?;
    let mut hasher = blake3::Hasher::new();
    let mut buffer = [0; 8192];
    loop {
        let read = file.read(&mut buffer).map_err(|source| DriftError::Io {
            path: path.to_owned(),
            source,
        })?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hasher.finalize().to_hex().to_string())
}
