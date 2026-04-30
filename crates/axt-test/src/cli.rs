use std::{str::FromStr, time::Duration};

use camino::Utf8PathBuf;
use clap::{Args as ClapArgs, Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(name = "axt-test")]
#[command(about = "Run project tests and emit normalized output.")]
#[command(version)]
pub struct Args {
    #[command(flatten)]
    pub common: axt_core::CommonArgs,

    #[command(flatten)]
    pub run: RunArgs,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    ListFrameworks,
}

#[derive(Debug, Clone, ClapArgs)]
pub struct RunArgs {
    #[arg(long, value_enum)]
    pub framework: Option<FrameworkArg>,

    #[arg(long, value_name = "PATTERN")]
    pub filter: Option<String>,

    #[arg(long, value_name = "PATH")]
    pub files: Vec<Utf8PathBuf>,

    #[arg(long)]
    pub changed: bool,

    #[arg(long, value_name = "REF")]
    pub changed_since: Option<String>,

    #[arg(long)]
    pub single: bool,

    #[arg(long)]
    pub bail: bool,

    #[arg(long, value_name = "N")]
    pub workers: Option<usize>,

    #[arg(long, default_value_t = 5, value_name = "N")]
    pub top_failures: usize,

    #[arg(long)]
    pub failures_only: bool,

    #[arg(long)]
    pub rerun_failed: bool,

    #[arg(long, value_name = "DURATION")]
    pub max_duration: Option<DurationArg>,

    #[arg(long = "include-output", default_value_t = false, action = clap::ArgAction::SetTrue)]
    pub include_output: bool,

    #[arg(long = "no-include-output", action = clap::ArgAction::SetFalse, overrides_with = "include_output")]
    pub no_include_output: bool,

    #[arg(long)]
    pub pass_through: bool,

    #[arg(last = true)]
    pub framework_flags: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DurationArg(pub Duration);

impl FromStr for DurationArg {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        parse_duration(value).map(Self)
    }
}

fn parse_duration(value: &str) -> Result<Duration, String> {
    let split = value
        .find(|ch: char| !ch.is_ascii_digit())
        .unwrap_or(value.len());
    if split == 0 {
        return Err(format!("missing numeric value: {value}"));
    }
    let amount = value[..split]
        .parse::<u64>()
        .map_err(|_| format!("invalid duration amount: {}", &value[..split]))?;
    let unit = &value[split..];
    let millis = match unit {
        "ms" => amount,
        "s" | "" => amount.saturating_mul(1_000),
        "m" => amount.saturating_mul(60_000),
        "h" => amount.saturating_mul(3_600_000),
        other => return Err(format!("unsupported duration unit: {other}")),
    };
    Ok(Duration::from_millis(millis))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum FrameworkArg {
    Jest,
    Vitest,
    Pytest,
    Cargo,
    Go,
    Bun,
    Deno,
}

impl FrameworkArg {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Jest => "jest",
            Self::Vitest => "vitest",
            Self::Pytest => "pytest",
            Self::Cargo => "cargo",
            Self::Go => "go",
            Self::Bun => "bun",
            Self::Deno => "deno",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_duration_units() {
        assert_eq!(parse_duration("250ms"), Ok(Duration::from_millis(250)));
        assert_eq!(parse_duration("2s"), Ok(Duration::from_secs(2)));
        assert_eq!(parse_duration("3m"), Ok(Duration::from_secs(180)));
    }
}
