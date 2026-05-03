use std::time::Instant;

use axt_core::CommandContext;
use tokio::process::Command as TokioCommand;

use crate::{
    cli::{Args, Command},
    error::{DriftError, Result},
    model::{DriftData, DriftOperation, RunCommand},
    output::DriftOutput,
    snapshot::{
        diff_captured_snapshots, diff_snapshot_files, list_marks, mark_path, reset_marks,
        CapturedSnapshot,
    },
};

pub async fn run(args: &Args, ctx: &CommandContext) -> Result<DriftOutput> {
    match args.command.as_ref().ok_or(DriftError::MissingSubcommand)? {
        Command::Mark(mark_args) => {
            mark_snapshot(&ctx.cwd, &mark_args.name, mark_args.hash).map(DriftOutput::Mark)
        }
        Command::Diff(diff_args) => {
            diff_snapshot(&ctx.cwd, &diff_args.since, diff_args.hash).map(DriftOutput::Diff)
        }
        Command::Run(run_args) => run_command(
            &ctx.cwd,
            &run_args.name,
            run_args.hash,
            &run_args.command,
            ctx.max_duration,
        )
        .await
        .map(DriftOutput::Run),
        Command::List => list(&ctx.cwd).map(DriftOutput::List),
        Command::Reset => reset(&ctx.cwd).map(DriftOutput::Reset),
    }
}

fn mark_snapshot(root: &camino::Utf8Path, name: &str, hash: bool) -> Result<DriftData> {
    let snapshot = CapturedSnapshot::capture(root, hash)?;
    let path = mark_path(root, name)?;
    snapshot.persist_to(&path)?;
    Ok(base_data(
        DriftOperation::Mark,
        hash,
        Some(name),
        Some(&path),
        snapshot.len(),
    ))
}

fn diff_snapshot(root: &camino::Utf8Path, name: &str, hash: bool) -> Result<DriftData> {
    let path = mark_path(root, name)?;
    if !path.exists() {
        return Err(DriftError::MarkNotFound(name.to_owned()));
    }
    let after = CapturedSnapshot::capture(root, hash)?;
    let mut data = base_data(
        DriftOperation::Diff,
        hash,
        Some(name),
        Some(&path),
        after.len(),
    );
    data.changes = diff_snapshot_files(&path, &after)?;
    Ok(data)
}

async fn run_command(
    root: &camino::Utf8Path,
    name: &str,
    hash: bool,
    command: &[String],
    max_duration: Option<std::time::Duration>,
) -> Result<DriftData> {
    let (program, args) = command.split_first().ok_or(DriftError::MissingCommand)?;
    let before = CapturedSnapshot::capture(root, hash)?;
    let started = Instant::now();
    let mut child = TokioCommand::new(program)
        .args(args)
        .current_dir(root)
        .spawn()
        .map_err(DriftError::Execute)?;
    let status = if let Some(max_duration) = max_duration {
        match tokio::time::timeout(max_duration, child.wait()).await {
            Ok(status) => status.map_err(DriftError::Execute)?,
            Err(_elapsed) => {
                let _kill_result = child.kill().await;
                let _wait_result = child.wait().await;
                return Err(DriftError::Timeout {
                    duration_ms: u64::try_from(max_duration.as_millis()).unwrap_or(u64::MAX),
                });
            }
        }
    } else {
        child.wait().await.map_err(DriftError::Execute)?
    };
    let duration_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
    let after = CapturedSnapshot::capture(root, hash)?;
    let path = mark_path(root, name)?;
    before.persist_to(&path)?;
    let mut data = base_data(
        DriftOperation::Run,
        hash,
        Some(name),
        Some(&path),
        after.len(),
    );
    data.changes = diff_captured_snapshots(&before, &after)?;
    data.command = Some(RunCommand {
        program: program.clone(),
        args: args.to_vec(),
    });
    data.exit = status.code();
    data.duration_ms = Some(duration_ms);
    Ok(data)
}

fn list(root: &camino::Utf8Path) -> Result<DriftData> {
    let marks = list_marks(root)?;
    let mut data = base_data(DriftOperation::List, false, None, None, 0);
    data.files = marks.iter().map(|mark| mark.files).sum();
    data.marks = marks;
    Ok(data)
}

fn reset(root: &camino::Utf8Path) -> Result<DriftData> {
    let removed = reset_marks(root)?;
    let mut data = base_data(DriftOperation::Reset, false, None, None, 0);
    data.removed = removed;
    Ok(data)
}

fn base_data(
    operation: DriftOperation,
    hash: bool,
    name: Option<&str>,
    mark_path: Option<&camino::Utf8Path>,
    files: usize,
) -> DriftData {
    DriftData {
        operation,
        name: name.map(ToOwned::to_owned),
        mark_path: mark_path.map(ToString::to_string),
        hash,
        files,
        changes: Vec::new(),
        marks: Vec::new(),
        removed: 0,
        command: None,
        exit: None,
        duration_ms: None,
    }
}
