use camino::Utf8PathBuf;
use clap::{Parser, ValueEnum};

use crate::model::Severity;

#[derive(Debug, Parser)]
#[command(name = "axt-logdx")]
#[command(about = "Diagnose local logs with bounded grouped output.")]
#[command(version)]
pub struct Args {
    #[command(flatten)]
    pub common: axt_core::CommonArgs,

    #[arg(value_name = "PATH")]
    pub paths: Vec<Utf8PathBuf>,

    #[arg(long)]
    pub stdin: bool,

    #[arg(long, value_enum, default_value_t = SeverityArg::Warn)]
    pub severity: SeverityArg,

    #[arg(long, value_name = "RFC3339")]
    pub since: Option<String>,

    #[arg(long, value_name = "RFC3339")]
    pub until: Option<String>,

    #[arg(long, default_value_t = 20, value_name = "N")]
    pub top: usize,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum SeverityArg {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
    Fatal,
}

impl From<SeverityArg> for Severity {
    fn from(value: SeverityArg) -> Self {
        match value {
            SeverityArg::Trace => Self::Trace,
            SeverityArg::Debug => Self::Debug,
            SeverityArg::Info => Self::Info,
            SeverityArg::Warn => Self::Warn,
            SeverityArg::Error => Self::Error,
            SeverityArg::Fatal => Self::Fatal,
        }
    }
}
