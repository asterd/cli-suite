use std::{net::IpAddr, str::FromStr, time::Duration};

use clap::{Args as ClapArgs, Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(name = "axt-port")]
#[command(about = "Inspect and free local TCP/UDP ports.")]
#[command(version)]
pub struct Args {
    #[command(flatten)]
    pub common: axt_core::CommonArgs,

    #[command(flatten)]
    pub filters: FilterArgs,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Clone, ClapArgs)]
pub struct FilterArgs {
    #[arg(long, value_enum, default_value_t = ProtocolArg::Tcp)]
    pub proto: ProtocolArg,

    #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
    pub include_loopback: bool,

    #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
    pub listening_only: bool,

    #[arg(long, value_name = "ADDR")]
    pub host: Option<IpAddr>,

    #[arg(long, value_name = "USER")]
    pub owner: Option<String>,

    #[arg(long, value_name = "PID")]
    pub pid: Option<u32>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    List,
    Who(PortsArgs),
    Free(FreeArgs),
    Watch(WatchArgs),
}

#[derive(Debug, ClapArgs)]
pub struct PortsArgs {
    #[arg(value_name = "PORT", required = true, value_parser = parse_port)]
    pub ports: Vec<u16>,
}

#[derive(Debug, ClapArgs)]
pub struct FreeArgs {
    #[arg(value_name = "PORT", required = true, value_parser = parse_port)]
    pub ports: Vec<u16>,

    #[arg(long, value_enum, default_value_t = SignalArg::Term)]
    pub signal: SignalArg,

    #[arg(long, default_value = "3s", value_parser = parse_duration)]
    pub grace: Duration,

    #[arg(long, default_value = "100ms", value_parser = parse_duration)]
    pub kill_grace: Duration,

    #[arg(long)]
    pub dry_run: bool,

    #[arg(long)]
    pub confirm: bool,

    #[arg(long)]
    pub tree: bool,

    #[arg(long)]
    pub force_self: bool,
}

#[derive(Debug, ClapArgs)]
pub struct WatchArgs {
    #[arg(value_name = "PORT", value_parser = parse_port)]
    pub port: u16,

    #[arg(long, default_value = "30s", value_parser = parse_duration)]
    pub timeout: Duration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ProtocolArg {
    Tcp,
    Udp,
    Both,
}

impl ProtocolArg {
    pub const fn matches(self, protocol: crate::model::Protocol) -> bool {
        matches!(
            (self, protocol),
            (Self::Both, _)
                | (Self::Tcp, crate::model::Protocol::Tcp)
                | (Self::Udp, crate::model::Protocol::Udp)
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum SignalArg {
    Term,
    Kill,
    Int,
}

impl SignalArg {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Term => "term",
            Self::Kill => "kill",
            Self::Int => "int",
        }
    }
}

fn parse_port(value: &str) -> Result<u16, String> {
    if value.contains(':') {
        return Err("remote host syntax is not supported; pass a local port number".to_owned());
    }
    let port = value
        .parse::<u16>()
        .map_err(|_| "port must be an integer between 0 and 65535".to_owned())?;
    if port == 0 {
        return Err("port must be greater than 0".to_owned());
    }
    Ok(port)
}

fn parse_duration(value: &str) -> Result<Duration, String> {
    let Some((number, unit)) = value
        .strip_suffix("ms")
        .map(|number| (number, "ms"))
        .or_else(|| value.strip_suffix('s').map(|number| (number, "s")))
        .or_else(|| value.strip_suffix('m').map(|number| (number, "m")))
    else {
        return Err("duration must end with ms, s, or m".to_owned());
    };
    let amount = u64::from_str(number).map_err(|_| "duration value must be an integer")?;
    Ok(match unit {
        "ms" => Duration::from_millis(amount),
        "s" => Duration::from_secs(amount),
        "m" => Duration::from_secs(amount.saturating_mul(60)),
        _ => Duration::from_secs(amount),
    })
}
