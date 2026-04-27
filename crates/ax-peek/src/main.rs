use clap::{ArgGroup, Parser};
use serde_json::json;

#[derive(Debug, Parser)]
#[command(name = "ax-peek")]
#[command(about = "Directory and repository snapshot command.")]
#[command(group(ArgGroup::new("mode").args(["json", "agent", "plain", "json_data"]).multiple(false)))]
struct Args {
    /// Emit a JSON envelope.
    #[arg(long)]
    json: bool,

    /// Emit agent-oriented NDJSON.
    #[arg(long)]
    agent: bool,

    /// Emit plain human-readable output.
    #[arg(long)]
    plain: bool,

    /// Emit only the JSON data payload.
    #[arg(long)]
    json_data: bool,

    /// Print the command schema placeholder.
    #[arg(long)]
    print_schema: bool,

    /// List standard error codes as NDJSON.
    #[arg(long)]
    list_errors: bool,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    if args.print_schema {
        println!(
            "{}",
            json!({
                "schema": "ax.peek.v1",
                "status": "stub"
            })
        );
        return Ok(());
    }

    if args.list_errors {
        for (code, exit) in ERROR_CODES {
            println!("{}", json!({ "code": code, "exit": exit }));
        }
        return Ok(());
    }

    if args.agent {
        println!(
            "{}",
            json!({
                "s": "ax.peek.summary.v1",
                "t": "summary",
                "ok": true,
                "stub": true
            })
        );
    } else if args.json {
        println!(
            "{}",
            json!({
                "schema": "ax.peek.v1",
                "ok": true,
                "data": {
                    "status": "stub"
                },
                "warnings": [],
                "errors": []
            })
        );
    } else if args.json_data {
        println!("{}", json!({ "status": "stub" }));
    } else {
        println!("ax-peek stub: Milestone 0 scaffolding only");
    }

    Ok(())
}

const ERROR_CODES: &[(&str, u8)] = &[
    ("ok", 0),
    ("runtime_error", 1),
    ("usage_error", 2),
    ("path_not_found", 3),
    ("permission_denied", 4),
    ("timeout", 5),
    ("output_truncated_strict", 6),
    ("interrupted", 7),
    ("io_error", 8),
    ("feature_unsupported", 9),
    ("schema_violation", 10),
    ("command_failed", 11),
    ("git_unavailable", 12),
    ("config_error", 13),
    ("network_disabled", 14),
];
