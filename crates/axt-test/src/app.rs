mod cli;
mod command;
mod discovery;
mod error;
mod frontend;
mod model;
mod render;

use std::{io::Write, process::ExitCode};

use axt_core::{ErrorCatalogEntry, ErrorCode, OutputMode, SchemaFormat, STANDARD_ERROR_CATALOG};
use axt_output::Renderable;
use clap::Parser;

use crate::{cli::Args, command::run, error::TestError};

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
    let mut stdout = std::io::stdout().lock();
    if mode == OutputMode::Jsonl && args.command.is_none() {
        return match command::run_jsonl_streaming(&args, &ctx, &mut stdout, &render_ctx) {
            Ok(true) => Ok(ExitCode::SUCCESS),
            Ok(false) => Ok(ExitCode::from(ErrorCode::CommandFailed.exit_code())),
            Err(TestError::Output(axt_output::OutputError::TruncatedStrict)) => {
                Ok(ExitCode::from(ErrorCode::OutputTruncatedStrict.exit_code()))
            }
            Err(err) => {
                eprintln!("Error: {err}");
                Ok(ExitCode::from(exit_code_for_test_error(&err)))
            }
        };
    }

    let output = match run(&args, &ctx) {
        Ok(output) => output,
        Err(err) => {
            eprintln!("Error: {err}");
            return Ok(ExitCode::from(exit_code_for_test_error(&err)));
        }
    };
    let result = match mode {
        OutputMode::Human | OutputMode::Plain => output.render_human(&mut stdout, &render_ctx),
        OutputMode::Json => output.render_json(&mut stdout, &render_ctx),
        OutputMode::JsonData => output.render_json_data(&mut stdout),
        OutputMode::Jsonl => output.render_jsonl(&mut stdout, &render_ctx),
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
            print!("{}", include_str!("../../../schemas/axt.test.v1.schema.json"));
        }
        SchemaFormat::Jsonl => println!(
            "schema=axt.test.jsonl.v1 records=axt.test.summary.v1,axt.test.suite.v1,axt.test.case.v1,axt.test.framework.v1,axt.test.warn.v1"
        ),
        SchemaFormat::Agent => println!(
            "schema=axt.test.agent.v1 mode=records prefixes=U,C,S fields=frameworks,total,passed,failed,skipped,todo,ms,started,name,file,line,message"
        ),
        SchemaFormat::Human => {
            println!("schema=axt.test.human.v1 sections=summary,failures,frameworks");
        }
    }
}

fn exit_code_for_test_error(err: &TestError) -> u8 {
    match err {
        TestError::NoFramework | TestError::MultipleFrameworks => ErrorCode::UsageError.exit_code(),
        TestError::MissingTool { .. } => ErrorCode::FeatureUnsupported.exit_code(),
        TestError::Command { source, .. }
            if source.kind() == std::io::ErrorKind::PermissionDenied =>
        {
            ErrorCode::PermissionDenied.exit_code()
        }
        TestError::Output(axt_output::OutputError::TruncatedStrict) => {
            ErrorCode::OutputTruncatedStrict.exit_code()
        }
        TestError::Output(_) | TestError::Io(_) => ErrorCode::IoError.exit_code(),
        TestError::GitUnavailable | TestError::Git(_) => ErrorCode::GitUnavailable.exit_code(),
        TestError::Command { .. } => ErrorCode::CommandFailed.exit_code(),
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
