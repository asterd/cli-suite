use std::{fmt, str::FromStr, time::Duration};

use clap::{Args as ClapArgs, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "axt-doc")]
#[command(about = "Diagnose local development environment issues.")]
#[command(version)]
pub struct Args {
    #[command(flatten)]
    pub common: axt_core::CommonArgs,

    #[arg(long)]
    pub show_secrets: bool,

    #[arg(value_name = "CMD")]
    pub cmd: Option<String>,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Which(WhichArgs),
    Path,
    Env,
    All(AllArgs),
}

#[derive(Debug, ClapArgs)]
pub struct WhichArgs {
    pub cmd: String,

    #[arg(long, default_value = "1500ms", value_name = "DURATION")]
    pub timeout: DurationArg,
}

#[derive(Debug, ClapArgs)]
pub struct AllArgs {
    pub cmd: String,

    #[arg(long, default_value = "1500ms", value_name = "DURATION")]
    pub timeout: DurationArg,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DurationArg(pub Duration);

impl FromStr for DurationArg {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        parse_duration(value).map(Self)
    }
}

impl fmt::Display for DurationArg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}ms", self.0.as_millis())
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
        other => return Err(format!("unsupported duration unit: {other}")),
    };
    Ok(Duration::from_millis(millis))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_duration_units() {
        assert_eq!(parse_duration("250ms"), Ok(Duration::from_millis(250)));
        assert_eq!(parse_duration("2s"), Ok(Duration::from_secs(2)));
        assert_eq!(parse_duration("1m"), Ok(Duration::from_secs(60)));
    }
}
