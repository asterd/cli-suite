mod cli;
mod command;
mod error;
mod execute;
mod fswatch;
mod model;
mod output;
mod render;
mod storage;

use std::{io::Write, process::ExitCode};

use axt_core::{ErrorCatalogEntry, ErrorCode, OutputMode, SchemaFormat, STANDARD_ERROR_CATALOG};
use axt_output::Renderable;
use clap::Parser;

use crate::{cli::Args, command::run};

#[tokio::main]
async fn main() -> anyhow::Result<ExitCode> {
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
    let output = match run(&args, &ctx).await {
        Ok(output) => output,
        Err(err) => {
            eprintln!("Error: {err}");
            return Ok(ExitCode::from(exit_code_for_run_error(&err)));
        }
    };
    let render_ctx =
        axt_output::RenderContext::new(mode, ctx.limits, ctx.color, ctx.clock.as_ref());
    let mut stdout = std::io::stdout().lock();

    let result = match mode {
        OutputMode::Human => output.render_human(&mut stdout, &render_ctx),
        OutputMode::Json => output.render_json(&mut stdout, &render_ctx),
        OutputMode::Agent => output.render_agent(&mut stdout, &render_ctx),
    };

    match result {
        Ok(()) => {
            if exit_code_for_output(&output) == ErrorCode::Ok {
                Ok(ExitCode::SUCCESS)
            } else {
                Ok(ExitCode::from(exit_code_for_output(&output).exit_code()))
            }
        }
        Err(axt_output::OutputError::TruncatedStrict) => {
            Ok(ExitCode::from(ErrorCode::OutputTruncatedStrict.exit_code()))
        }
        Err(err) => Err(err.into()),
    }
}

fn print_schema(format: SchemaFormat) {
    match format {
        SchemaFormat::Json => print!("{}", include_str!("../../../schemas/axt.run.v1.schema.json")),
        SchemaFormat::Agent => println!(
            "schema=axt.run.agent.v1 records=axt.run.summary.v1,axt.run.file.v1,axt.run.stream.v1,axt.run.list.v1,axt.run.clean.v1,axt.run.warn.v1 first=summary"
        ),
        SchemaFormat::Human => {
            println!("schema=axt.run.human.v1 sections=summary,stderr_tail,changed_files,saved")
        }
    }
}

fn exit_code_for_output(output: &crate::output::RunOutput) -> ErrorCode {
    match output {
        crate::output::RunOutput::Run(data) | crate::output::RunOutput::Show(data) => {
            if data.timed_out {
                ErrorCode::Timeout
            } else if data.exit == Some(0) {
                ErrorCode::Ok
            } else {
                ErrorCode::CommandFailed
            }
        }
        crate::output::RunOutput::Stream { .. }
        | crate::output::RunOutput::List { .. }
        | crate::output::RunOutput::Clean { .. } => ErrorCode::Ok,
    }
}

fn exit_code_for_run_error(err: &crate::error::RunError) -> u8 {
    match err {
        crate::error::RunError::MissingCommand
        | crate::error::RunError::InvalidEnv(_)
        | crate::error::RunError::EnvFileParse { .. }
        | crate::error::RunError::Glob { .. } => ErrorCode::UsageError.exit_code(),
        crate::error::RunError::PathNotFound(_) | crate::error::RunError::RunNotFound(_) => {
            ErrorCode::PathNotFound.exit_code()
        }
        crate::error::RunError::Io { source, .. }
        | crate::error::RunError::EnvFileRead { source, .. }
            if source.kind() == std::io::ErrorKind::PermissionDenied =>
        {
            ErrorCode::PermissionDenied.exit_code()
        }
        crate::error::RunError::Io { .. }
        | crate::error::RunError::EnvFileRead { .. }
        | crate::error::RunError::PathNotUtf8(_)
        | crate::error::RunError::Serialize(_) => ErrorCode::IoError.exit_code(),
        crate::error::RunError::Execute(_) | crate::error::RunError::Render(_) => {
            ErrorCode::RuntimeError.exit_code()
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
