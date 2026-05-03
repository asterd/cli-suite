use std::{
    fs, io,
    io::Write,
    process::{Command, ExitCode},
};

use axt_core::{ErrorCatalogEntry, ErrorCode, OutputMode, SchemaFormat, STANDARD_ERROR_CATALOG};
use axt_output::{
    AgentJsonlWriter, JsonEnvelope, RenderContext, Renderable, Result as RenderResult,
};
use camino::{Utf8Path, Utf8PathBuf};
use clap::Parser;
use serde::Serialize;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

const DEFAULT_COMMITS: usize = 5;
const DEFAULT_INLINE_DIFF_MAX_BYTES: usize = 12_000;

#[derive(Debug, Parser)]
#[command(name = "axt-gitctx")]
#[command(about = "Emit bounded local Git context.")]
#[command(version)]
struct Args {
    #[command(flatten)]
    common: axt_core::CommonArgs,

    #[arg(value_name = "ROOT", default_value = ".")]
    root: Utf8PathBuf,

    #[arg(long, default_value_t = DEFAULT_COMMITS, value_name = "N")]
    commits: usize,

    #[arg(
        long,
        default_value_t = DEFAULT_INLINE_DIFF_MAX_BYTES,
        value_name = "BYTES"
    )]
    inline_diff_max_bytes: usize,

    #[arg(long)]
    changed_only: bool,
}

#[derive(Debug, thiserror::Error)]
enum GitctxError {
    #[error("path not found: {0}")]
    PathNotFound(Utf8PathBuf),
    #[error("not inside a git repository: {0}")]
    NoGitRepository(Utf8PathBuf),
    #[error(transparent)]
    Git(#[from] axt_git::GitError),
    #[error("git executable is unavailable")]
    GitExecutableUnavailable,
    #[error("git {args} failed: {message}")]
    GitCommand { args: String, message: String },
    #[error("failed to read {path}: {source}")]
    ReadFile {
        path: Utf8PathBuf,
        source: io::Error,
    },
    #[error("failed to read metadata for {path}: {source}")]
    Metadata {
        path: Utf8PathBuf,
        source: io::Error,
    },
    #[error("git output was not valid UTF-8 for {operation}")]
    GitOutputUtf8 { operation: &'static str },
}

#[derive(Debug, Serialize)]
struct GitctxData {
    repo: String,
    root: String,
    git: GitRepositoryInfo,
    branch: BranchInfo,
    summary: Summary,
    files: Vec<ChangedFile>,
    commits: Vec<RecentCommit>,
    next: Vec<String>,
}

#[derive(Debug, Serialize)]
struct GitRepositoryInfo {
    shallow: bool,
    submodules: Vec<GitSubmoduleInfo>,
}

#[derive(Debug, Serialize)]
struct GitSubmoduleInfo {
    path: String,
    status: String,
    head: Option<String>,
}

#[derive(Debug, Serialize)]
struct BranchInfo {
    name: Option<String>,
    upstream: Option<String>,
    ahead: usize,
    behind: usize,
}

#[derive(Debug, Serialize)]
struct Summary {
    changed: usize,
    staged: usize,
    unstaged: usize,
    untracked: usize,
    added: usize,
    deleted: usize,
    dirty: bool,
    truncated: bool,
}

#[derive(Debug, Clone, Serialize)]
struct ChangedFile {
    path: String,
    previous_path: Option<String>,
    status: String,
    index_status: Option<String>,
    worktree_status: Option<String>,
    additions: usize,
    deletions: usize,
    hunks: usize,
    bytes: u64,
    diff_inline: bool,
    diff_truncated: bool,
    diff: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct RecentCommit {
    hash: String,
    subject: String,
    author: String,
    timestamp: Option<String>,
    age: Option<String>,
}

#[derive(Debug)]
struct StatusEntry {
    path: String,
    previous_path: Option<String>,
    index: StatusSlot,
    worktree: StatusSlot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StatusSlot {
    Unmodified,
    Modified,
    Added,
    Deleted,
    Renamed,
    Copied,
    UpdatedButUnmerged,
    Untracked,
    Ignored,
    Unknown,
}

#[derive(Debug, Serialize)]
struct AgentSummary<'a> {
    schema: &'static str,
    #[serde(rename = "type")]
    kind: &'static str,
    ok: bool,
    repo: &'a str,
    branch: Option<&'a str>,
    upstream: Option<&'a str>,
    ahead: usize,
    behind: usize,
    changed: usize,
    staged: usize,
    unstaged: usize,
    untracked: usize,
    shallow: bool,
    submodules: usize,
    dirty: bool,
    truncated: bool,
    next: &'a [String],
}

fn main() -> anyhow::Result<ExitCode> {
    let args = Args::parse();

    if let Some(format) = args.common.print_schema {
        print_schema(format);
        return Ok(ExitCode::SUCCESS);
    }

    if args.common.list_errors {
        write_error_catalog(std::io::stdout().lock(), STANDARD_ERROR_CATALOG)?;
        return Ok(ExitCode::SUCCESS);
    }

    let mode = args.common.mode()?;
    let ctx =
        axt_core::CommandContext::from_common_args(&args.common, Box::new(axt_core::SystemClock))?;
    let data = match run(&args, &ctx) {
        Ok(data) => data,
        Err(err) => {
            eprintln!("Error: {err}");
            return Ok(ExitCode::from(exit_code_for_error(&err)));
        }
    };
    let render_ctx =
        axt_output::RenderContext::new(mode, ctx.limits, ctx.color, ctx.clock.as_ref());
    let mut stdout = std::io::stdout().lock();
    let result = match mode {
        OutputMode::Human => data.render_human(&mut stdout, &render_ctx),
        OutputMode::Json => data.render_json(&mut stdout, &render_ctx),
        OutputMode::Agent => data.render_agent(&mut stdout, &render_ctx),
    };

    match result {
        Ok(()) => Ok(ExitCode::SUCCESS),
        Err(axt_output::OutputError::TruncatedStrict) => {
            Ok(ExitCode::from(ErrorCode::OutputTruncatedStrict.exit_code()))
        }
        Err(err) => Err(err.into()),
    }
}

fn run(args: &Args, ctx: &axt_core::CommandContext) -> Result<GitctxData, GitctxError> {
    let input_root = resolve_root(&ctx.cwd, &args.root);
    if !input_root.exists() {
        return Err(GitctxError::PathNotFound(input_root));
    }
    let repo = axt_git::repo_root_for(&input_root)?
        .ok_or_else(|| GitctxError::NoGitRepository(input_root.clone()))?;
    let repo_root = repo.root().clone();
    let git = git_repository_info(&repo)?;
    let repo_display = args.root.to_string();
    let upstream_ref = upstream_ref();
    let upstream = git_optional_line(
        &repo_root,
        &["rev-parse", "--abbrev-ref", upstream_ref.as_str()],
    )?;
    let (ahead, behind) = ahead_behind(&repo_root, upstream.as_deref())?;
    let branch = BranchInfo {
        name: axt_git::current_branch(&repo)?,
        upstream,
        ahead,
        behind,
    };

    let entries = status_entries(&repo_root)?;
    let mut files = Vec::with_capacity(entries.len());
    for entry in &entries {
        files.push(changed_file(&repo_root, entry, args.inline_diff_max_bytes)?);
    }
    let commits = if args.changed_only {
        Vec::new()
    } else {
        recent_commits(&repo_root, args.commits, ctx.clock.as_ref())?
    };
    let summary = summarize(&files, &entries, ctx.limits.max_records);
    let next = next_hints(&files);
    Ok(GitctxData {
        repo: repo_display,
        root: repo_root.to_string(),
        git,
        branch,
        summary,
        files,
        commits,
        next,
    })
}

fn git_repository_info(repo: &axt_git::RepoHandle) -> Result<GitRepositoryInfo, GitctxError> {
    let info = axt_git::repository_info(repo)?;
    Ok(GitRepositoryInfo {
        shallow: info.shallow,
        submodules: info
            .submodules
            .into_iter()
            .map(|submodule| GitSubmoduleInfo {
                path: submodule.path.to_string(),
                status: git_status_label(submodule.status).to_owned(),
                head: submodule.head,
            })
            .collect(),
    })
}

fn resolve_root(cwd: &Utf8Path, root: &Utf8Path) -> Utf8PathBuf {
    if root.is_absolute() {
        root.to_path_buf()
    } else {
        cwd.join(root)
    }
}

fn upstream_ref() -> String {
    let mut value = String::from("@");
    value.push('{');
    value.push_str("upstream");
    value.push('}');
    value
}

fn ahead_behind(
    repo_root: &Utf8Path,
    upstream: Option<&str>,
) -> Result<(usize, usize), GitctxError> {
    if upstream.is_none() {
        return Ok((0, 0));
    }
    let upstream_range = format!("HEAD...{}", upstream_ref());
    let output = git_output(
        repo_root,
        &["rev-list", "--left-right", "--count", upstream_range.as_str()],
    )?;
    let text = utf8_output("rev-list", &output)?;
    let mut parts = text.split_whitespace();
    let ahead = parts
        .next()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0);
    let behind = parts
        .next()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0);
    Ok((ahead, behind))
}

fn status_entries(repo_root: &Utf8Path) -> Result<Vec<StatusEntry>, GitctxError> {
    let output = git_output(repo_root, &["status", "--porcelain=v1", "-z"])?;
    let fields = output
        .split(|byte| *byte == 0)
        .filter(|field| !field.is_empty())
        .collect::<Vec<_>>();
    let mut entries = Vec::new();
    let mut index = 0;
    while index < fields.len() {
        let field = fields[index];
        let text = std::str::from_utf8(field)
            .map_err(|_err| GitctxError::GitOutputUtf8 { operation: "status" })?;
        if text.len() < 4 {
            index += 1;
            continue;
        }
        let status = &text[..2];
        let path = &text[3..];
        let mut chars = status.chars();
        let index_status = chars.next().map_or(StatusSlot::Unknown, status_slot);
        let worktree_status = chars.next().map_or(StatusSlot::Unknown, status_slot);
        let mut previous_path = None;
        if matches!(index_status, StatusSlot::Renamed | StatusSlot::Copied)
            || matches!(worktree_status, StatusSlot::Renamed | StatusSlot::Copied)
        {
            if let Some(previous) = fields.get(index + 1) {
                previous_path = Some(
                    std::str::from_utf8(previous)
                        .map_err(|_err| GitctxError::GitOutputUtf8 {
                            operation: "status rename",
                        })?
                        .to_owned(),
                );
                index += 1;
            }
        }
        entries.push(StatusEntry {
            path: path.to_owned(),
            previous_path,
            index: index_status,
            worktree: worktree_status,
        });
        index += 1;
    }
    Ok(entries)
}

fn changed_file(
    repo_root: &Utf8Path,
    entry: &StatusEntry,
    inline_diff_max_bytes: usize,
) -> Result<ChangedFile, GitctxError> {
    let (additions, deletions) = diff_numstat(repo_root, entry)?;
    let hunks = diff_hunks(repo_root, entry)?;
    let bytes = file_size(repo_root, &entry.path)?;
    let raw_diff = diff_text(repo_root, entry)?;
    let diff_bytes = raw_diff.len();
    let include_diff = diff_bytes > 0 && diff_bytes <= inline_diff_max_bytes;
    Ok(ChangedFile {
        path: entry.path.clone(),
        previous_path: entry.previous_path.clone(),
        status: overall_status(entry).to_owned(),
        index_status: status_label_optional(entry.index).map(str::to_owned),
        worktree_status: status_label_optional(entry.worktree).map(str::to_owned),
        additions,
        deletions,
        hunks,
        bytes,
        diff_inline: include_diff,
        diff_truncated: diff_bytes > inline_diff_max_bytes,
        diff: include_diff.then_some(raw_diff),
    })
}

fn diff_numstat(repo_root: &Utf8Path, entry: &StatusEntry) -> Result<(usize, usize), GitctxError> {
    if entry.index == StatusSlot::Untracked && entry.worktree == StatusSlot::Untracked {
        return Ok((line_count(repo_root, &entry.path)?, 0));
    }
    let mut additions = 0;
    let mut deletions = 0;
    for args in [
        vec![
            "diff",
            "--no-ext-diff",
            "--no-textconv",
            "--numstat",
            "--",
            entry.path.as_str(),
        ],
        vec![
            "diff",
            "--cached",
            "--no-ext-diff",
            "--no-textconv",
            "--numstat",
            "--",
            entry.path.as_str(),
        ],
    ] {
        let output = git_output(repo_root, &args)?;
        let text = utf8_output("diff --numstat", &output)?;
        for line in text.lines() {
            let mut fields = line.split('\t');
            let add = fields
                .next()
                .and_then(|value| value.parse::<usize>().ok())
                .unwrap_or(0);
            let del = fields
                .next()
                .and_then(|value| value.parse::<usize>().ok())
                .unwrap_or(0);
            additions += add;
            deletions += del;
        }
    }
    Ok((additions, deletions))
}

fn diff_hunks(repo_root: &Utf8Path, entry: &StatusEntry) -> Result<usize, GitctxError> {
    if entry.index == StatusSlot::Untracked && entry.worktree == StatusSlot::Untracked {
        return Ok(0);
    }
    let mut hunks = 0;
    for args in [
        vec![
            "diff",
            "--no-ext-diff",
            "--no-textconv",
            "--unified=0",
            "--",
            entry.path.as_str(),
        ],
        vec![
            "diff",
            "--cached",
            "--no-ext-diff",
            "--no-textconv",
            "--unified=0",
            "--",
            entry.path.as_str(),
        ],
    ] {
        let output = git_output(repo_root, &args)?;
        let text = utf8_output("diff --unified=0", &output)?;
        hunks += text.lines().filter(|line| line.starts_with("@@")).count();
    }
    Ok(hunks)
}

fn diff_text(repo_root: &Utf8Path, entry: &StatusEntry) -> Result<String, GitctxError> {
    if entry.index == StatusSlot::Untracked && entry.worktree == StatusSlot::Untracked {
        return synthetic_untracked_diff(repo_root, &entry.path);
    }
    let mut combined = String::new();
    for args in [
        vec![
            "diff",
            "--no-ext-diff",
            "--no-textconv",
            "--",
            entry.path.as_str(),
        ],
        vec![
            "diff",
            "--cached",
            "--no-ext-diff",
            "--no-textconv",
            "--",
            entry.path.as_str(),
        ],
    ] {
        let output = git_output(repo_root, &args)?;
        let text = utf8_output("diff", &output)?;
        if !text.is_empty() {
            combined.push_str(&text);
            if !combined.ends_with('\n') {
                combined.push('\n');
            }
        }
    }
    Ok(combined)
}

fn synthetic_untracked_diff(repo_root: &Utf8Path, path: &str) -> Result<String, GitctxError> {
    let absolute = repo_root.join(path);
    if absolute.is_dir() {
        return Ok(String::new());
    }
    let text = match fs::read_to_string(&absolute) {
        Ok(value) => value,
        Err(err) if err.kind() == io::ErrorKind::InvalidData => return Ok(String::new()),
        Err(source) => {
            return Err(GitctxError::ReadFile {
                path: absolute,
                source,
            });
        }
    };
    let diff_path_a = quote_diff_path(&format!("a/{path}"));
    let diff_path_b = quote_diff_path(&format!("b/{path}"));
    let mut diff = format!(
        "diff --git {diff_path_a} {diff_path_b}\nnew file mode 100644\n--- /dev/null\n+++ {diff_path_b}\n"
    );
    if !text.is_empty() {
        diff.push_str("@@ -0,0 +1");
        let count = text.lines().count();
        if count > 1 {
            diff.push(',');
            diff.push_str(&count.to_string());
        }
        diff.push_str(" @@\n");
        for line in text.lines() {
            diff.push('+');
            diff.push_str(line);
            diff.push('\n');
        }
    }
    Ok(diff)
}

fn quote_diff_path(path: &str) -> String {
    let mut quoted = String::from("\"");
    for byte in path.bytes() {
        match byte {
            b'\n' => quoted.push_str("\\n"),
            b'\r' => quoted.push_str("\\r"),
            b'\t' => quoted.push_str("\\t"),
            b'\\' => quoted.push_str("\\\\"),
            b'"' => quoted.push_str("\\\""),
            0x20..=0x7e => quoted.push(char::from(byte)),
            other => {
                quoted.push('\\');
                quoted.push(char::from(b'0' + ((other >> 6) & 0o7)));
                quoted.push(char::from(b'0' + ((other >> 3) & 0o7)));
                quoted.push(char::from(b'0' + (other & 0o7)));
            }
        }
    }
    quoted.push('"');
    quoted
}

fn file_size(repo_root: &Utf8Path, path: &str) -> Result<u64, GitctxError> {
    let absolute = repo_root.join(path);
    match fs::metadata(&absolute) {
        Ok(metadata) => Ok(metadata.len()),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(0),
        Err(source) => Err(GitctxError::Metadata {
            path: absolute,
            source,
        }),
    }
}

fn line_count(repo_root: &Utf8Path, path: &str) -> Result<usize, GitctxError> {
    let absolute = repo_root.join(path);
    if absolute.is_dir() {
        return Ok(0);
    }
    match fs::read_to_string(&absolute) {
        Ok(text) => Ok(text.lines().count()),
        Err(err) if err.kind() == io::ErrorKind::InvalidData => Ok(0),
        Err(source) => Err(GitctxError::ReadFile {
            path: absolute,
            source,
        }),
    }
}

fn recent_commits(
    repo_root: &Utf8Path,
    limit: usize,
    clock: &dyn axt_core::Clock,
) -> Result<Vec<RecentCommit>, GitctxError> {
    if limit == 0 {
        return Ok(Vec::new());
    }
    let count = format!("-n{limit}");
    let output = match git_output(
        repo_root,
        &["log", count.as_str(), "--pretty=format:%H%x1f%an%x1f%aI%x1f%s%x1e"],
    ) {
        Ok(output) => output,
        Err(GitctxError::GitCommand { .. }) => return Ok(Vec::new()),
        Err(err) => return Err(err),
    };
    let text = utf8_output("log", &output)?;
    let now = clock.now_utc();
    let mut commits = Vec::new();
    for record in text.split('\x1e').filter(|record| !record.trim().is_empty()) {
        let mut fields = record.trim_matches('\n').split('\x1f');
        let hash = fields.next().unwrap_or_default().to_owned();
        let author = fields.next().unwrap_or_default().to_owned();
        let timestamp = fields.next().map(str::to_owned);
        let subject = fields.next().unwrap_or_default().to_owned();
        let age = timestamp
            .as_deref()
            .and_then(|value| OffsetDateTime::parse(value, &Rfc3339).ok())
            .map(|time| relative_age(now, time));
        commits.push(RecentCommit {
            hash,
            subject,
            author,
            timestamp,
            age,
        });
    }
    Ok(commits)
}

fn relative_age(now: OffsetDateTime, then: OffsetDateTime) -> String {
    let duration = now - then;
    let seconds = duration.whole_seconds().max(0);
    if seconds < 60 {
        format!("{seconds}s")
    } else if seconds < 3_600 {
        format!("{}m", seconds / 60)
    } else if seconds < 86_400 {
        format!("{}h", seconds / 3_600)
    } else {
        format!("{}d", seconds / 86_400)
    }
}

fn summarize(files: &[ChangedFile], entries: &[StatusEntry], max_records: usize) -> Summary {
    let staged = entries
        .iter()
        .filter(|entry| is_staged(entry.index))
        .count();
    let unstaged = entries
        .iter()
        .filter(|entry| is_unstaged(entry.worktree))
        .count();
    let untracked = entries
        .iter()
        .filter(|entry| entry.index == StatusSlot::Untracked && entry.worktree == StatusSlot::Untracked)
        .count();
    let added = files.iter().map(|file| file.additions).sum();
    let deleted = files.iter().map(|file| file.deletions).sum();
    Summary {
        changed: files.len(),
        staged,
        unstaged,
        untracked,
        added,
        deleted,
        dirty: !files.is_empty(),
        truncated: files.len().saturating_add(1) > max_records,
    }
}

fn next_hints(files: &[ChangedFile]) -> Vec<String> {
    files
        .iter()
        .take(3)
        .map(|file| format!("axt-slice {} --agent", shell_quote(&file.path)))
        .collect()
}

fn shell_quote(value: &str) -> String {
    if value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'.' | b'_' | b'-'))
    {
        value.to_owned()
    } else {
        format!("'{}'", value.replace('\'', "'\\''"))
    }
}

fn git_optional_line(repo_root: &Utf8Path, args: &[&str]) -> Result<Option<String>, GitctxError> {
    match git_output(repo_root, args) {
        Ok(output) => {
            let text = utf8_output("git optional", &output)?;
            let trimmed = text.trim();
            if trimmed.is_empty() {
                Ok(None)
            } else {
                Ok(Some(trimmed.to_owned()))
            }
        }
        Err(GitctxError::GitCommand { .. }) => Ok(None),
        Err(err) => Err(err),
    }
}

fn git_output(repo_root: &Utf8Path, args: &[&str]) -> Result<Vec<u8>, GitctxError> {
    let output = Command::new("git")
        .args(args)
        .current_dir(repo_root)
        .output()
        .map_err(|err| {
            if err.kind() == io::ErrorKind::NotFound {
                GitctxError::GitExecutableUnavailable
            } else {
                GitctxError::GitCommand {
                    args: args.join(" "),
                    message: err.to_string(),
                }
            }
        })?;
    if output.status.success() {
        Ok(output.stdout)
    } else {
        let message = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        Err(GitctxError::GitCommand {
            args: args.join(" "),
            message,
        })
    }
}

fn utf8_output(operation: &'static str, bytes: &[u8]) -> Result<String, GitctxError> {
    String::from_utf8(bytes.to_vec()).map_err(|_err| GitctxError::GitOutputUtf8 { operation })
}

const fn status_slot(ch: char) -> StatusSlot {
    match ch {
        ' ' => StatusSlot::Unmodified,
        'M' => StatusSlot::Modified,
        'A' => StatusSlot::Added,
        'D' => StatusSlot::Deleted,
        'R' => StatusSlot::Renamed,
        'C' => StatusSlot::Copied,
        'U' => StatusSlot::UpdatedButUnmerged,
        '?' => StatusSlot::Untracked,
        '!' => StatusSlot::Ignored,
        _ => StatusSlot::Unknown,
    }
}

const fn status_label_optional(slot: StatusSlot) -> Option<&'static str> {
    match slot {
        StatusSlot::Unmodified => None,
        StatusSlot::Modified => Some("modified"),
        StatusSlot::Added => Some("added"),
        StatusSlot::Deleted => Some("deleted"),
        StatusSlot::Renamed => Some("renamed"),
        StatusSlot::Copied => Some("copied"),
        StatusSlot::UpdatedButUnmerged => Some("unmerged"),
        StatusSlot::Untracked => Some("untracked"),
        StatusSlot::Ignored => Some("ignored"),
        StatusSlot::Unknown => Some("unknown"),
    }
}

const fn git_status_label(status: axt_git::GitStatus) -> &'static str {
    match status {
        axt_git::GitStatus::Clean => "clean",
        axt_git::GitStatus::Modified => "modified",
        axt_git::GitStatus::Untracked => "untracked",
        axt_git::GitStatus::Added => "added",
        axt_git::GitStatus::Deleted => "deleted",
        axt_git::GitStatus::Renamed => "renamed",
        axt_git::GitStatus::Mixed => "mixed",
    }
}

fn overall_status(entry: &StatusEntry) -> &'static str {
    if entry.index == StatusSlot::Untracked && entry.worktree == StatusSlot::Untracked {
        return "untracked";
    }
    for slot in [entry.index, entry.worktree] {
        match slot {
            StatusSlot::Renamed => return "renamed",
            StatusSlot::Copied => return "copied",
            StatusSlot::Added => return "added",
            StatusSlot::Deleted => return "deleted",
            StatusSlot::Modified => return "modified",
            StatusSlot::UpdatedButUnmerged => return "unmerged",
            StatusSlot::Unmodified | StatusSlot::Untracked | StatusSlot::Ignored | StatusSlot::Unknown => {}
        }
    }
    "modified"
}

const fn is_staged(slot: StatusSlot) -> bool {
    !matches!(
        slot,
        StatusSlot::Unmodified | StatusSlot::Untracked | StatusSlot::Ignored
    )
}

const fn is_unstaged(slot: StatusSlot) -> bool {
    !matches!(
        slot,
        StatusSlot::Unmodified | StatusSlot::Ignored
    )
}

impl Renderable for GitctxData {
    fn render_human(&self, w: &mut dyn Write, _ctx: &RenderContext<'_>) -> RenderResult<()> {
        writeln!(w, "Repository {}", self.repo)?;
        writeln!(
            w,
            "Branch     {} upstream={} ahead={} behind={}",
            self.branch.name.as_deref().unwrap_or("detached"),
            self.branch.upstream.as_deref().unwrap_or("none"),
            self.branch.ahead,
            self.branch.behind
        )?;
        writeln!(
            w,
            "Summary    changed={} staged={} unstaged={} untracked={} +{} -{} dirty={} truncated={}",
            self.summary.changed,
            self.summary.staged,
            self.summary.unstaged,
            self.summary.untracked,
            self.summary.added,
            self.summary.deleted,
            self.summary.dirty,
            self.summary.truncated
        )?;
        writeln!(
            w,
            "Git        shallow={} submodules={}",
            self.git.shallow,
            self.git.submodules.len()
        )?;
        if !self.files.is_empty() {
            writeln!(w)?;
            writeln!(w, "Changes")?;
            for file in &self.files {
                let previous = file
                    .previous_path
                    .as_ref()
                    .map_or(String::new(), |path| format!(" from {path}"));
                writeln!(
                    w,
                    "  {:<10} {:<32} +{} -{} hunks={} bytes={}{}",
                    file.status,
                    file.path,
                    file.additions,
                    file.deletions,
                    file.hunks,
                    file.bytes,
                    previous
                )?;
            }
        }
        if !self.commits.is_empty() {
            writeln!(w)?;
            writeln!(w, "Commits")?;
            for commit in &self.commits {
                let short = commit.hash.get(..7).unwrap_or(commit.hash.as_str());
                writeln!(w, "  {} {} {}", short, commit.author, commit.subject)?;
            }
        }
        for file in self.files.iter().filter(|file| file.diff_inline) {
            if let Some(diff) = &file.diff {
                writeln!(w)?;
                writeln!(w, "Diff {}", file.path)?;
                write!(w, "{diff}")?;
                if !diff.ends_with('\n') {
                    writeln!(w)?;
                }
            }
        }
        Ok(())
    }

    fn render_json(&self, w: &mut dyn Write, _ctx: &RenderContext<'_>) -> RenderResult<()> {
        let envelope = JsonEnvelope::new("axt.gitctx.v1", self, Vec::new(), Vec::new());
        serde_json::to_writer(&mut *w, &envelope)?;
        writeln!(w)?;
        Ok(())
    }

    fn render_agent(&self, w: &mut dyn Write, ctx: &RenderContext<'_>) -> RenderResult<()> {
        let detail_records = agent_detail_records(self);
        let truncated = agent_would_truncate(self, &detail_records, ctx)?;
        let mut writer = AgentJsonlWriter::new(w, ctx.limits);
        writer.write_record(&AgentSummary {
            schema: "axt.gitctx.summary.v1",
            kind: "summary",
            ok: true,
            repo: &self.repo,
            branch: self.branch.name.as_deref(),
            upstream: self.branch.upstream.as_deref(),
            ahead: self.branch.ahead,
            behind: self.branch.behind,
            changed: self.summary.changed,
            staged: self.summary.staged,
            unstaged: self.summary.unstaged,
            untracked: self.summary.untracked,
            shallow: self.git.shallow,
            submodules: self.git.submodules.len(),
            dirty: self.summary.dirty,
            truncated,
            next: &self.next,
        })?;
        for record in &detail_records {
            writer.write_record(record)?;
        }
        let _summary = writer.finish("axt.gitctx.warn.v1")?;
        Ok(())
    }
}

fn agent_detail_records(data: &GitctxData) -> Vec<serde_json::Value> {
    let mut records = Vec::with_capacity(
        data.files
            .len()
            .saturating_add(data.commits.len())
            .saturating_add(data.git.submodules.len()),
    );
    for submodule in &data.git.submodules {
        records.push(serde_json::json!({
            "schema": "axt.gitctx.submodule.v1",
            "type": "submodule",
            "path": submodule.path,
            "status": submodule.status,
            "head": submodule.head,
        }));
    }
    for file in &data.files {
        records.push(serde_json::json!({
            "schema": "axt.gitctx.file.v1",
            "type": "file",
            "p": file.path,
            "prev": file.previous_path,
            "g": file.status,
            "idx": file.index_status,
            "wt": file.worktree_status,
            "add": file.additions,
            "del": file.deletions,
            "hunks": file.hunks,
            "b": file.bytes,
            "diff_inline": file.diff_inline,
            "diff_truncated": file.diff_truncated,
            "diff": file.diff,
        }));
    }
    for commit in &data.commits {
        records.push(serde_json::json!({
            "schema": "axt.gitctx.commit.v1",
            "type": "commit",
            "hash": commit.hash,
            "subject": commit.subject,
            "author": commit.author,
            "ts": commit.timestamp,
            "age": commit.age,
        }));
    }
    records
}

fn agent_would_truncate(
    data: &GitctxData,
    detail_records: &[serde_json::Value],
    ctx: &RenderContext<'_>,
) -> RenderResult<bool> {
    let summary = AgentSummary {
        schema: "axt.gitctx.summary.v1",
        kind: "summary",
        ok: true,
        repo: &data.repo,
        branch: data.branch.name.as_deref(),
        upstream: data.branch.upstream.as_deref(),
        ahead: data.branch.ahead,
        behind: data.branch.behind,
        changed: data.summary.changed,
        staged: data.summary.staged,
        unstaged: data.summary.unstaged,
        untracked: data.summary.untracked,
        shallow: data.git.shallow,
        submodules: data.git.submodules.len(),
        dirty: data.summary.dirty,
        truncated: false,
        next: &data.next,
    };
    let mut bytes = json_record_len(&summary)?;
    if ctx.limits.max_records == 0 || bytes > ctx.limits.max_bytes {
        return Ok(true);
    }
    for (index, record) in detail_records.iter().enumerate() {
        let records = index + 1;
        let len = json_record_len(record)?;
        if records >= ctx.limits.max_records || bytes + len > ctx.limits.max_bytes {
            return Ok(true);
        }
        bytes += len;
    }
    Ok(false)
}

fn json_record_len(record: &impl Serialize) -> RenderResult<usize> {
    Ok(serde_json::to_vec(record)?.len() + 1)
}

fn print_schema(format: SchemaFormat) {
    match format {
        SchemaFormat::Json => {
            print!("{}", include_str!("../../../schemas/axt.gitctx.v1.schema.json"));
        }
        SchemaFormat::Agent => println!(
            "schema=axt.gitctx.agent.v1 records=axt.gitctx.summary.v1,axt.gitctx.file.v1,axt.gitctx.commit.v1,axt.gitctx.warn.v1 first=summary"
        ),
        SchemaFormat::Human => println!(
            "schema=axt.gitctx.human.v1 sections=repository,branch,summary,changes,commits,diffs"
        ),
    }
}

fn write_error_catalog(
    mut w: impl Write,
    catalog: &[ErrorCatalogEntry],
) -> Result<(), serde_json::Error> {
    for entry in catalog {
        serde_json::to_writer(&mut w, entry)?;
        if let Err(err) = writeln!(w) {
            return Err(serde_json::Error::io(err));
        }
    }
    Ok(())
}

fn exit_code_for_error(err: &GitctxError) -> u8 {
    match err {
        GitctxError::PathNotFound(_) => ErrorCode::PathNotFound.exit_code(),
        GitctxError::NoGitRepository(_)
        | GitctxError::Git(_)
        | GitctxError::GitExecutableUnavailable
        | GitctxError::GitCommand { .. } => ErrorCode::GitUnavailable.exit_code(),
        GitctxError::ReadFile { source, .. } | GitctxError::Metadata { source, .. }
            if source.kind() == io::ErrorKind::PermissionDenied =>
        {
            ErrorCode::PermissionDenied.exit_code()
        }
        GitctxError::ReadFile { .. } | GitctxError::Metadata { .. } => {
            ErrorCode::IoError.exit_code()
        }
        GitctxError::GitOutputUtf8 { .. } => ErrorCode::RuntimeError.exit_code(),
    }
}
