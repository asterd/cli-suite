use std::{fmt, str::FromStr, time::Duration};

use camino::Utf8PathBuf;
use clap::{Args as ClapArgs, Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(name = "axt-run")]
#[command(about = "Run commands and emit structured execution envelopes.")]
#[command(version)]
pub struct Args {
    #[command(flatten)]
    pub common: axt_core::CommonArgs,

    #[arg(long)]
    pub rerun_last: bool,

    #[command(subcommand)]
    pub subcommand: Option<Command>,

    #[command(flatten)]
    pub run: RunArgs,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Show(ShowArgs),
    List,
    Clean(CleanArgs),
}

#[derive(Debug, ClapArgs)]
pub struct ShowArgs {
    #[arg(default_value = "last")]
    pub name: String,

    #[arg(long)]
    pub stdout: bool,

    #[arg(long)]
    pub stderr: bool,
}

#[derive(Debug, ClapArgs)]
pub struct CleanArgs {
    #[arg(long, value_name = "DURATION")]
    pub older_than: Option<DurationArg>,
}

#[derive(Debug, ClapArgs)]
pub struct RunArgs {
    #[arg(long, value_name = "NAME")]
    pub save: Option<String>,

    #[arg(long)]
    pub no_save: bool,

    #[arg(long, value_name = "DIR")]
    pub cwd: Option<Utf8PathBuf>,

    #[arg(long = "env", value_name = "KEY=VALUE")]
    pub env: Vec<String>,

    #[arg(long, value_name = "FILE")]
    pub env_file: Option<Utf8PathBuf>,

    #[arg(long, value_name = "DURATION")]
    pub timeout: Option<DurationArg>,

    #[arg(long, default_value_t = CaptureMode::Auto)]
    pub capture: CaptureMode,

    #[arg(long, default_value = "5MiB", value_name = "SIZE")]
    pub max_log_bytes: SizeArg,

    #[arg(long = "watch-files", action = clap::ArgAction::SetTrue)]
    pub watch_files: bool,

    #[arg(
        long = "no-watch-files",
        action = clap::ArgAction::SetTrue,
        conflicts_with = "watch_files"
    )]
    pub no_watch_files: bool,

    #[arg(long, value_name = "GLOB")]
    pub include: Vec<String>,

    #[arg(long, value_name = "GLOB")]
    pub exclude: Vec<String>,

    #[arg(long)]
    pub shell: bool,

    #[arg(long)]
    pub summary_only: bool,

    #[arg(long, value_name = "N")]
    pub tail_bytes: Option<usize>,

    #[arg(long)]
    pub hash: bool,

    #[arg(last = true, value_name = "COMMAND")]
    pub command: Vec<String>,
}

impl RunArgs {
    #[must_use]
    pub const fn watch_files_enabled(&self) -> bool {
        !self.no_watch_files
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum CaptureMode {
    Always,
    Never,
    Auto,
}

impl fmt::Display for CaptureMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Always => "always",
            Self::Never => "never",
            Self::Auto => "auto",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DurationArg(pub Duration);

impl FromStr for DurationArg {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        parse_duration(value).map(Self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SizeArg(pub u64);

impl FromStr for SizeArg {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        parse_size(value).map(Self)
    }
}

fn parse_duration(value: &str) -> Result<Duration, String> {
    let (number, unit) = split_number_unit(value)?;
    let amount = number
        .parse::<u64>()
        .map_err(|_| format!("invalid duration amount: {number}"))?;
    let millis = match unit {
        "ms" => amount,
        "s" | "" => amount.saturating_mul(1_000),
        "m" => amount.saturating_mul(60_000),
        "h" => amount.saturating_mul(3_600_000),
        "d" => amount.saturating_mul(86_400_000),
        other => return Err(format!("unsupported duration unit: {other}")),
    };
    Ok(Duration::from_millis(millis))
}

fn parse_size(value: &str) -> Result<u64, String> {
    let (number, unit) = split_number_unit(value)?;
    let amount = number
        .parse::<u64>()
        .map_err(|_| format!("invalid size amount: {number}"))?;
    let multiplier = match unit.to_ascii_lowercase().as_str() {
        "" | "b" => 1,
        "k" | "kb" => 1_000,
        "m" | "mb" => 1_000_000,
        "g" | "gb" => 1_000_000_000,
        "kib" => 1_024,
        "mib" => 1_048_576,
        "gib" => 1_073_741_824,
        other => return Err(format!("unsupported size unit: {other}")),
    };
    Ok(amount.saturating_mul(multiplier))
}

fn split_number_unit(value: &str) -> Result<(&str, &str), String> {
    let split = value
        .find(|ch: char| !ch.is_ascii_digit())
        .unwrap_or(value.len());
    if split == 0 {
        return Err(format!("missing numeric value: {value}"));
    }
    Ok((&value[..split], &value[split..]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_duration_units() {
        assert_eq!(parse_duration("30s"), Ok(Duration::from_secs(30)));
        assert_eq!(parse_duration("5m"), Ok(Duration::from_secs(300)));
        assert_eq!(parse_duration("250ms"), Ok(Duration::from_millis(250)));
    }

    #[test]
    fn parses_size_units() {
        assert_eq!(parse_size("5MiB"), Ok(5 * 1_048_576));
        assert_eq!(parse_size("10kb"), Ok(10_000));
    }
}
