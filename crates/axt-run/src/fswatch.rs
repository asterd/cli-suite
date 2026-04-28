use std::{collections::BTreeMap, fs, io::Read, time::SystemTime};

use camino::{Utf8Path, Utf8PathBuf};
use globset::{Glob, GlobSet, GlobSetBuilder};
use walkdir::WalkDir;

use crate::{
    error::{Result, RunError},
    model::{ChangeAction, FileChange},
};

#[derive(Debug, Clone)]
pub struct WatchOptions {
    pub include: Vec<String>,
    pub exclude: Vec<String>,
    pub hash: bool,
}

#[derive(Debug)]
pub struct Snapshot {
    files: BTreeMap<String, FileState>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FileState {
    bytes: u64,
    mtime_ns: Option<u128>,
    inode: Option<u64>,
    hash: Option<String>,
}

#[derive(Debug)]
struct Matchers {
    include: Option<GlobSet>,
    exclude: Option<GlobSet>,
}

impl Snapshot {
    pub fn capture(root: &Utf8Path, options: &WatchOptions) -> Result<Self> {
        let matchers = Matchers::new(&options.include, &options.exclude)?;
        let mut files = BTreeMap::new();
        for entry in WalkDir::new(root).follow_links(false) {
            let entry = entry.map_err(|err| RunError::Io {
                path: root.to_owned(),
                source: err
                    .into_io_error()
                    .unwrap_or_else(|| std::io::Error::other("failed to walk directory")),
            })?;
            if entry.depth() == 0 {
                continue;
            }
            let path = Utf8PathBuf::from_path_buf(entry.path().to_path_buf())
                .map_err(RunError::PathNotUtf8)?;
            let rel = path.strip_prefix(root).map_err(|err| RunError::Io {
                path: path.clone(),
                source: std::io::Error::other(err.to_string()),
            })?;
            if rel.as_str().starts_with(".axt/") || rel.as_str() == ".axt" {
                continue;
            }
            if !entry.file_type().is_file() || !matchers.matches(rel.as_str()) {
                continue;
            }
            let metadata = entry.metadata().map_err(|err| RunError::Io {
                path: path.clone(),
                source: err
                    .into_io_error()
                    .unwrap_or_else(|| std::io::Error::other("failed to read entry metadata")),
            })?;
            files.insert(
                rel.to_string(),
                FileState {
                    bytes: metadata.len(),
                    mtime_ns: metadata.modified().ok().and_then(system_time_ns),
                    inode: inode(&metadata),
                    hash: if options.hash {
                        Some(hash_file(&path)?)
                    } else {
                        None
                    },
                },
            );
        }
        Ok(Self { files })
    }

    #[must_use]
    pub fn diff(&self, after: &Self) -> Vec<FileChange> {
        let mut changes = Vec::new();
        for (path, before_state) in &self.files {
            match after.files.get(path) {
                Some(after_state) if before_state != after_state => changes.push(FileChange {
                    path: path.clone(),
                    action: ChangeAction::Modified,
                    bytes: Some(after_state.bytes),
                    hash: after_state.hash.clone(),
                }),
                Some(_) => {}
                None => changes.push(FileChange {
                    path: path.clone(),
                    action: ChangeAction::Deleted,
                    bytes: None,
                    hash: None,
                }),
            }
        }
        for (path, after_state) in &after.files {
            if !self.files.contains_key(path) {
                changes.push(FileChange {
                    path: path.clone(),
                    action: ChangeAction::Created,
                    bytes: Some(after_state.bytes),
                    hash: after_state.hash.clone(),
                });
            }
        }
        changes
    }
}

impl Matchers {
    fn new(include: &[String], exclude: &[String]) -> Result<Self> {
        Ok(Self {
            include: build_set(include)?.filter(|set| !set.is_empty()),
            exclude: build_set(exclude)?,
        })
    }

    fn matches(&self, path: &str) -> bool {
        let included = self
            .include
            .as_ref()
            .is_none_or(|include| include.is_match(path));
        let excluded = self
            .exclude
            .as_ref()
            .is_some_and(|exclude| exclude.is_match(path));
        included && !excluded
    }
}

fn build_set(patterns: &[String]) -> Result<Option<GlobSet>> {
    if patterns.is_empty() {
        return Ok(None);
    }
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        let glob = Glob::new(pattern).map_err(|source| RunError::Glob {
            pattern: pattern.clone(),
            source,
        })?;
        builder.add(glob);
    }
    builder.build().map(Some).map_err(|source| RunError::Glob {
        pattern: patterns.join(","),
        source,
    })
}

fn system_time_ns(value: SystemTime) -> Option<u128> {
    value
        .duration_since(SystemTime::UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_nanos())
}

fn hash_file(path: &Utf8Path) -> Result<String> {
    let mut file = fs::File::open(path).map_err(|source| RunError::Io {
        path: path.to_owned(),
        source,
    })?;
    let mut hasher = blake3::Hasher::new();
    let mut buffer = [0; 8192];
    loop {
        let read = file.read(&mut buffer).map_err(|source| RunError::Io {
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

#[cfg(unix)]
fn inode(metadata: &fs::Metadata) -> Option<u64> {
    use std::os::unix::fs::MetadataExt;
    Some(metadata.ino())
}

#[cfg(not(unix))]
fn inode(_metadata: &fs::Metadata) -> Option<u64> {
    None
}
