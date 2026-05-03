mod cli;
mod command;
mod error;
mod model;
mod output;
mod render;
mod snapshot;

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
            return Ok(ExitCode::from(exit_code_for_drift_error(&err)));
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
            if output.ok() {
                Ok(ExitCode::SUCCESS)
            } else {
                Ok(ExitCode::from(ErrorCode::CommandFailed.exit_code()))
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
        SchemaFormat::Json => {
            print!("{}", include_str!("../../../schemas/axt.drift.v1.schema.json"));
        }
        SchemaFormat::Agent => println!(
            "schema=axt.drift.agent.v1 records=axt.drift.summary.v1,axt.drift.file.v1,axt.drift.mark.v1,axt.drift.warn.v1 first=summary"
        ),
        SchemaFormat::Human => {
            println!("schema=axt.drift.human.v1 sections=mark,diff,run,list,reset");
        }
    }
}

fn exit_code_for_drift_error(err: &crate::error::DriftError) -> u8 {
    match err {
        crate::error::DriftError::MissingSubcommand
        | crate::error::DriftError::MissingCommand
        | crate::error::DriftError::InvalidName(_) => ErrorCode::UsageError.exit_code(),
        crate::error::DriftError::MarkNotFound(_) => ErrorCode::PathNotFound.exit_code(),
        crate::error::DriftError::Io { source, .. }
            if source.kind() == std::io::ErrorKind::PermissionDenied =>
        {
            ErrorCode::PermissionDenied.exit_code()
        }
        crate::error::DriftError::Timeout { .. } => ErrorCode::Timeout.exit_code(),
        crate::error::DriftError::Io { .. }
        | crate::error::DriftError::PathNotUtf8(_)
        | crate::error::DriftError::SnapshotParse { .. }
        | crate::error::DriftError::Serialize(_) => ErrorCode::IoError.exit_code(),
        crate::error::DriftError::Execute(_) => ErrorCode::RuntimeError.exit_code(),
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
