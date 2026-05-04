use std::{
    fs,
    io::{BufRead, BufReader, BufWriter, Read, Seek, SeekFrom, Write},
    time::SystemTime,
};

use camino::{Utf8Path, Utf8PathBuf};
use rayon::prelude::*;
use tempfile::NamedTempFile;
use walkdir::WalkDir;

use crate::{
    error::{DriftError, Result},
    model::{ChangeAction, FileChange, MarkEntry, SnapshotRecord},
};

const SORT_CHUNK_RECORDS: usize = 8192;

pub struct CapturedSnapshot {
    file: NamedTempFile,
    source: Utf8PathBuf,
    len: usize,
    hash_skipped_size: usize,
}

struct SnapshotCursor {
    reader: BufReader<fs::File>,
    source: Utf8PathBuf,
    line: usize,
    current: Option<SnapshotRecord>,
}

struct SnapshotCandidate {
    full_path: Utf8PathBuf,
    record: SnapshotRecord,
}

impl CapturedSnapshot {
    pub fn capture(root: &Utf8Path, hash: bool, hash_max_bytes: u64) -> Result<Self> {
        let mut candidates = collect_candidates(root)?;
        if hash {
            hash_candidates(&mut candidates, hash_max_bytes)?;
        }
        let hash_skipped_size = candidates
            .iter()
            .filter(|candidate| candidate.record.hash_skipped_size)
            .count();
        let records = candidates
            .into_iter()
            .map(|candidate| candidate.record)
            .collect::<Vec<_>>();
        Self::from_records(records, hash_skipped_size)
    }

    fn from_records(records: Vec<SnapshotRecord>, hash_skipped_size: usize) -> Result<Self> {
        let len = records.len();
        let mut chunks = Vec::new();
        for chunk in records.chunks(SORT_CHUNK_RECORDS) {
            chunks.push(write_sorted_chunk(chunk.to_vec())?);
        }

        let mut file = NamedTempFile::new().map_err(|source| DriftError::Io {
            path: Utf8PathBuf::from("."),
            source,
        })?;
        merge_chunks(&mut chunks, &mut file)?;
        let source = temp_path(file.path())?;
        file.as_file_mut()
            .seek(SeekFrom::Start(0))
            .map_err(|err| DriftError::Io {
                path: self_path(&source),
                source: err,
            })?;
        Ok(Self {
            file,
            source,
            len,
            hash_skipped_size,
        })
    }

    #[must_use]
    pub const fn len(&self) -> usize {
        self.len
    }

    #[must_use]
    pub const fn hash_skipped_size(&self) -> usize {
        self.hash_skipped_size
    }

    pub fn persist_to(&self, path: &Utf8Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|source| DriftError::Io {
                path: parent.to_owned(),
                source,
            })?;
        }
        let parent = path.parent().unwrap_or_else(|| Utf8Path::new("."));
        let mut output = NamedTempFile::new_in(parent).map_err(|source| DriftError::Io {
            path: parent.to_owned(),
            source,
        })?;
        let mut input = self.file.reopen().map_err(|source| DriftError::Io {
            path: self.source.clone(),
            source,
        })?;
        input
            .seek(SeekFrom::Start(0))
            .map_err(|source| DriftError::Io {
                path: self.source.clone(),
                source,
            })?;
        std::io::copy(&mut input, output.as_file_mut()).map_err(|source| DriftError::Io {
            path: path.to_owned(),
            source,
        })?;
        output
            .as_file()
            .sync_all()
            .map_err(|source| DriftError::Io {
                path: path.to_owned(),
                source,
            })?;
        output.persist(path).map_err(|err| DriftError::Io {
            path: path.to_owned(),
            source: err.error,
        })?;
        sync_parent_dir(parent)
    }

    fn cursor(&self) -> Result<SnapshotCursor> {
        let file = self.file.reopen().map_err(|source| DriftError::Io {
            path: self.source.clone(),
            source,
        })?;
        SnapshotCursor::from_file(file, self.source.clone())
    }
}

impl SnapshotCursor {
    fn from_path(path: &Utf8Path) -> Result<Self> {
        let file = fs::File::open(path).map_err(|source| DriftError::Io {
            path: path.to_owned(),
            source,
        })?;
        Self::from_file(file, path.to_owned())
    }

    fn from_file(file: fs::File, source: Utf8PathBuf) -> Result<Self> {
        let mut cursor = Self {
            reader: BufReader::new(file),
            source,
            line: 0,
            current: None,
        };
        cursor.advance()?;
        Ok(cursor)
    }

    fn advance(&mut self) -> Result<()> {
        self.current = read_next_record(&mut self.reader, &self.source, &mut self.line)?;
        Ok(())
    }
}

pub fn diff_snapshot_files(
    before_path: &Utf8Path,
    after: &CapturedSnapshot,
) -> Result<Vec<FileChange>> {
    let mut before = SnapshotCursor::from_path(before_path)?;
    let mut current = after.cursor()?;
    diff_cursors(&mut before, &mut current)
}

pub fn diff_captured_snapshots(
    before: &CapturedSnapshot,
    after: &CapturedSnapshot,
) -> Result<Vec<FileChange>> {
    let mut before = before.cursor()?;
    let mut current = after.cursor()?;
    diff_cursors(&mut before, &mut current)
}

fn diff_cursors(
    before: &mut SnapshotCursor,
    current: &mut SnapshotCursor,
) -> Result<Vec<FileChange>> {
    let mut changes = Vec::new();
    while before.current.is_some() || current.current.is_some() {
        match (before.current.as_ref(), current.current.as_ref()) {
            (Some(left), Some(right)) if left.path == right.path => {
                if record_changed(left, right) {
                    changes.push(modified_change(left, right));
                }
                before.advance()?;
                current.advance()?;
            }
            (Some(left), Some(right)) if left.path < right.path => {
                changes.push(deleted_change(left));
                before.advance()?;
            }
            (Some(_left), Some(right)) => {
                changes.push(created_change(right));
                current.advance()?;
            }
            (Some(left), None) => {
                changes.push(deleted_change(left));
                before.advance()?;
            }
            (None, Some(right)) => {
                changes.push(created_change(right));
                current.advance()?;
            }
            (None, None) => break,
        }
    }
    sort_changes(&mut changes);
    Ok(changes)
}

fn collect_candidates(root: &Utf8Path) -> Result<Vec<SnapshotCandidate>> {
    let mut candidates = Vec::new();
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
        let snapshot_path = relative_snapshot_path(rel);
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
        candidates.push(SnapshotCandidate {
            full_path: path,
            record: SnapshotRecord {
                path: snapshot_path,
                size: metadata.len(),
                mtime_ns: metadata.modified().ok().and_then(system_time_ns),
                hash: None,
                hash_skipped_size: false,
            },
        });
    }
    Ok(candidates)
}

fn hash_candidates(candidates: &mut [SnapshotCandidate], hash_max_bytes: u64) -> Result<()> {
    candidates
        .par_iter_mut()
        .map(|candidate| {
            if candidate.record.size > hash_max_bytes {
                candidate.record.hash = None;
                candidate.record.hash_skipped_size = true;
                Ok(())
            } else {
                candidate.record.hash = Some(hash_file(&candidate.full_path)?);
                Ok(())
            }
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(())
}

fn write_sorted_chunk(mut records: Vec<SnapshotRecord>) -> Result<NamedTempFile> {
    records.sort_by(|left, right| left.path.cmp(&right.path));
    let mut file = NamedTempFile::new().map_err(|source| DriftError::Io {
        path: Utf8PathBuf::from("."),
        source,
    })?;
    write_records_to_file(&mut file, records.iter())?;
    let source = temp_path(file.path())?;
    file.as_file_mut()
        .seek(SeekFrom::Start(0))
        .map_err(|err| DriftError::Io {
            path: self_path(&source),
            source: err,
        })?;
    Ok(file)
}

fn merge_chunks(chunks: &mut [NamedTempFile], output: &mut NamedTempFile) -> Result<()> {
    let mut cursors = chunks
        .iter_mut()
        .map(|chunk| {
            let source = temp_path(chunk.path())?;
            let file = chunk.reopen().map_err(|source_err| DriftError::Io {
                path: source.clone(),
                source: source_err,
            })?;
            SnapshotCursor::from_file(file, source)
        })
        .collect::<Result<Vec<_>>>()?;

    {
        let output_path = temp_path(output.path()).unwrap_or_else(|_| Utf8PathBuf::from("."));
        let mut writer = BufWriter::new(output.as_file_mut());
        loop {
            let next = cursors
                .iter()
                .enumerate()
                .filter_map(|(index, cursor)| {
                    cursor
                        .current
                        .as_ref()
                        .map(|record| (index, record.path.as_str()))
                })
                .min_by(|left, right| left.1.cmp(right.1))
                .map(|(index, _path)| index);
            let Some(index) = next else {
                break;
            };
            if let Some(record) = cursors[index].current.as_ref() {
                serde_json::to_writer(&mut writer, record)?;
                writeln!(writer).map_err(|source| DriftError::Io {
                    path: output_path.clone(),
                    source,
                })?;
            }
            cursors[index].advance()?;
        }
        writer.flush().map_err(|source| DriftError::Io {
            path: output_path,
            source,
        })?;
    }
    let output_path = temp_path(output.path())?;
    output
        .as_file()
        .sync_all()
        .map_err(|source| DriftError::Io {
            path: output_path,
            source,
        })
}

fn write_records_to_file<'a>(
    file: &mut NamedTempFile,
    records: impl Iterator<Item = &'a SnapshotRecord>,
) -> Result<()> {
    {
        let file_path = temp_path(file.path()).unwrap_or_else(|_| Utf8PathBuf::from("."));
        let mut writer = BufWriter::new(file.as_file_mut());
        for record in records {
            serde_json::to_writer(&mut writer, record)?;
            writeln!(writer).map_err(|source| DriftError::Io {
                path: file_path.clone(),
                source,
            })?;
        }
        writer.flush().map_err(|source| DriftError::Io {
            path: file_path,
            source,
        })?;
    }
    let path = temp_path(file.path())?;
    file.as_file()
        .sync_all()
        .map_err(|source| DriftError::Io { path, source })
}

fn read_next_record(
    reader: &mut impl BufRead,
    source: &Utf8Path,
    line: &mut usize,
) -> Result<Option<SnapshotRecord>> {
    let mut text = String::new();
    loop {
        text.clear();
        let bytes = reader
            .read_line(&mut text)
            .map_err(|source_err| DriftError::Io {
                path: source.to_owned(),
                source: source_err,
            })?;
        if bytes == 0 {
            return Ok(None);
        }
        *line += 1;
        if text.trim().is_empty() {
            continue;
        }
        let record =
            serde_json::from_str(&text).map_err(|source_err| DriftError::SnapshotParse {
                path: source.to_owned(),
                line: *line,
                source: source_err,
            })?;
        return Ok(Some(record));
    }
}

fn temp_path(path: &std::path::Path) -> Result<Utf8PathBuf> {
    Utf8PathBuf::from_path_buf(path.to_path_buf()).map_err(DriftError::PathNotUtf8)
}

fn self_path(path: &Utf8Path) -> Utf8PathBuf {
    path.to_owned()
}

pub fn count_snapshot_records(path: &Utf8Path) -> Result<usize> {
    let mut cursor = SnapshotCursor::from_path(path)?;
    let mut count = 0;
    while cursor.current.is_some() {
        count += 1;
        cursor.advance()?;
    }
    Ok(count)
}

#[cfg(unix)]
fn sync_parent_dir(parent: &Utf8Path) -> Result<()> {
    fs::File::open(parent)
        .and_then(|dir| dir.sync_all())
        .map_err(|source| DriftError::Io {
            path: parent.to_owned(),
            source,
        })
}

#[cfg(not(unix))]
fn sync_parent_dir(_parent: &Utf8Path) -> Result<()> {
    Ok(())
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
        let files = count_snapshot_records(&path)?;
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

fn modified_change(before: &SnapshotRecord, current: &SnapshotRecord) -> FileChange {
    FileChange {
        path: before.path.clone(),
        action: ChangeAction::Modified,
        size_before: Some(before.size),
        size_after: Some(current.size),
        size_delta: size_delta(Some(before.size), Some(current.size)),
        hash: current.hash.clone(),
        hash_skipped_size: current.hash_skipped_size,
    }
}

fn deleted_change(before: &SnapshotRecord) -> FileChange {
    FileChange {
        path: before.path.clone(),
        action: ChangeAction::Deleted,
        size_before: Some(before.size),
        size_after: None,
        size_delta: size_delta(Some(before.size), None),
        hash: None,
        hash_skipped_size: before.hash_skipped_size,
    }
}

fn created_change(current: &SnapshotRecord) -> FileChange {
    FileChange {
        path: current.path.clone(),
        action: ChangeAction::Created,
        size_before: None,
        size_after: Some(current.size),
        size_delta: size_delta(None, Some(current.size)),
        hash: current.hash.clone(),
        hash_skipped_size: current.hash_skipped_size,
    }
}

fn sort_changes(changes: &mut [FileChange]) {
    changes.sort_by(|left, right| {
        right
            .size_delta
            .abs()
            .cmp(&left.size_delta.abs())
            .then_with(|| left.path.cmp(&right.path))
    });
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

fn relative_snapshot_path(path: &Utf8Path) -> String {
    path.as_str().replace('\\', "/")
}

fn record_changed(before: &SnapshotRecord, after: &SnapshotRecord) -> bool {
    before.size != after.size
        || before.mtime_ns != after.mtime_ns
        || match (&before.hash, &after.hash) {
            (Some(before_hash), Some(after_hash)) => before_hash != after_hash,
            (None, None) => before.hash_skipped_size != after.hash_skipped_size,
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
