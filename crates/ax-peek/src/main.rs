use std::{io::Write, process::ExitCode};

use ax_core::{
    CommonArgs, ErrorCatalogEntry, ErrorCode, OutputMode, SystemClock, STANDARD_ERROR_CATALOG,
};
use ax_output::{
    format_agent_fields, AgentCompactWriter, AgentField, JsonEnvelope, JsonlWriter, RenderContext,
    Renderable, Result as RenderResult,
};
use clap::Parser;
use serde::Serialize;

#[derive(Debug, Parser)]
#[command(name = "ax-peek")]
#[command(about = "Directory and repository snapshot command.")]
struct Args {
    #[command(flatten)]
    common: CommonArgs,
}

fn main() -> anyhow::Result<ExitCode> {
    let args = Args::parse();

    if args.common.print_schema {
        print!(
            "{}",
            include_str!("../../../schemas/ax.peek.v1.schema.json")
        );
        return Ok(ExitCode::SUCCESS);
    }

    if args.common.list_errors {
        write_error_catalog(std::io::stdout().lock(), STANDARD_ERROR_CATALOG)?;
        return Ok(ExitCode::SUCCESS);
    }

    let clock = SystemClock;
    let mode = args.common.mode()?;
    let ctx = RenderContext::new(
        mode,
        args.common.limits(),
        ax_core::resolve_color_choice(ax_core::stdout_is_terminal()),
        &clock,
    );
    let stub = PeekStub;
    let mut stdout = std::io::stdout().lock();

    match mode {
        OutputMode::Human | OutputMode::Plain => stub.render_human(&mut stdout, &ctx)?,
        OutputMode::Json => stub.render_json(&mut stdout, &ctx)?,
        OutputMode::JsonData => write_json_data(&mut stdout)?,
        OutputMode::Jsonl => {
            if let Err(err) = stub.render_jsonl(&mut stdout, &ctx) {
                if matches!(err, ax_output::OutputError::TruncatedStrict) {
                    return Ok(ExitCode::from(ErrorCode::OutputTruncatedStrict.exit_code()));
                }
                return Err(err.into());
            }
        }
        OutputMode::Agent => {
            if let Err(err) = stub.render_agent(&mut stdout, &ctx) {
                if matches!(err, ax_output::OutputError::TruncatedStrict) {
                    return Ok(ExitCode::from(ErrorCode::OutputTruncatedStrict.exit_code()));
                }
                return Err(err.into());
            }
        }
    }

    Ok(ExitCode::SUCCESS)
}

#[derive(Debug, Serialize)]
struct PeekData {
    status: &'static str,
}

#[derive(Debug, Serialize)]
struct PeekSummary {
    schema: &'static str,
    #[serde(rename = "type")]
    kind: &'static str,
    ok: bool,
    stub: bool,
}

#[derive(Debug)]
struct PeekStub;

impl Renderable for PeekStub {
    fn render_human(&self, w: &mut dyn Write, _ctx: &RenderContext<'_>) -> RenderResult<()> {
        writeln!(w, "ax-peek stub: Milestone 0 scaffolding only")?;
        Ok(())
    }

    fn render_json(&self, w: &mut dyn Write, _ctx: &RenderContext<'_>) -> RenderResult<()> {
        let envelope = JsonEnvelope::new(
            "ax.peek.v1",
            PeekData { status: "stub" },
            Vec::new(),
            Vec::new(),
        );
        serde_json::to_writer(&mut *w, &envelope)?;
        writeln!(w)?;
        Ok(())
    }

    fn render_jsonl(&self, w: &mut dyn Write, ctx: &RenderContext<'_>) -> RenderResult<()> {
        let mut writer = JsonlWriter::new(w, ctx.limits);
        let summary = PeekSummary {
            schema: "ax.peek.summary.v1",
            kind: "summary",
            ok: true,
            stub: true,
        };
        let _written = writer.write_record(&summary)?;
        let _summary = writer.finish("ax.peek.warn.v1")?;
        Ok(())
    }

    fn render_agent(&self, w: &mut dyn Write, ctx: &RenderContext<'_>) -> RenderResult<()> {
        let mut writer = AgentCompactWriter::new(w, ctx.limits);
        let mut fields = [
            AgentField::str("schema", "ax.peek.agent.v1"),
            AgentField::bool("ok", true),
            AgentField::str("mode", "records"),
            AgentField::bool("stub", true),
            AgentField::bool("truncated", false),
        ];
        let probe_line = format_agent_fields(&fields)?;
        let line = if ctx.limits.max_records == 0 || probe_line.len() + 1 > ctx.limits.max_bytes {
            fields[4] = AgentField::bool("truncated", true);
            format_agent_fields(&fields)?
        } else {
            probe_line
        };
        let _written = writer.write_line(&line)?;
        let _summary = writer.finish()?;
        Ok(())
    }
}

fn write_json_data(w: &mut dyn Write) -> RenderResult<()> {
    serde_json::to_writer(&mut *w, &PeekData { status: "stub" })?;
    writeln!(w)?;
    Ok(())
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
