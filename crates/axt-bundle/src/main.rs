#![deny(unsafe_code)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![allow(clippy::missing_errors_doc, clippy::module_name_repetitions)]

use std::{fs, io::Write, process::ExitCode};

use axt_core::{ErrorCatalogEntry, ErrorCode, OutputMode, SchemaFormat, STANDARD_ERROR_CATALOG};
use axt_fs::{EntryKind, WalkOptions};
use axt_output::{
    AgentJsonlWriter, JsonEnvelope, RenderContext, Renderable, Result as RenderResult,
};
use camino::Utf8PathBuf;
use clap::Parser;
use serde::Serialize;

const MANIFEST_NAMES: &[&str] = &[
    "Cargo.toml",
    "package.json",
    "pyproject.toml",
    "go.mod",
    "deno.json",
    "bun.lock",
    "pnpm-lock.yaml",
    "package-lock.json",
];

#[derive(Debug, Parser)]
#[command(name = "axt-bundle")]
#[command(about = "Emit a compact session warmup bundle.")]
#[command(version)]
struct Args {
    #[command(flatten)]
    common: axt_core::CommonArgs,

    #[arg(value_name = "ROOT", default_value = ".")]
    root: Utf8PathBuf,

    #[arg(long, default_value_t = 2, value_name = "N")]
    depth: usize,

    #[arg(long, default_value_t = 40, value_name = "N")]
    max_files: usize,

    #[arg(long)]
    include_hidden: bool,

    #[arg(long)]
    no_ignore: bool,
}

#[derive(Debug, thiserror::Error)]
enum BundleError {
    #[error("path not found: {0}")]
    PathNotFound(Utf8PathBuf),
    #[error(transparent)]
    Fs(#[from] axt_fs::FsError),
    #[error(transparent)]
    Git(#[from] axt_git::GitError),
    #[error("failed to read manifest {path}: {source}")]
    ReadManifest {
        path: Utf8PathBuf,
        source: std::io::Error,
    },
}

#[derive(Debug, Serialize)]
struct BundleData {
    root: String,
    summary: BundleSummary,
    files: Vec<BundleFile>,
    manifests: Vec<BundleManifest>,
    git: Option<BundleGit>,
    next: Vec<String>,
}

#[derive(Debug, Serialize)]
struct BundleSummary {
    files: usize,
    dirs: usize,
    manifests: usize,
    git: bool,
    truncated: bool,
}

#[derive(Debug, Serialize)]
struct BundleFile {
    path: String,
    kind: String,
    bytes: u64,
    lang: Option<String>,
}

#[derive(Debug, Serialize)]
struct BundleManifest {
    path: String,
    kind: String,
    bytes: u64,
    preview: String,
}

#[derive(Debug, Serialize)]
struct BundleGit {
    root: String,
    branch: Option<String>,
    modified: usize,
    untracked: usize,
}

#[derive(Debug, Serialize)]
struct AgentSummary<'a> {
    schema: &'static str,
    #[serde(rename = "type")]
    kind: &'static str,
    ok: bool,
    root: &'a str,
    files: usize,
    dirs: usize,
    manifests: usize,
    git: bool,
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
    let data = match run(&args, &ctx.cwd) {
        Ok(data) => data,
        Err(err) => {
            eprintln!("Error: {err}");
            return Ok(ExitCode::from(exit_code_for_error(&err)));
        }
    };
    let render_ctx = RenderContext::new(mode, ctx.limits, ctx.color, ctx.clock.as_ref());
    let mut stdout = std::io::stdout().lock();
    let result = match mode {
        OutputMode::Human => data.render_human(&mut stdout, &render_ctx),
        OutputMode::Compact => data.render_compact(&mut stdout, &render_ctx),
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

fn run(args: &Args, cwd: &Utf8PathBuf) -> Result<BundleData, BundleError> {
    let root = if args.root.is_absolute() {
        args.root.clone()
    } else {
        cwd.join(&args.root)
    };
    if !root.exists() {
        return Err(BundleError::PathNotFound(root));
    }

    let collection = axt_fs::collect_metadata_with_warnings(
        &root,
        WalkOptions {
            max_depth: Some(args.depth),
            include_hidden: args.include_hidden,
            no_ignore: args.no_ignore,
            ..WalkOptions::default()
        },
    )?;
    let total_files = collection
        .entries
        .iter()
        .filter(|entry| entry.kind == EntryKind::File)
        .count();
    let total_dirs = collection
        .entries
        .iter()
        .filter(|entry| entry.kind == EntryKind::Dir)
        .count();
    let walk_entries = collection
        .entries
        .iter()
        .filter(|entry| matches!(entry.kind, EntryKind::File | EntryKind::Dir))
        .collect::<Vec<_>>();
    debug_assert!(walk_entries
        .windows(2)
        .all(|items| items[0].path <= items[1].path));
    let files = walk_entries
        .iter()
        .take(args.max_files)
        .map(|entry| BundleFile {
            path: entry.path.to_string(),
            kind: entry_kind(entry.kind).to_owned(),
            bytes: entry.size,
            lang: entry.language.clone(),
        })
        .collect::<Vec<_>>();
    let manifests = collect_manifests(&root)?;
    let git = collect_git(&root)?;
    let root_display = args.root.to_string();
    let next = next_hints(&root_display, git.as_ref(), !manifests.is_empty());
    Ok(BundleData {
        root: root_display,
        summary: BundleSummary {
            files: total_files,
            dirs: total_dirs,
            manifests: manifests.len(),
            git: git.is_some(),
            truncated: walk_entries.len() > files.len(),
        },
        files,
        manifests,
        git,
        next,
    })
}

fn collect_manifests(root: &Utf8PathBuf) -> Result<Vec<BundleManifest>, BundleError> {
    let mut manifests = Vec::new();
    for name in MANIFEST_NAMES {
        let path = root.join(name);
        if !path.is_file() {
            continue;
        }
        let decoded =
            axt_fs::read_to_string_smart(&path).map_err(|source| BundleError::ReadManifest {
                path: path.clone(),
                source: std::io::Error::other(source.to_string()),
            })?;
        if decoded.converted || decoded.lossy {
            eprintln!(
                "Warning: manifest {} decoded as {}{}",
                path,
                decoded.encoding.as_str(),
                if decoded.lossy {
                    " with replacement characters"
                } else {
                    ""
                }
            );
        }
        let text = decoded.text;
        let preview = text.lines().take(12).collect::<Vec<_>>().join("\n");
        let bytes = fs::metadata(&path)
            .map_err(|source| BundleError::ReadManifest {
                path: path.clone(),
                source,
            })?
            .len();
        manifests.push(BundleManifest {
            path: (*name).to_owned(),
            kind: manifest_kind(name).to_owned(),
            bytes,
            preview,
        });
    }
    Ok(manifests)
}

fn collect_git(root: &Utf8PathBuf) -> Result<Option<BundleGit>, BundleError> {
    let Some(repo) = axt_git::repo_root_for(root)? else {
        return Ok(None);
    };
    let dirty = axt_git::dirty_count(&repo)?;
    Ok(Some(BundleGit {
        root: repo.root().to_string(),
        branch: axt_git::current_branch(&repo)?,
        modified: dirty.modified,
        untracked: dirty.untracked,
    }))
}

fn next_hints(root: &str, git: Option<&BundleGit>, has_manifest: bool) -> Vec<String> {
    let mut next = vec![
        format!("axt-peek {root} --agent"),
        format!("axt-outline {root} --agent"),
    ];
    if git.is_some_and(|state| state.modified > 0 || state.untracked > 0) {
        next.push(format!("axt-peek {root} --changed --agent"));
    }
    if has_manifest {
        next.push("axt-test --agent".to_owned());
    }
    next
}

impl Renderable for BundleData {
    fn render_human(&self, w: &mut dyn Write, _ctx: &RenderContext<'_>) -> RenderResult<()> {
        writeln!(
            w,
            "root={} files={} dirs={} manifests={} git={} truncated={}",
            self.root,
            self.summary.files,
            self.summary.dirs,
            self.summary.manifests,
            self.summary.git,
            self.summary.truncated
        )?;
        for manifest in &self.manifests {
            writeln!(w, "manifest {} {}B", manifest.path, manifest.bytes)?;
        }
        if let Some(git) = &self.git {
            writeln!(
                w,
                "git branch={} modified={} untracked={}",
                git.branch.as_deref().unwrap_or("detached"),
                git.modified,
                git.untracked
            )?;
        }
        Ok(())
    }

    fn render_compact(&self, w: &mut dyn Write, _ctx: &RenderContext<'_>) -> RenderResult<()> {
        writeln!(
            w,
            "bundle root={} files={} dirs={} manifests={} git={} truncated={}",
            self.root,
            self.summary.files,
            self.summary.dirs,
            self.summary.manifests,
            self.summary.git,
            self.summary.truncated
        )?;
        for manifest in &self.manifests {
            writeln!(
                w,
                "manifest path={} kind={} bytes={}",
                manifest.path, manifest.kind, manifest.bytes
            )?;
        }
        if let Some(git) = &self.git {
            writeln!(
                w,
                "git branch={} modified={} untracked={} root={}",
                git.branch.as_deref().unwrap_or("detached"),
                git.modified,
                git.untracked,
                git.root
            )?;
        }
        for file in &self.files {
            writeln!(
                w,
                "file path={} kind={} bytes={} lang={}",
                file.path,
                file.kind,
                file.bytes,
                file.lang.as_deref().unwrap_or("-")
            )?;
        }
        Ok(())
    }

    fn render_json(&self, w: &mut dyn Write, _ctx: &RenderContext<'_>) -> RenderResult<()> {
        let envelope = JsonEnvelope::new("axt.bundle.v1", self, Vec::new(), Vec::new());
        serde_json::to_writer(&mut *w, &envelope)?;
        writeln!(w)?;
        Ok(())
    }

    fn render_agent(&self, w: &mut dyn Write, ctx: &RenderContext<'_>) -> RenderResult<()> {
        let mut writer = AgentJsonlWriter::new(w, ctx.limits);
        writer.write_record(&AgentSummary {
            schema: "axt.bundle.summary.v1",
            kind: "summary",
            ok: true,
            root: &self.root,
            files: self.summary.files,
            dirs: self.summary.dirs,
            manifests: self.summary.manifests,
            git: self.summary.git,
            truncated: self.summary.truncated,
            next: &self.next,
        })?;
        for manifest in &self.manifests {
            writer.write_record(&serde_json::json!({
                "schema": "axt.bundle.manifest.v1",
                "type": "manifest",
                "p": manifest.path,
                "k": manifest.kind,
                "b": manifest.bytes,
                "preview": manifest.preview,
            }))?;
        }
        if let Some(git) = &self.git {
            writer.write_record(&serde_json::json!({
                "schema": "axt.bundle.git.v1",
                "type": "git",
                "root": git.root,
                "branch": git.branch,
                "modified": git.modified,
                "untracked": git.untracked,
            }))?;
        }
        for file in &self.files {
            writer.write_record(&serde_json::json!({
                "schema": "axt.bundle.file.v1",
                "type": "file",
                "p": file.path,
                "k": file.kind,
                "b": file.bytes,
                "l": file.lang,
            }))?;
        }
        let _summary = writer.finish("axt.bundle.warn.v1")?;
        Ok(())
    }
}

fn print_schema(format: SchemaFormat) {
    match format {
        SchemaFormat::Json => {
            print!("{}", include_str!("../../../schemas/axt.bundle.v1.schema.json"));
        }
        SchemaFormat::Agent => println!(
            "schema=axt.bundle.agent.v1 records=axt.bundle.summary.v1,axt.bundle.manifest.v1,axt.bundle.git.v1,axt.bundle.file.v1,axt.bundle.warn.v1 first=summary"
        ),
        SchemaFormat::Compact => println!(
            "schema=axt.bundle.compact.v1 format=text records=summary,manifest,git,file default=non-tty"
        ),
        SchemaFormat::Human => {
            println!("schema=axt.bundle.human.v1 sections=summary,manifests,git");
        }
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

fn exit_code_for_error(err: &BundleError) -> u8 {
    match err {
        BundleError::PathNotFound(_) => ErrorCode::PathNotFound.exit_code(),
        BundleError::Fs(
            axt_fs::FsError::Metadata { source, .. } | axt_fs::FsError::Read { source, .. },
        )
        | BundleError::ReadManifest { source, .. }
            if source.kind() == std::io::ErrorKind::PermissionDenied =>
        {
            ErrorCode::PermissionDenied.exit_code()
        }
        BundleError::Git(_) => ErrorCode::GitUnavailable.exit_code(),
        BundleError::Fs(_) | BundleError::ReadManifest { .. } => ErrorCode::IoError.exit_code(),
    }
}

const fn entry_kind(kind: EntryKind) -> &'static str {
    match kind {
        EntryKind::File => "file",
        EntryKind::Dir => "dir",
        EntryKind::Symlink => "symlink",
        EntryKind::Other => "other",
    }
}

fn manifest_kind(name: &str) -> &'static str {
    match name {
        "Cargo.toml" => "rust",
        "package.json" | "package-lock.json" | "pnpm-lock.yaml" | "bun.lock" | "deno.json" => "js",
        "pyproject.toml" => "python",
        "go.mod" => "go",
        _ => "manifest",
    }
}
