mod cli;
mod command;
mod error;
mod model;
mod render;
mod tree;

use std::{io::Write, process::ExitCode};

use axt_core::{ErrorCatalogEntry, ErrorCode, OutputMode, SchemaFormat, STANDARD_ERROR_CATALOG};
use axt_output::Renderable;
use clap::Parser;

use crate::{cli::Args, command::run, error::OutlineError};

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
    let render_ctx =
        axt_output::RenderContext::new(mode, ctx.limits, ctx.color, ctx.clock.as_ref());
    let output = match run(&args, &ctx) {
        Ok(output) => output,
        Err(err) => {
            eprintln!("Error: {err}");
            return Ok(ExitCode::from(exit_code_for_outline_error(&err)));
        }
    };

    let mut stdout = std::io::stdout().lock();
    let result = match mode {
        OutputMode::Human => output.render_human(&mut stdout, &render_ctx),
        OutputMode::Json => output.render_json(&mut stdout, &render_ctx),
        OutputMode::Agent => output.render_agent(&mut stdout, &render_ctx),
    };

    match result {
        Ok(()) => Ok(ExitCode::SUCCESS),
        Err(axt_output::OutputError::TruncatedStrict) => {
            Ok(ExitCode::from(ErrorCode::OutputTruncatedStrict.exit_code()))
        }
        Err(err) => Err(err.into()),
    }
}

fn print_schema(format: SchemaFormat) {
    match format {
        SchemaFormat::Json => {
            print!("{}", include_str!("../../../schemas/axt.outline.v1.schema.json"));
        }
        SchemaFormat::Agent => println!(
            "schema=axt.outline.agent.v1 records=axt.outline.summary.v1,axt.outline.symbol.v1,axt.outline.warn.v1 first=summary"
        ),
        SchemaFormat::Human => {
            println!("schema=axt.outline.human.v1 sections=summary,symbols,warnings");
        }
    }
}

fn exit_code_for_outline_error(err: &OutlineError) -> u8 {
    match err {
        OutlineError::PathNotFound(_) => ErrorCode::PathNotFound.exit_code(),
        OutlineError::NoSupportedFiles => ErrorCode::FeatureUnsupported.exit_code(),
        OutlineError::Output(axt_output::OutputError::TruncatedStrict) => {
            ErrorCode::OutputTruncatedStrict.exit_code()
        }
        OutlineError::Io { source, .. } if source.kind() == std::io::ErrorKind::PermissionDenied => {
            ErrorCode::PermissionDenied.exit_code()
        }
        OutlineError::Walk(_) | OutlineError::Io { .. } | OutlineError::Output(_) => {
            ErrorCode::IoError.exit_code()
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
