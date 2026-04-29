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

    #[arg(long = "include-output", default_value_t = false, action = clap::ArgAction::SetTrue)]
    pub include_output: bool,

    #[arg(long = "no-include-output", action = clap::ArgAction::SetFalse, overrides_with = "include_output")]
    pub no_include_output: bool,

    #[arg(long)]
    pub pass_through: bool,

    #[arg(last = true)]
    pub framework_flags: Vec<String>,
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
