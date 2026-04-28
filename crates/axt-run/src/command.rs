use std::fs;

use axt_core::CommandContext;
use camino::{Utf8Path, Utf8PathBuf};

use crate::{
    cli::{Args, Command},
    error::{Result, RunError},
    execute::{run_command, RunRequest},
    output::{ListRun, RunOutput},
    storage,
};

pub async fn run(args: &Args, ctx: &CommandContext) -> Result<RunOutput> {
    match &args.subcommand {
        Some(Command::Show(show)) => show_run(&ctx.cwd, &show.name, show.stdout, show.stderr),
        Some(Command::List) => list_runs(&ctx.cwd),
        Some(Command::Clean(clean)) => {
            let removed = storage::clean(&ctx.cwd, clean.older_than.map(|duration| duration.0))?;
            Ok(RunOutput::Clean { removed })
        }
        None => run_command(RunRequest {
            args: &args.run,
            base_cwd: &ctx.cwd,
        })
        .await
        .map(RunOutput::Run),
    }
}

fn show_run(root: &Utf8Path, name: &str, stdout: bool, stderr: bool) -> Result<RunOutput> {
    let (dir, run) = storage::read_saved(root, name)?;
    if stdout || stderr {
        let stream = if stderr { "stderr" } else { "stdout" };
        let path = dir.join(format!("{stream}.log"));
        let text = fs::read_to_string(&path).map_err(|source| RunError::Io {
            path: path.clone(),
            source,
        })?;
        return Ok(RunOutput::Stream {
            name: run
                .data
                .saved
                .as_ref()
                .map_or_else(|| name.to_owned(), |saved| saved.name.clone()),
            stream: stream.to_owned(),
            text,
        });
    }
    Ok(RunOutput::Show(run.data))
}

fn list_runs(root: &Utf8Path) -> Result<RunOutput> {
    let runs = storage::list_saved(root)?
        .into_iter()
        .map(|(dir, run)| ListRun {
            name: dir_name(&dir),
            path: dir.to_string(),
            created_at: run.created_at,
            ok: run.data.ok(),
            exit: run.data.exit,
            duration_ms: run.data.duration_ms,
            command: command_string(&run.data.command.program, &run.data.command.args),
        })
        .collect();
    Ok(RunOutput::List { runs })
}

fn dir_name(dir: &Utf8PathBuf) -> String {
    dir.file_name()
        .map_or_else(|| dir.to_string(), ToOwned::to_owned)
}

fn command_string(program: &str, args: &[String]) -> String {
    let mut parts = Vec::with_capacity(1 + args.len());
    parts.push(program);
    parts.extend(args.iter().map(String::as_str));
    parts.join(" ")
}
