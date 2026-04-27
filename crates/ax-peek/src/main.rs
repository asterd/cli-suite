mod cli;
mod collect;
mod command;
mod error;
mod model;
mod render;

use std::{io::Write, process::ExitCode};

use ax_core::{ErrorCatalogEntry, ErrorCode, OutputMode, SchemaFormat, STANDARD_ERROR_CATALOG};
use ax_output::Renderable;
use clap::Parser;

use crate::{cli::Args, command::run};

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
        ax_core::CommandContext::from_common_args(&args.common, Box::new(ax_core::SystemClock))?;
    let data = match run(&args, &ctx) {
        Ok(data) => data,
        Err(err) => {
            eprintln!("Error: {err}");
            return Ok(ExitCode::from(exit_code_for_peek_error(&err)));
        }
    };
    let render_ctx = ax_output::RenderContext::new(mode, ctx.limits, ctx.color, ctx.clock.as_ref());
    let mut stdout = std::io::stdout().lock();

    let result = match mode {
        OutputMode::Human | OutputMode::Plain => data.render_human(&mut stdout, &render_ctx),
        OutputMode::Json => data.render_json(&mut stdout, &render_ctx),
        OutputMode::JsonData => data.render_json_data(&mut stdout),
        OutputMode::Jsonl => data.render_jsonl(&mut stdout, &render_ctx),
        OutputMode::Agent => data.render_agent(&mut stdout, &render_ctx),
    };

    match result {
        Ok(()) => Ok(ExitCode::SUCCESS),
        Err(ax_output::OutputError::TruncatedStrict) => {
            Ok(ExitCode::from(ErrorCode::OutputTruncatedStrict.exit_code()))
        }
        Err(err) => Err(err.into()),
    }
}

fn print_schema(format: SchemaFormat) {
    match format {
        SchemaFormat::Json => print!("{}", include_str!("../../../schemas/ax.peek.v1.schema.json")),
        SchemaFormat::Jsonl => println!(
            "schema=ax.peek.jsonl.v1 records=ax.peek.summary.v1,ax.peek.entry.v1,ax.peek.warn.v1 first=summary"
        ),
        SchemaFormat::Agent => println!(
            "schema=ax.peek.agent.v1 mode=table cols=path,kind,bytes,lang,git,mtime warnings=W"
        ),
        SchemaFormat::Human => println!(
            "schema=ax.peek.human.v1 sections=tree,summary columns=path,bytes,lang,git"
        ),
    }
}

fn exit_code_for_peek_error(err: &crate::error::PeekError) -> u8 {
    match err {
        crate::error::PeekError::PathNotFound(_) => ErrorCode::PathNotFound.exit_code(),
        crate::error::PeekError::Git(_) => ErrorCode::GitUnavailable.exit_code(),
        crate::error::PeekError::Fs(ax_fs::FsError::Metadata { source, .. })
        | crate::error::PeekError::Fs(ax_fs::FsError::Read { source, .. })
            if source.kind() == std::io::ErrorKind::PermissionDenied =>
        {
            ErrorCode::PermissionDenied.exit_code()
        }
        crate::error::PeekError::Fs(_) | crate::error::PeekError::Canonicalize { .. } => {
            ErrorCode::IoError.exit_code()
        }
        crate::error::PeekError::CanonicalPathNotUtf8(_)
        | crate::error::PeekError::TimestampFormat(_)
        | crate::error::PeekError::TimestampRange(_) => ErrorCode::RuntimeError.exit_code(),
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
