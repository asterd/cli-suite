//! Internal use only, no stability guarantees.

#![deny(unsafe_code)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![allow(clippy::missing_errors_doc, clippy::module_name_repetitions)]

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    path::PathBuf,
};

use camino::{Utf8Path, Utf8PathBuf};
use gix::bstr::{BStr, ByteSlice};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Error type for git repository helpers.
#[derive(Debug, Error)]
pub enum GitError {
    /// A filesystem path was not valid UTF-8.
    #[error("path is not valid UTF-8: {0:?}")]
    PathNotUtf8(PathBuf),

    /// A git path was not valid UTF-8.
    #[error("git path is not valid UTF-8: {0}")]
    GitPathNotUtf8(String),

    /// A path was not inside the repository worktree.
    #[error("path {path} is not inside repository root {root}")]
    PathOutsideRepo {
        /// Repository root.
        root: Utf8PathBuf,
        /// Input path.
        path: Utf8PathBuf,
    },

    /// A git operation failed.
    #[error("{operation} failed: {message}")]
    Git {
        /// Operation being performed.
        operation: &'static str,
        /// Error message from gix.
        message: String,
    },
}

/// Git helper result type.
pub type Result<T> = std::result::Result<T, GitError>;

/// Open git repository handle.
pub struct RepoHandle {
    inner: gix::Repository,
    root: Utf8PathBuf,
}

impl fmt::Debug for RepoHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RepoHandle")
            .field("root", &self.root)
            .finish_non_exhaustive()
    }
}

impl RepoHandle {
    /// Repository worktree root.
    #[must_use]
    pub const fn root(&self) -> &Utf8PathBuf {
        &self.root
    }
}

/// Git status values exposed to command crates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GitStatus {
    /// No status entry for the path.
    Clean,
    /// Tracked file modified.
    Modified,
    /// Untracked path.
    Untracked,
    /// Added to the index.
    Added,
    /// Deleted from the worktree or index.
    Deleted,
    /// Renamed.
    Renamed,
    /// More than one status applies.
    Mixed,
}

/// Dirty count split used by summaries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct DirtyCount {
    /// Modified, added, deleted, renamed, or mixed paths.
    pub modified: usize,
    /// Untracked paths.
    pub untracked: usize,
}

/// Cached repository status, suitable for repeated lookups in large trees.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StatusCache {
    statuses: BTreeMap<Utf8PathBuf, GitStatus>,
}

impl StatusCache {
    /// Build a repository-wide status cache.
    pub fn from_repo(repo: &RepoHandle) -> Result<Self> {
        let mut statuses = BTreeMap::new();
        add_index_statuses(repo, &mut statuses)?;

        let iter = repo
            .inner
            .status(gix::progress::Discard)
            .map_err(|err| git_error("read status", err))?
            .untracked_files(gix::status::UntrackedFiles::Files)
            .into_index_worktree_iter(Vec::<gix::bstr::BString>::new())
            .map_err(|err| git_error("create status iterator", err))?;

        for item in iter {
            let item = item.map_err(|err| git_error("read status item", err))?;
            let status = status_from_item(&item);
            if status == GitStatus::Clean {
                continue;
            }
            let path = bstr_path(item.rela_path())?;
            statuses
                .entry(path)
                .and_modify(|current| *current = merge_status(*current, status))
                .or_insert(status);
        }

        Ok(Self { statuses })
    }

    /// Return the status for a repository-relative path.
    #[must_use]
    pub fn status_for_relative(&self, path: &Utf8Path) -> GitStatus {
        self.statuses.get(path).copied().unwrap_or(GitStatus::Clean)
    }

    /// Return the dirty count represented by this cache.
    #[must_use]
    pub fn dirty_count(&self) -> DirtyCount {
        self.statuses
            .values()
            .fold(DirtyCount::default(), |mut count, status| {
                match status {
                    GitStatus::Clean => {}
                    GitStatus::Untracked => count.untracked += 1,
                    GitStatus::Modified
                    | GitStatus::Added
                    | GitStatus::Deleted
                    | GitStatus::Renamed
                    | GitStatus::Mixed => count.modified += 1,
                }
                count
            })
    }
}

/// Discover the git repository containing `path`.
pub fn repo_root_for(path: &Utf8Path) -> Result<Option<RepoHandle>> {
    match gix::discover(path.as_std_path()) {
        Ok(inner) => {
            let root = inner
                .workdir()
                .unwrap_or_else(|| inner.git_dir())
                .to_path_buf();
            Ok(Some(RepoHandle {
                inner,
                root: utf8_path(root)?,
            }))
        }
        Err(err) if is_no_git_repository(&err) => Ok(None),
        Err(err) => Err(git_error("discover repository", err)),
    }
}

/// Return status for `path` within `repo`.
pub fn status_for(repo: &RepoHandle, path: &Utf8Path) -> Result<GitStatus> {
    let relative = relative_to_repo(repo, path)?;
    let cache = StatusCache::from_repo(repo)?;
    Ok(cache.status_for_relative(&relative))
}

/// Return the current branch name, or `None` for detached or unborn HEAD.
pub fn current_branch(repo: &RepoHandle) -> Result<Option<String>> {
    repo.inner
        .head_name()
        .map_err(|err| git_error("read current branch", err))?
        .map(|name| bstr_string(name.shorten()))
        .transpose()
}

/// Return repository dirty counts.
pub fn dirty_count(repo: &RepoHandle) -> Result<DirtyCount> {
    Ok(StatusCache::from_repo(repo)?.dirty_count())
}

/// Return repository-relative paths changed between two revisions.
pub fn diff_paths(repo: &RepoHandle, ref_a: &str, ref_b: &str) -> Result<Vec<Utf8PathBuf>> {
    let old_tree = tree_for_ref(repo, ref_a)?;
    let new_tree = tree_for_ref(repo, ref_b)?;
    let mut paths = BTreeSet::new();
    let mut platform = old_tree
        .changes()
        .map_err(|err| git_error("create tree diff", err))?;
    platform
        .for_each_to_obtain_tree(&new_tree, |change| {
            collect_diff_path(&mut paths, change)?;
            Ok::<_, GitError>(gix::object::tree::diff::Action::Continue)
        })
        .map_err(|err| git_error("read tree diff", err))?;

    Ok(paths.into_iter().collect())
}

fn tree_for_ref<'repo>(repo: &'repo RepoHandle, name: &str) -> Result<gix::Tree<'repo>> {
    repo.inner
        .rev_parse_single(BStr::new(name))
        .map_err(|err| git_error("parse revision", err))?
        .object()
        .map_err(|err| git_error("find revision object", err))?
        .peel_to_tree()
        .map_err(|err| git_error("peel revision to tree", err))
}

fn collect_diff_path(
    paths: &mut BTreeSet<Utf8PathBuf>,
    change: gix::object::tree::diff::Change<'_, '_, '_>,
) -> Result<()> {
    paths.insert(bstr_path(change.location())?);
    Ok(())
}

fn relative_to_repo(repo: &RepoHandle, path: &Utf8Path) -> Result<Utf8PathBuf> {
    let absolute = if path.is_absolute() {
        path.to_owned()
    } else {
        repo.root.join(path)
    };
    absolute
        .strip_prefix(&repo.root)
        .map(Utf8Path::to_path_buf)
        .map_err(|_err| GitError::PathOutsideRepo {
            root: repo.root.clone(),
            path: absolute,
        })
}

fn status_from_item(item: &gix::status::index_worktree::Item) -> GitStatus {
    item.summary()
        .map_or(GitStatus::Clean, status_from_worktree_summary)
}

const fn status_from_worktree_summary(
    summary: gix::status::index_worktree::iter::Summary,
) -> GitStatus {
    use gix::status::index_worktree::iter::Summary;

    match summary {
        Summary::Added => GitStatus::Untracked,
        Summary::Removed => GitStatus::Deleted,
        Summary::TypeChange | Summary::Modified => GitStatus::Modified,
        Summary::Renamed => GitStatus::Renamed,
        Summary::Copied | Summary::IntentToAdd => GitStatus::Added,
        Summary::Conflict => GitStatus::Mixed,
    }
}

const fn merge_status(left: GitStatus, right: GitStatus) -> GitStatus {
    match (left, right) {
        (GitStatus::Clean, status) | (status, GitStatus::Clean) => status,
        (GitStatus::Modified, GitStatus::Modified) => GitStatus::Modified,
        (GitStatus::Untracked, GitStatus::Untracked) => GitStatus::Untracked,
        (GitStatus::Added, GitStatus::Added) => GitStatus::Added,
        (GitStatus::Deleted, GitStatus::Deleted) => GitStatus::Deleted,
        (GitStatus::Renamed, GitStatus::Renamed) => GitStatus::Renamed,
        (
            GitStatus::Modified
            | GitStatus::Untracked
            | GitStatus::Added
            | GitStatus::Deleted
            | GitStatus::Renamed
            | GitStatus::Mixed,
            GitStatus::Modified
            | GitStatus::Untracked
            | GitStatus::Added
            | GitStatus::Deleted
            | GitStatus::Renamed
            | GitStatus::Mixed,
        ) => GitStatus::Mixed,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct EntryFingerprint {
    id: String,
    mode: String,
}

fn add_index_statuses(
    repo: &RepoHandle,
    statuses: &mut BTreeMap<Utf8PathBuf, GitStatus>,
) -> Result<()> {
    let head_entries = head_index_entries(repo)?;
    let index = repo
        .inner
        .index_or_empty()
        .map_err(|err| git_error("read index", err))?;
    let index = &*index;
    let mut current_entries = BTreeMap::new();
    for entry in index.entries() {
        current_entries.insert(
            bstr_path(entry.path(index))?,
            EntryFingerprint {
                id: entry.id.to_string(),
                mode: format!("{:?}", entry.mode),
            },
        );
    }

    let mut staged = BTreeMap::new();
    let paths = head_entries
        .keys()
        .chain(current_entries.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    for path in paths {
        let status = match (head_entries.get(&path), current_entries.get(&path)) {
            (None, Some(_current)) => GitStatus::Added,
            (Some(_head), None) => GitStatus::Deleted,
            (Some(head), Some(current)) if head != current => GitStatus::Modified,
            _ => GitStatus::Clean,
        };
        if status != GitStatus::Clean {
            staged.insert(path, status);
        }
    }

    mark_renames(&head_entries, &current_entries, &mut staged);
    for (path, status) in staged {
        insert_status(statuses, path, status);
    }

    Ok(())
}

fn head_index_entries(repo: &RepoHandle) -> Result<BTreeMap<Utf8PathBuf, EntryFingerprint>> {
    let commit = match repo.inner.head_commit() {
        Ok(commit) => commit,
        Err(_err) => return Ok(BTreeMap::new()),
    };
    let tree_id = commit
        .tree_id()
        .map_err(|err| git_error("read head tree", err))?;
    let index = repo
        .inner
        .index_from_tree(&tree_id)
        .map_err(|err| git_error("read head index", err))?;

    let mut entries = BTreeMap::new();
    for entry in index.entries() {
        entries.insert(
            bstr_path(entry.path(&index))?,
            EntryFingerprint {
                id: entry.id.to_string(),
                mode: format!("{:?}", entry.mode),
            },
        );
    }
    Ok(entries)
}

fn mark_renames(
    head_entries: &BTreeMap<Utf8PathBuf, EntryFingerprint>,
    current_entries: &BTreeMap<Utf8PathBuf, EntryFingerprint>,
    staged: &mut BTreeMap<Utf8PathBuf, GitStatus>,
) {
    let deleted = staged
        .iter()
        .filter_map(|(path, status)| (*status == GitStatus::Deleted).then_some(path.clone()))
        .collect::<Vec<_>>();
    let added = staged
        .iter()
        .filter_map(|(path, status)| (*status == GitStatus::Added).then_some(path.clone()))
        .collect::<Vec<_>>();

    for deleted_path in deleted {
        let Some(deleted_fingerprint) = head_entries.get(&deleted_path) else {
            continue;
        };
        for added_path in &added {
            if current_entries.get(added_path) == Some(deleted_fingerprint) {
                staged.insert(deleted_path.clone(), GitStatus::Renamed);
                staged.insert(added_path.clone(), GitStatus::Renamed);
            }
        }
    }
}

fn insert_status(
    statuses: &mut BTreeMap<Utf8PathBuf, GitStatus>,
    path: Utf8PathBuf,
    status: GitStatus,
) {
    statuses
        .entry(path)
        .and_modify(|current| *current = merge_status(*current, status))
        .or_insert(status);
}

fn bstr_path(path: &gix::bstr::BStr) -> Result<Utf8PathBuf> {
    bstr_string(path).map(Utf8PathBuf::from)
}

fn bstr_string(value: &gix::bstr::BStr) -> Result<String> {
    value
        .to_str()
        .map(str::to_owned)
        .map_err(|_err| GitError::GitPathNotUtf8(value.to_string()))
}

fn utf8_path(path: PathBuf) -> Result<Utf8PathBuf> {
    Utf8PathBuf::from_path_buf(path).map_err(GitError::PathNotUtf8)
}

fn git_error(operation: &'static str, error: impl fmt::Display) -> GitError {
    GitError::Git {
        operation,
        message: error.to_string(),
    }
}

const fn is_no_git_repository(error: &gix::discover::Error) -> bool {
    matches!(
        error,
        gix::discover::Error::Discover(
            gix::discover::upwards::Error::NoGitRepository { .. }
                | gix::discover::upwards::Error::NoGitRepositoryWithinCeiling { .. }
                | gix::discover::upwards::Error::NoGitRepositoryWithinFs { .. }
        )
    )
}

#[cfg(test)]
mod tests {
    use std::{error::Error, fs, io, process::Command};

    use super::*;

    #[test]
    fn repository_absence_is_graceful() -> std::result::Result<(), Box<dyn Error>> {
        let temp = tempfile::tempdir()?;
        let root = utf8_test_path(temp.path().to_path_buf())?;

        assert!(repo_root_for(&root)?.is_none());
        Ok(())
    }

    #[test]
    fn repository_status_branch_and_dirty_count_are_reported(
    ) -> std::result::Result<(), Box<dyn Error>> {
        let (_temp, root) = initialized_repo()?;
        let repo = require_repo(&root)?;

        assert_eq!(repo.root(), &root);
        assert_eq!(current_branch(&repo)?.as_deref(), Some("main"));

        fs::write(root.join("tracked.txt"), "tracked modified\n")?;
        fs::write(root.join("untracked.txt"), "new file\n")?;
        fs::write(root.join("added.txt"), "added to index\n")?;
        run_git(&root, &["add", "added.txt"])?;

        let cache = StatusCache::from_repo(&repo)?;
        assert_eq!(
            cache.status_for_relative(Utf8Path::new("tracked.txt")),
            GitStatus::Modified
        );
        assert_eq!(
            cache.status_for_relative(Utf8Path::new("untracked.txt")),
            GitStatus::Untracked
        );
        assert_eq!(
            cache.status_for_relative(Utf8Path::new("added.txt")),
            GitStatus::Added
        );
        assert_eq!(
            status_for(&repo, &root.join("tracked.txt"))?,
            GitStatus::Modified
        );
        assert_eq!(
            cache.dirty_count(),
            DirtyCount {
                modified: 2,
                untracked: 1,
            }
        );
        assert_eq!(dirty_count(&repo)?, cache.dirty_count());

        Ok(())
    }

    #[test]
    fn diff_paths_reports_changed_paths_between_revisions(
    ) -> std::result::Result<(), Box<dyn Error>> {
        let (_temp, root) = initialized_repo()?;
        fs::write(root.join("changed.txt"), "second version\n")?;
        run_git(&root, &["add", "changed.txt"])?;
        run_git(&root, &["commit", "-m", "second"])?;

        let repo = require_repo(&root)?;
        let changed = diff_paths(&repo, "HEAD~1", "HEAD")?;

        assert_eq!(changed, vec![Utf8PathBuf::from("changed.txt")]);
        Ok(())
    }

    fn initialized_repo() -> std::result::Result<(tempfile::TempDir, Utf8PathBuf), Box<dyn Error>> {
        let temp = tempfile::tempdir()?;
        let root = utf8_test_path(temp.path().join("repo"))?;
        copy_dir(&fixture_root(), &root)?;

        run_git(&root, &["init"])?;
        run_git(&root, &["config", "user.name", "axt tests"])?;
        run_git(
            &root,
            &["config", "user.email", "axt-tests@example.invalid"],
        )?;
        run_git(&root, &["add", "."])?;
        run_git(&root, &["commit", "-m", "initial"])?;
        run_git(&root, &["branch", "-M", "main"])?;

        Ok((temp, root))
    }

    fn fixture_root() -> Utf8PathBuf {
        Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/fs-with-git")
    }

    fn require_repo(root: &Utf8Path) -> std::result::Result<RepoHandle, Box<dyn Error>> {
        repo_root_for(root)?.ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "temporary repository was not discovered",
            )
            .into()
        })
    }

    fn copy_dir(from: &Utf8Path, to: &Utf8Path) -> io::Result<()> {
        fs::create_dir_all(to)?;
        for entry in fs::read_dir(from)? {
            let entry = entry?;
            let source = entry.path();
            let file_name = entry.file_name();
            let target = to.join(file_name.to_string_lossy().as_ref());
            if source.is_dir() {
                copy_dir(&utf8_test_path_io(source)?, &target)?;
            } else {
                fs::copy(source, target)?;
            }
        }
        Ok(())
    }

    fn run_git(root: &Utf8Path, args: &[&str]) -> std::result::Result<(), Box<dyn Error>> {
        let status = Command::new("git")
            .arg("-C")
            .arg(root.as_std_path())
            .args(args)
            .status()?;
        if status.success() {
            Ok(())
        } else {
            Err(io::Error::other(format!("git {} failed with {status}", args.join(" "))).into())
        }
    }

    fn utf8_test_path(path: PathBuf) -> std::result::Result<Utf8PathBuf, Box<dyn Error>> {
        utf8_test_path_io(path).map_err(Into::into)
    }

    fn utf8_test_path_io(path: PathBuf) -> io::Result<Utf8PathBuf> {
        Utf8PathBuf::from_path_buf(path).map_err(|path| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("test path is not valid UTF-8: {path:?}"),
            )
        })
    }
}
