//! Internal use only, no stability guarantees.

#![deny(unsafe_code)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![allow(
    clippy::missing_errors_doc,
    clippy::module_name_repetitions,
    clippy::struct_excessive_bools
)]

use std::{
    collections::HashSet,
    fs::{self, File, Metadata},
    io::{self, Read},
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

use camino::{Utf8Path, Utf8PathBuf};
use ignore::{Error as IgnoreError, WalkBuilder};
use serde::{Deserialize, Serialize};
use thiserror::Error;

const SAMPLE_BYTES: usize = 8 * 1024;
const GENERATED_MARKER_BYTES: usize = 200;

/// Error type for filesystem walking and metadata extraction.
#[derive(Debug, Error)]
pub enum FsError {
    /// A filesystem path was not valid UTF-8.
    #[error("path is not valid UTF-8: {0:?}")]
    PathNotUtf8(PathBuf),

    /// A walked path was not below the configured root.
    #[error("path {path} is not below root {root}")]
    StripPrefix {
        /// Root passed to the walker.
        root: Utf8PathBuf,
        /// Path returned by the walker.
        path: Utf8PathBuf,
    },

    /// The ignore walker failed.
    #[error("failed to walk filesystem: {0}")]
    Walk(String),

    /// File metadata could not be read.
    #[error("failed to read metadata for {path}: {source}")]
    Metadata {
        /// Path whose metadata failed.
        path: Utf8PathBuf,
        /// Underlying IO error.
        source: io::Error,
    },

    /// File bytes could not be read.
    #[error("failed to read {path}: {source}")]
    Read {
        /// Path whose bytes failed.
        path: Utf8PathBuf,
        /// Underlying IO error.
        source: io::Error,
    },
}

/// Filesystem helper result type.
pub type Result<T> = std::result::Result<T, FsError>;

/// Hash algorithms supported by `axt-fs`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HashAlgorithm {
    /// BLAKE3, the only v1 hash algorithm.
    Blake3,
}

/// Options for directory walks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct WalkOptions {
    /// Maximum depth to walk below the root, where `0` returns no entries.
    pub max_depth: Option<usize>,
    /// Return only regular files.
    pub files_only: bool,
    /// Return only directories.
    pub dirs_only: bool,
    /// Include hidden paths such as dotfiles.
    pub include_hidden: bool,
    /// Disable `.ignore`, `.gitignore`, global gitignore, and standard ignore filters.
    pub no_ignore: bool,
    /// Allow traversal across filesystem boundaries.
    pub cross_fs: bool,
    /// Follow symbolic links while walking.
    pub follow_symlinks: bool,
    /// Skip regular files larger than this many bytes.
    pub max_file_size: Option<u64>,
    /// Optional hash algorithm for regular files.
    pub hash: Option<HashAlgorithm>,
}

/// Entry kind from filesystem metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntryKind {
    /// Regular file.
    File,
    /// Directory.
    Dir,
    /// Symbolic link.
    Symlink,
    /// Anything else, such as sockets or device files.
    Other,
}

/// Coarse text/binary classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContentKind {
    /// Text-like content.
    Text,
    /// Binary-like content.
    Binary,
    /// Not applicable, for non-file entries.
    NotApplicable,
}

/// Encoding guess for file content.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Encoding {
    /// UTF-8 content.
    Utf8,
    /// UTF-16 content.
    Utf16,
    /// Latin-1-like content.
    Latin1,
    /// Unknown or binary content.
    Unknown,
}

/// Newline style found in a file sample.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NewlineStyle {
    /// Only LF newlines.
    Lf,
    /// Only CRLF newlines.
    Crlf,
    /// Both LF and CRLF newlines.
    Mixed,
    /// No newlines in the inspected sample.
    None,
}

/// Per-entry filesystem metadata used by command crates.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntryMetadata {
    /// Path relative to the walk root.
    pub path: Utf8PathBuf,
    /// Filesystem entry kind.
    pub kind: EntryKind,
    /// File size in bytes from metadata.
    pub size: u64,
    /// Modification time as milliseconds since the Unix epoch, when available.
    pub mtime_unix_ms: Option<i128>,
    /// Whether the entry appears executable on this platform.
    pub is_executable: bool,
    /// Language guess, if known.
    pub language: Option<String>,
    /// MIME type guess, if known.
    pub mime: Option<String>,
    /// Text-vs-binary classification.
    pub content: ContentKind,
    /// Encoding guess.
    pub encoding: Encoding,
    /// Newline style guess.
    pub newline: NewlineStyle,
    /// Heuristic generated-file marker.
    pub generated_likely: bool,
    /// BLAKE3 hash, when requested and applicable.
    pub blake3: Option<String>,
}

/// Non-fatal filesystem warning emitted while walking.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FsWarning {
    /// Stable warning code.
    pub code: FsWarningCode,
    /// Path associated with the warning, when available.
    pub path: Option<Utf8PathBuf>,
    /// Human-readable reason.
    pub reason: String,
}

/// Warning codes from filesystem walking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FsWarningCode {
    /// A subtree could not be read due to permissions.
    PermissionDenied,
    /// A followed symlink would create a loop.
    SymlinkLoop,
    /// A path was not valid UTF-8.
    PathNotUtf8,
}

/// Metadata collection plus non-fatal warnings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MetadataCollection {
    /// Collected entries.
    pub entries: Vec<EntryMetadata>,
    /// Non-fatal warnings encountered during collection.
    pub warnings: Vec<FsWarning>,
}

/// Walk `root` and collect deterministic per-entry metadata.
pub fn collect_metadata(root: &Utf8Path, options: WalkOptions) -> Result<Vec<EntryMetadata>> {
    collect_metadata_with_warnings(root, options).map(|collection| collection.entries)
}

/// Walk `root` and collect deterministic metadata plus non-fatal warnings.
pub fn collect_metadata_with_warnings(
    root: &Utf8Path,
    options: WalkOptions,
) -> Result<MetadataCollection> {
    let mut builder = WalkBuilder::new(root.as_std_path());
    builder.hidden(!options.include_hidden);
    builder.ignore(!options.no_ignore);
    builder.git_ignore(!options.no_ignore);
    builder.git_global(!options.no_ignore);
    builder.git_exclude(!options.no_ignore);
    builder.parents(!options.no_ignore);
    builder.follow_links(options.follow_symlinks);
    builder.same_file_system(!options.cross_fs);
    builder.max_depth(options.max_depth);

    let mut seen_dirs = HashSet::new();
    let mut entries = Vec::new();
    let mut warnings = Vec::new();
    for item in builder.build() {
        let entry = match item {
            Ok(entry) => entry,
            Err(err) if handle_walk_warning(&mut warnings, &err) => continue,
            Err(err) => return Err(FsError::Walk(err.to_string())),
        };
        if entry.depth() == 0 {
            continue;
        }

        let path = match utf8_path(entry.path().to_path_buf()) {
            Ok(path) => path,
            Err(FsError::PathNotUtf8(path)) => {
                warnings.push(FsWarning {
                    code: FsWarningCode::PathNotUtf8,
                    path: None,
                    reason: format!("path is not valid UTF-8: {}", path.display()),
                });
                continue;
            }
            Err(err) => return Err(err),
        };
        let metadata = match metadata_for(&path, options.follow_symlinks) {
            Ok(metadata) => metadata,
            Err(FsError::Metadata { path, source })
                if source.kind() == io::ErrorKind::PermissionDenied =>
            {
                warnings.push(FsWarning {
                    code: FsWarningCode::PermissionDenied,
                    path: Some(relative_path_lossy(root, &path)),
                    reason: source.to_string(),
                });
                continue;
            }
            Err(err) => return Err(err),
        };
        if options.follow_symlinks
            && metadata.is_dir()
            && seen_dir_before(&mut seen_dirs, path.as_std_path(), &metadata)
        {
            warnings.push(FsWarning {
                code: FsWarningCode::SymlinkLoop,
                path: Some(relative_path_lossy(root, &path)),
                reason: "directory was already visited".to_owned(),
            });
            continue;
        }

        let kind = entry_kind(&metadata);
        if !kind_matches_options(kind, options) || exceeds_max_file_size(kind, &metadata, options) {
            continue;
        }

        entries.push(metadata_for_entry(root, &path, &metadata, options.hash)?);
    }

    entries.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(MetadataCollection { entries, warnings })
}

fn handle_walk_warning(warnings: &mut Vec<FsWarning>, err: &IgnoreError) -> bool {
    let Some(warning) = warning_from_ignore_error(err, err) else {
        return false;
    };
    warnings.push(warning);
    true
}

fn warning_from_ignore_error(err: &IgnoreError, root_err: &IgnoreError) -> Option<FsWarning> {
    match err {
        IgnoreError::Loop { child, .. } => Some(FsWarning {
            code: FsWarningCode::SymlinkLoop,
            path: Utf8PathBuf::from_path_buf(child.clone()).ok(),
            reason: root_err.to_string(),
        }),
        IgnoreError::WithPath { path, err }
            if err
                .io_error()
                .is_some_and(|io_err| io_err.kind() == io::ErrorKind::PermissionDenied) =>
        {
            Some(FsWarning {
                code: FsWarningCode::PermissionDenied,
                path: Utf8PathBuf::from_path_buf(path.clone()).ok(),
                reason: root_err.to_string(),
            })
        }
        IgnoreError::WithPath { err, .. }
        | IgnoreError::WithDepth { err, .. }
        | IgnoreError::WithLineNumber { err, .. } => warning_from_ignore_error(err, root_err),
        IgnoreError::Partial(errors) => errors
            .iter()
            .find_map(|error| warning_from_ignore_error(error, root_err)),
        _ => None,
    }
}

fn metadata_for_entry(
    root: &Utf8Path,
    path: &Utf8Path,
    metadata: &Metadata,
    hash: Option<HashAlgorithm>,
) -> Result<EntryMetadata> {
    let relative = relative_path(root, path)?;
    let kind = entry_kind(metadata);
    let sample = if kind == EntryKind::File {
        read_sample(path)?
    } else {
        Vec::new()
    };
    let content = content_kind(kind, &sample);
    let encoding = encoding_for(content, &sample);
    let generated_likely = generated_likely(&relative, &sample);

    Ok(EntryMetadata {
        path: relative,
        kind,
        size: metadata.len(),
        mtime_unix_ms: mtime_unix_ms(metadata),
        is_executable: is_executable(path, metadata),
        language: language_for(path, &sample),
        mime: mime_for(path, &sample),
        content,
        encoding,
        newline: newline_style(&sample),
        generated_likely,
        blake3: hash_for(path, kind, hash)?,
    })
}

fn kind_matches_options(kind: EntryKind, options: WalkOptions) -> bool {
    (!options.files_only || kind == EntryKind::File)
        && (!options.dirs_only || kind == EntryKind::Dir)
}

fn exceeds_max_file_size(kind: EntryKind, metadata: &Metadata, options: WalkOptions) -> bool {
    kind == EntryKind::File
        && options
            .max_file_size
            .is_some_and(|max_file_size| metadata.len() > max_file_size)
}

fn metadata_for(path: &Utf8Path, follow_symlinks: bool) -> Result<Metadata> {
    let result = if follow_symlinks {
        fs::metadata(path)
    } else {
        fs::symlink_metadata(path)
    };
    result.map_err(|source| FsError::Metadata {
        path: path.to_owned(),
        source,
    })
}

fn utf8_path(path: PathBuf) -> Result<Utf8PathBuf> {
    Utf8PathBuf::from_path_buf(path).map_err(FsError::PathNotUtf8)
}

fn relative_path(root: &Utf8Path, path: &Utf8Path) -> Result<Utf8PathBuf> {
    path.strip_prefix(root)
        .map(normalized_relative_path)
        .map_err(|_err| FsError::StripPrefix {
            root: root.to_owned(),
            path: path.to_owned(),
        })
}

fn relative_path_lossy(root: &Utf8Path, path: &Utf8Path) -> Utf8PathBuf {
    path.strip_prefix(root)
        .map_or_else(|_err| path.to_path_buf(), normalized_relative_path)
}

fn normalized_relative_path(path: &Utf8Path) -> Utf8PathBuf {
    Utf8PathBuf::from(path.as_str().replace('\\', "/"))
}

fn entry_kind(metadata: &Metadata) -> EntryKind {
    let file_type = metadata.file_type();
    if file_type.is_file() {
        EntryKind::File
    } else if file_type.is_dir() {
        EntryKind::Dir
    } else if file_type.is_symlink() {
        EntryKind::Symlink
    } else {
        EntryKind::Other
    }
}

fn mtime_unix_ms(metadata: &Metadata) -> Option<i128> {
    let modified = metadata.modified().ok()?;
    let duration = modified.duration_since(UNIX_EPOCH).ok()?;
    Some(i128::from(duration.as_secs()) * 1_000 + i128::from(duration.subsec_millis()))
}

fn read_sample(path: &Utf8Path) -> Result<Vec<u8>> {
    let mut file = File::open(path).map_err(|source| FsError::Read {
        path: path.to_owned(),
        source,
    })?;
    let mut buf = Vec::with_capacity(SAMPLE_BYTES);
    file.by_ref()
        .take(SAMPLE_BYTES as u64)
        .read_to_end(&mut buf)
        .map_err(|source| FsError::Read {
            path: path.to_owned(),
            source,
        })?;
    Ok(buf)
}

fn content_kind(kind: EntryKind, sample: &[u8]) -> ContentKind {
    if kind != EntryKind::File {
        return ContentKind::NotApplicable;
    }
    if sample_has_binary_nul(sample) || has_high_control_byte_ratio(sample) {
        ContentKind::Binary
    } else {
        ContentKind::Text
    }
}

fn sample_has_binary_nul(sample: &[u8]) -> bool {
    let has_nul = sample.contains(&0);
    has_nul && !looks_like_utf16(sample)
}

fn looks_like_utf16(sample: &[u8]) -> bool {
    sample.starts_with(&[0xFF, 0xFE])
        || sample.starts_with(&[0xFE, 0xFF])
        || sample
            .chunks_exact(2)
            .take(16)
            .filter(|chunk| chunk[1] == 0)
            .count()
            >= 4
}

fn has_high_control_byte_ratio(sample: &[u8]) -> bool {
    if sample.is_empty() {
        return false;
    }
    let control = sample
        .iter()
        .filter(|byte| matches!(byte, 0x01..=0x08 | 0x0B | 0x0C | 0x0E..=0x1F))
        .count();
    control * 100 > sample.len() * 30
}

fn encoding_for(content: ContentKind, sample: &[u8]) -> Encoding {
    if content != ContentKind::Text {
        return Encoding::Unknown;
    }
    if looks_like_utf16(sample) {
        Encoding::Utf16
    } else if std::str::from_utf8(sample).is_ok() {
        Encoding::Utf8
    } else {
        Encoding::Latin1
    }
}

fn newline_style(sample: &[u8]) -> NewlineStyle {
    let crlf = sample.windows(2).any(|window| window == b"\r\n");
    let lf = sample.iter().enumerate().any(|(index, byte)| {
        *byte == b'\n' && index.checked_sub(1).and_then(|prev| sample.get(prev)) != Some(&b'\r')
    });

    match (lf, crlf) {
        (true, true) => NewlineStyle::Mixed,
        (true, false) => NewlineStyle::Lf,
        (false, true) => NewlineStyle::Crlf,
        (false, false) => NewlineStyle::None,
    }
}

fn language_for(path: &Utf8Path, sample: &[u8]) -> Option<String> {
    path.extension()
        .and_then(language_from_extension)
        .or_else(|| language_from_infer(sample))
        .map(str::to_owned)
}

fn language_from_extension(extension: &str) -> Option<&'static str> {
    match extension.to_ascii_lowercase().as_str() {
        "c" | "h" => Some("C"),
        "cpp" | "cc" | "cxx" | "hpp" => Some("C++"),
        "css" => Some("CSS"),
        "go" => Some("Go"),
        "html" | "htm" => Some("HTML"),
        "java" => Some("Java"),
        "js" | "cjs" | "mjs" => Some("JavaScript"),
        "jsx" => Some("JSX"),
        "json" => Some("JSON"),
        "md" | "markdown" => Some("Markdown"),
        "py" => Some("Python"),
        "rs" => Some("Rust"),
        "scss" => Some("SCSS"),
        "sh" | "bash" | "zsh" => Some("Shell"),
        "toml" => Some("TOML"),
        "ts" => Some("TypeScript"),
        "tsx" => Some("TSX"),
        "txt" => Some("Text"),
        "yaml" | "yml" => Some("YAML"),
        _ => None,
    }
}

fn language_from_infer(sample: &[u8]) -> Option<&'static str> {
    let inferred = infer::get(sample)?;
    match inferred.mime_type() {
        "application/json" => Some("JSON"),
        "application/xml" | "text/xml" => Some("XML"),
        "image/svg+xml" => Some("SVG"),
        "text/plain" => Some("Text"),
        _ => None,
    }
}

fn mime_for(path: &Utf8Path, sample: &[u8]) -> Option<String> {
    mime_guess::from_path(path.as_std_path())
        .first_raw()
        .or_else(|| infer::get(sample).map(|inferred| inferred.mime_type()))
        .map(str::to_owned)
}

fn generated_likely(path: &Utf8Path, sample: &[u8]) -> bool {
    generated_path_component(path) || generated_marker(sample) || minified_javascript(path, sample)
}

fn generated_path_component(path: &Utf8Path) -> bool {
    path.components().any(|component| {
        matches!(
            component.as_str(),
            ".next" | "build" | "coverage" | "dist" | "node_modules" | "out" | "target" | "vendor"
        )
    })
}

fn generated_marker(sample: &[u8]) -> bool {
    let end = sample.len().min(GENERATED_MARKER_BYTES);
    let prefix = String::from_utf8_lossy(&sample[..end]).to_ascii_lowercase();
    prefix.contains("generated") || prefix.contains("do not edit")
}

fn minified_javascript(path: &Utf8Path, sample: &[u8]) -> bool {
    if !matches!(path.extension(), Some("js" | "mjs" | "cjs")) || sample.len() < 512 {
        return false;
    }
    let longest_line = sample
        .split(|byte| *byte == b'\n')
        .map(<[u8]>::len)
        .max()
        .unwrap_or(0);
    let whitespace = sample
        .iter()
        .filter(|byte| matches!(byte, b' ' | b'\n' | b'\r' | b'\t'))
        .count();
    longest_line > 500 && whitespace * 100 < sample.len() * 5
}

fn hash_for(
    path: &Utf8Path,
    kind: EntryKind,
    hash: Option<HashAlgorithm>,
) -> Result<Option<String>> {
    if kind != EntryKind::File || hash.is_none() {
        return Ok(None);
    }

    let mut file = File::open(path).map_err(|source| FsError::Read {
        path: path.to_owned(),
        source,
    })?;
    let mut hasher = blake3::Hasher::new();
    let mut buf = [0_u8; 8 * 1024];
    loop {
        let read = file.read(&mut buf).map_err(|source| FsError::Read {
            path: path.to_owned(),
            source,
        })?;
        if read == 0 {
            break;
        }
        hasher.update(&buf[..read]);
    }

    Ok(Some(hasher.finalize().to_hex().to_string()))
}

#[cfg(unix)]
fn is_executable(_path: &Utf8Path, metadata: &Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt;

    metadata.is_file() && metadata.permissions().mode() & 0o111 != 0
}

#[cfg(windows)]
fn is_executable(path: &Utf8Path, metadata: &Metadata) -> bool {
    metadata.is_file()
        && path.extension().is_some_and(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "bat" | "cmd" | "com" | "exe" | "ps1"
            )
        })
}

#[cfg(not(any(unix, windows)))]
fn is_executable(_path: &Utf8Path, _metadata: &Metadata) -> bool {
    false
}

#[cfg(unix)]
type FileIdentity = (u64, u64);

#[cfg(unix)]
fn seen_dir_before(
    seen_dirs: &mut HashSet<FileIdentity>,
    _path: &Path,
    metadata: &Metadata,
) -> bool {
    use std::os::unix::fs::MetadataExt;

    !seen_dirs.insert((metadata.dev(), metadata.ino()))
}

#[cfg(not(unix))]
type FileIdentity = PathBuf;

#[cfg(not(unix))]
fn seen_dir_before(
    seen_dirs: &mut HashSet<FileIdentity>,
    path: &Path,
    _metadata: &Metadata,
) -> bool {
    fs::canonicalize(path)
        .ok()
        .is_some_and(|identity| !seen_dirs.insert(identity))
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        time::{Duration, Instant},
    };

    use super::*;

    fn fixture_root() -> Utf8PathBuf {
        Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/fs-small")
    }

    fn has_path(entries: &[EntryMetadata], path: &str) -> bool {
        entries.iter().any(|entry| entry.path.as_str() == path)
    }

    #[test]
    fn default_walk_is_deterministic_fast_and_honors_ignore(
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let root = fixture_root();
        let started = Instant::now();
        let first = collect_metadata(&root, WalkOptions::default())?;
        let elapsed = started.elapsed();
        let second = collect_metadata(&root, WalkOptions::default())?;

        assert!(elapsed < Duration::from_millis(50));
        assert_eq!(serde_json::to_vec(&first)?, serde_json::to_vec(&second)?);

        assert!(has_path(&first, "README.md"));
        assert!(has_path(&first, "src/main.rs"));
        assert!(!has_path(&first, "ignored.tmp"));
        assert!(!has_path(&first, ".hidden.txt"));

        Ok(())
    }

    #[test]
    fn no_ignore_includes_ignored_files() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let root = fixture_root();
        let entries = collect_metadata(
            &root,
            WalkOptions {
                no_ignore: true,
                ..WalkOptions::default()
            },
        )?;
        assert!(has_path(&entries, "ignored.tmp"));
        assert!(!has_path(&entries, ".hidden.txt"));
        Ok(())
    }

    #[test]
    fn include_hidden_is_separate_from_ignore_handling(
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let root = fixture_root();
        let entries = collect_metadata(
            &root,
            WalkOptions {
                include_hidden: true,
                ..WalkOptions::default()
            },
        )?;

        assert!(has_path(&entries, ".hidden.txt"));
        assert!(!has_path(&entries, "ignored.tmp"));
        Ok(())
    }

    #[test]
    fn walker_options_cover_peek_filters() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let root = fixture_root();
        let shallow = collect_metadata(
            &root,
            WalkOptions {
                max_depth: Some(1),
                ..WalkOptions::default()
            },
        )?;
        assert!(has_path(&shallow, "src"));
        assert!(!has_path(&shallow, "src/main.rs"));

        let files = collect_metadata(
            &root,
            WalkOptions {
                files_only: true,
                include_hidden: true,
                no_ignore: true,
                ..WalkOptions::default()
            },
        )?;
        assert!(files.iter().all(|entry| entry.kind == EntryKind::File));

        let dirs = collect_metadata(
            &root,
            WalkOptions {
                dirs_only: true,
                include_hidden: true,
                no_ignore: true,
                ..WalkOptions::default()
            },
        )?;
        assert!(dirs.iter().all(|entry| entry.kind == EntryKind::Dir));

        let small_only = collect_metadata(
            &root,
            WalkOptions {
                max_file_size: Some(20),
                include_hidden: true,
                no_ignore: true,
                ..WalkOptions::default()
            },
        )?;
        assert!(has_path(&small_only, ".hidden.txt"));
        assert!(!has_path(&small_only, "src/main.rs"));

        Ok(())
    }

    #[test]
    fn metadata_classifies_language_generated_newlines_and_hash(
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let root = fixture_root();
        let entries = collect_metadata(
            &root,
            WalkOptions {
                hash: Some(HashAlgorithm::Blake3),
                ..WalkOptions::default()
            },
        )?;

        let rust = entry_named(&entries, "src/main.rs")?;
        assert_eq!(rust.kind, EntryKind::File);
        assert_eq!(rust.language.as_deref(), Some("Rust"));
        assert_eq!(rust.mime.as_deref(), Some("text/x-rust"));
        assert_eq!(rust.content, ContentKind::Text);
        assert_eq!(rust.encoding, Encoding::Utf8);
        let rust_fixture = include_bytes!("../../../fixtures/fs-small/src/main.rs");
        assert_eq!(rust.newline, newline_style(rust_fixture));
        assert!(!rust.generated_likely);
        assert_eq!(
            rust.blake3.as_deref(),
            Some(blake3::hash(rust_fixture).to_hex().as_str())
        );

        let generated = entry_named(&entries, "generated.txt")?;
        assert!(generated.generated_likely);

        let minified = entry_named(&entries, "dist/app.min.js")?;
        assert_eq!(minified.language.as_deref(), Some("JavaScript"));
        assert!(minified.generated_likely);

        Ok(())
    }

    #[test]
    fn generated_paths_are_evaluated_relative_to_root(
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let root = Utf8PathBuf::from_path_buf(temp.path().join("build").join("fixture"))
            .map_err(FsError::PathNotUtf8)?;
        fs::create_dir_all(root.as_std_path())?;
        fs::write(root.join("plain.txt"), "plain hand-written text\n")?;

        let entries = collect_metadata(&root, WalkOptions::default())?;
        let plain = entry_named(&entries, "plain.txt")?;

        assert!(!plain.generated_likely);
        Ok(())
    }

    fn entry_named<'a>(entries: &'a [EntryMetadata], path: &str) -> Result<&'a EntryMetadata> {
        entries
            .iter()
            .find(|entry| entry.path == path)
            .ok_or_else(|| FsError::Walk(format!("missing fixture entry {path}")))
    }
}
