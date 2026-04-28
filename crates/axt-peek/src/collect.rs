use std::collections::BTreeSet;

use axt_fs::{EntryMetadata, HashAlgorithm, WalkOptions};
use camino::{Utf8Path, Utf8PathBuf};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

use crate::{
    cli::{Args, HashMode, SortKey, TypeFilter},
    error::{PeekError, Result},
    model::{
        Encoding, Entry, EntryKind, GitState, GitStatus, NewlineStyle, PeekData, PeekWarning,
        Summary, WarningCode,
    },
};

pub fn collect(args: &Args, cwd: &Utf8Path) -> Result<PeekData> {
    let mut entries = Vec::new();
    let mut ignored = 0;
    let mut warnings = Vec::new();
    let roots = normalize_roots(&args.paths, cwd)?;

    for root in &roots {
        let mut root_result = collect_root(root, args)?;
        ignored += ignored_count(root, args)?;
        entries.append(&mut root_result.entries);
        warnings.append(&mut root_result.warnings);
    }

    apply_filters(&mut entries, args);
    sort_entries(&mut entries, args.sort, args.reverse);

    let summary = summarize(&entries, ignored, args.git_enabled());
    Ok(PeekData {
        root: root_label(&roots, cwd),
        summary,
        entries,
        warnings,
    })
}

fn normalize_roots(paths: &[Utf8PathBuf], cwd: &Utf8Path) -> Result<Vec<Utf8PathBuf>> {
    paths
        .iter()
        .map(|path| {
            let joined = if path.is_absolute() {
                path.clone()
            } else {
                cwd.join(path)
            };
            if !joined.exists() {
                return Err(PeekError::PathNotFound(path.clone()));
            }
            let canonical =
                dunce::canonicalize(&joined).map_err(|source| PeekError::Canonicalize {
                    path: joined.clone(),
                    source,
                })?;
            Utf8PathBuf::from_path_buf(canonical).map_err(PeekError::CanonicalPathNotUtf8)
        })
        .collect()
}

struct RootCollection {
    entries: Vec<Entry>,
    warnings: Vec<PeekWarning>,
}

fn collect_root(root: &Utf8Path, args: &Args) -> Result<RootCollection> {
    let metadata = axt_fs::collect_metadata_with_warnings(root, walk_options(args))?;
    let git = if args.git_enabled() {
        GitContext::for_root(root)?
    } else {
        GitContext::disabled()
    };
    let changed_since = changed_since_paths(&git, args.changed_since.as_deref())?;
    let prefix = output_prefix(root);

    let entries = metadata
        .entries
        .into_iter()
        .filter(|entry| changed_since_matches(entry, changed_since.as_ref()))
        .map(|entry| entry_from_metadata(root, &prefix, entry, &git))
        .collect::<Result<Vec<_>>>()?;
    let warnings = metadata
        .warnings
        .into_iter()
        .map(peek_warning_from_fs)
        .collect();

    Ok(RootCollection { entries, warnings })
}

fn walk_options(args: &Args) -> WalkOptions {
    WalkOptions {
        max_depth: Some(args.depth),
        files_only: args.files_only,
        dirs_only: args.dirs_only,
        include_hidden: args.include_hidden,
        no_ignore: args.no_ignore,
        cross_fs: args.cross_fs,
        follow_symlinks: args.follow_symlinks,
        max_file_size: args.max_file_size,
        hash: match args.hash {
            HashMode::None => None,
            HashMode::Blake3 => Some(HashAlgorithm::Blake3),
        },
    }
}

fn ignored_count(root: &Utf8Path, args: &Args) -> Result<usize> {
    if args.no_ignore {
        return Ok(0);
    }
    let visible = axt_fs::collect_metadata(root, walk_options(args))?
        .into_iter()
        .map(|entry| entry.path)
        .collect::<BTreeSet<_>>();
    let all = axt_fs::collect_metadata(
        root,
        WalkOptions {
            no_ignore: true,
            ..walk_options(args)
        },
    )?
    .into_iter()
    .map(|entry| entry.path)
    .collect::<BTreeSet<_>>();
    Ok(all.difference(&visible).count())
}

fn peek_warning_from_fs(warning: axt_fs::FsWarning) -> PeekWarning {
    let code = match warning.code {
        axt_fs::FsWarningCode::PermissionDenied => WarningCode::PermissionDenied,
        axt_fs::FsWarningCode::SymlinkLoop => WarningCode::SymlinkLoop,
        axt_fs::FsWarningCode::PathNotUtf8 => WarningCode::PathNotUtf8,
    };
    PeekWarning {
        code,
        path: warning.path.map(|path| path.to_string()),
        reason: warning.reason,
    }
}

fn changed_since_paths(
    git: &GitContext,
    changed_since: Option<&str>,
) -> Result<Option<BTreeSet<Utf8PathBuf>>> {
    let Some(reference) = changed_since else {
        return Ok(None);
    };
    let Some(repo) = git.repo.as_ref() else {
        return Ok(Some(BTreeSet::new()));
    };
    Ok(Some(
        axt_git::diff_paths(repo, reference, "HEAD")?
            .into_iter()
            .collect(),
    ))
}

fn changed_since_matches(
    entry: &EntryMetadata,
    changed_since: Option<&BTreeSet<Utf8PathBuf>>,
) -> bool {
    changed_since.is_none_or(|paths| paths.contains(&entry.path))
}

fn entry_from_metadata(
    root: &Utf8Path,
    prefix: &str,
    metadata: EntryMetadata,
    git: &GitContext,
) -> Result<Entry> {
    let git_status = if metadata.kind == axt_fs::EntryKind::Dir
        && root.join(&metadata.path).join(".git").exists()
    {
        GitStatus::Mixed
    } else {
        git.status_for(root, &metadata.path)
    };
    let path = if prefix.is_empty() {
        metadata.path.to_string()
    } else {
        format!("{prefix}/{}", metadata.path)
    };

    Ok(Entry {
        path,
        kind: map_entry_kind(metadata.kind),
        bytes: if metadata.kind == axt_fs::EntryKind::File {
            metadata.size
        } else {
            0
        },
        language: metadata.language.map(|value| value.to_ascii_lowercase()),
        mime: metadata.mime,
        encoding: (metadata.kind == axt_fs::EntryKind::File)
            .then_some(map_encoding(metadata.encoding)),
        newline: (metadata.kind == axt_fs::EntryKind::File)
            .then_some(map_newline(metadata.newline)),
        is_generated: metadata.generated_likely,
        git: git_status,
        mtime: format_mtime(metadata.mtime_unix_ms)?,
        hash: metadata.blake3,
    })
}

fn apply_filters(entries: &mut Vec<Entry>, args: &Args) {
    entries.retain(|entry| {
        let changed = !args.changed || !matches!(entry.git, GitStatus::Clean | GitStatus::None);
        let lang = args
            .lang
            .as_ref()
            .is_none_or(|lang| entry.language.as_deref() == Some(lang.as_str()));
        let kind = args
            .type_filter
            .is_none_or(|filter| entry_matches_type(entry, filter));
        changed && lang && kind
    });
}

fn entry_matches_type(entry: &Entry, filter: TypeFilter) -> bool {
    match filter {
        TypeFilter::Text => entry
            .encoding
            .is_some_and(|encoding| encoding != Encoding::Unknown),
        TypeFilter::Binary => entry.encoding == Some(Encoding::Unknown),
        TypeFilter::Image => entry
            .mime
            .as_deref()
            .is_some_and(|mime| mime.starts_with("image/")),
        TypeFilter::Archive => entry.mime.as_deref().is_some_and(is_archive_mime),
        TypeFilter::Code => entry.language.is_some() && entry.kind == EntryKind::File,
        TypeFilter::Config => {
            entry.path.ends_with(".toml")
                || entry.path.ends_with(".yaml")
                || entry.path.ends_with(".yml")
                || entry.path.ends_with(".json")
        }
        TypeFilter::Data => {
            entry.path.ends_with(".csv")
                || entry.path.ends_with(".json")
                || entry.path.ends_with(".xml")
        }
    }
}

fn is_archive_mime(mime: &str) -> bool {
    matches!(
        mime,
        "application/zip"
            | "application/x-tar"
            | "application/gzip"
            | "application/x-7z-compressed"
    )
}

fn sort_entries(entries: &mut [Entry], sort: SortKey, reverse: bool) {
    entries.sort_by(|left, right| {
        let ordering = match sort {
            SortKey::Name => left.path.cmp(&right.path),
            SortKey::Size => left
                .bytes
                .cmp(&right.bytes)
                .then(left.path.cmp(&right.path)),
            SortKey::Mtime => left
                .mtime
                .cmp(&right.mtime)
                .then(left.path.cmp(&right.path)),
            SortKey::Git => left.git.cmp(&right.git).then(left.path.cmp(&right.path)),
            SortKey::Type => left.kind.cmp(&right.kind).then(left.path.cmp(&right.path)),
        };
        if reverse {
            ordering.reverse()
        } else {
            ordering
        }
    });
}

fn summarize(entries: &[Entry], ignored: usize, git_enabled: bool) -> Summary {
    let files = entries
        .iter()
        .filter(|entry| entry.kind == EntryKind::File)
        .count();
    let dirs = entries
        .iter()
        .filter(|entry| entry.kind == EntryKind::Dir)
        .count();
    let bytes = entries
        .iter()
        .filter(|entry| entry.kind == EntryKind::File)
        .map(|entry| entry.bytes)
        .sum();
    let modified = entries
        .iter()
        .filter(|entry| {
            matches!(
                entry.git,
                GitStatus::Modified
                    | GitStatus::Added
                    | GitStatus::Deleted
                    | GitStatus::Renamed
                    | GitStatus::Mixed
            )
        })
        .count();
    let untracked = entries
        .iter()
        .filter(|entry| entry.git == GitStatus::Untracked)
        .count();
    let git_state = if !git_enabled || entries.iter().any(|entry| entry.git == GitStatus::None) {
        GitState::None
    } else if modified > 0 || untracked > 0 {
        GitState::Dirty
    } else {
        GitState::Clean
    };

    Summary {
        files,
        dirs,
        bytes,
        git_state,
        modified,
        untracked,
        ignored,
        truncated: false,
    }
}

fn format_mtime(mtime_unix_ms: Option<i128>) -> Result<Option<String>> {
    let Some(ms) = mtime_unix_ms else {
        return Ok(None);
    };
    let nanos = ms.saturating_mul(1_000_000);
    let ts = OffsetDateTime::from_unix_timestamp_nanos(nanos)?;
    Ok(Some(ts.format(&Rfc3339)?))
}

fn map_entry_kind(kind: axt_fs::EntryKind) -> EntryKind {
    match kind {
        axt_fs::EntryKind::File => EntryKind::File,
        axt_fs::EntryKind::Dir => EntryKind::Dir,
        axt_fs::EntryKind::Symlink => EntryKind::Symlink,
        axt_fs::EntryKind::Other => EntryKind::Other,
    }
}

fn map_encoding(encoding: axt_fs::Encoding) -> Encoding {
    match encoding {
        axt_fs::Encoding::Utf8 => Encoding::Utf8,
        axt_fs::Encoding::Utf16 => Encoding::Utf16,
        axt_fs::Encoding::Latin1 => Encoding::Latin1,
        axt_fs::Encoding::Unknown => Encoding::Unknown,
    }
}

fn map_newline(newline: axt_fs::NewlineStyle) -> NewlineStyle {
    match newline {
        axt_fs::NewlineStyle::Lf => NewlineStyle::Lf,
        axt_fs::NewlineStyle::Crlf => NewlineStyle::Crlf,
        axt_fs::NewlineStyle::Mixed => NewlineStyle::Mixed,
        axt_fs::NewlineStyle::None => NewlineStyle::None,
    }
}

fn output_prefix(_root: &Utf8Path) -> String {
    String::new()
}

fn root_label(roots: &[Utf8PathBuf], cwd: &Utf8Path) -> String {
    if roots.len() != 1 {
        return ".".to_owned();
    }
    roots
        .first()
        .and_then(|root| root.strip_prefix(cwd).ok())
        .map_or_else(
            || roots[0].to_string(),
            |relative| {
                if relative.as_str().is_empty() {
                    ".".to_owned()
                } else {
                    relative.to_string()
                }
            },
        )
}

struct GitContext {
    repo: Option<axt_git::RepoHandle>,
    cache: Option<axt_git::StatusCache>,
}

impl GitContext {
    fn disabled() -> Self {
        Self {
            repo: None,
            cache: None,
        }
    }

    fn for_root(root: &Utf8Path) -> Result<Self> {
        let Some(repo) = axt_git::repo_root_for(root)? else {
            return Ok(Self::disabled());
        };
        let cache = axt_git::StatusCache::from_repo(&repo)?;
        Ok(Self {
            repo: Some(repo),
            cache: Some(cache),
        })
    }

    fn status_for(&self, root: &Utf8Path, path: &Utf8Path) -> GitStatus {
        let Some(repo) = self.repo.as_ref() else {
            return GitStatus::None;
        };
        let Some(cache) = self.cache.as_ref() else {
            return GitStatus::None;
        };
        let absolute = root.join(path);
        let Ok(relative) = absolute.strip_prefix(repo.root()) else {
            return GitStatus::None;
        };
        map_git_status(cache.status_for_relative(relative))
    }
}

fn map_git_status(status: axt_git::GitStatus) -> GitStatus {
    match status {
        axt_git::GitStatus::Clean => GitStatus::Clean,
        axt_git::GitStatus::Modified => GitStatus::Modified,
        axt_git::GitStatus::Untracked => GitStatus::Untracked,
        axt_git::GitStatus::Added => GitStatus::Added,
        axt_git::GitStatus::Deleted => GitStatus::Deleted,
        axt_git::GitStatus::Renamed => GitStatus::Renamed,
        axt_git::GitStatus::Mixed => GitStatus::Mixed,
    }
}
