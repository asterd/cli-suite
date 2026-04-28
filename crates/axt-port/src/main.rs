mod cli;
mod command;
mod discovery;
mod error;
mod model;
mod output;
mod render;
mod signal;

use std::{io::Write, net::TcpListener, process::ExitCode, time::Duration};

use axt_core::{ErrorCatalogEntry, ErrorCode, OutputMode, SchemaFormat, STANDARD_ERROR_CATALOG};
use axt_output::Renderable;
use clap::Parser;

use crate::{cli::Args, command::run};

#[tokio::main]
async fn main() -> anyhow::Result<ExitCode> {
    if std::env::var_os("AXT_PORT_LISTENER_FIXTURE").is_some() {
        return listener_fixture();
    }

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
            return Ok(ExitCode::from(exit_code_for_port_error(&err)));
        }
    };
    let render_ctx =
        axt_output::RenderContext::new(mode, ctx.limits, ctx.color, ctx.clock.as_ref());
    let mut stdout = std::io::stdout().lock();
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

fn listener_fixture() -> anyhow::Result<ExitCode> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    println!("{}", listener.local_addr()?.port());
    loop {
        std::thread::sleep(Duration::from_secs(60));
    }
}

fn print_schema(format: SchemaFormat) {
    match format {
        SchemaFormat::Json => {
            print!("{}", include_str!("../../../schemas/axt.port.v1.schema.json"));
        }
        SchemaFormat::Jsonl => println!(
            "schema=axt.port.jsonl.v1 records=axt.port.summary.v1,axt.port.socket.v1,axt.port.holder.v1,axt.port.action.v1,axt.port.warn.v1 first=summary"
        ),
        SchemaFormat::Agent => println!(
            "schema=axt.port.agent.v1 mode=records prefixes=H,A,X fields=action,port,held,holders,freed,timed_out,pid,name,cmd,cwd,bound,owner,mem,signal,result,ms"
        ),
        SchemaFormat::Human => {
            println!("schema=axt.port.human.v1 sections=list,who,free,watch");
        }
    }
}

fn exit_code_for_port_error(err: &crate::error::PortError) -> u8 {
    match err {
        crate::error::PortError::MissingSubcommand => ErrorCode::UsageError.exit_code(),
        crate::error::PortError::Inspect(_) => ErrorCode::FeatureUnsupported.exit_code(),
        crate::error::PortError::Command { source, .. }
            if source.kind() == std::io::ErrorKind::PermissionDenied =>
        {
            ErrorCode::PermissionDenied.exit_code()
        }
        crate::error::PortError::Command { .. } => ErrorCode::RuntimeError.exit_code(),
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
