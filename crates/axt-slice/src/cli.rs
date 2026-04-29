use camino::Utf8PathBuf;
use clap::{ArgGroup, Parser, ValueEnum};

#[derive(Debug, Parser)]
#[command(name = "axt-slice")]
#[command(about = "Extract source by symbol or enclosing line.")]
#[command(version)]
#[command(group(ArgGroup::new("selector").args(["symbol", "line"]).multiple(false)))]
pub struct Args {
    #[command(flatten)]
    pub common: axt_core::CommonArgs,

    #[arg(value_name = "FILE")]
    pub file: Option<Utf8PathBuf>,

    #[arg(long, value_name = "NAME")]
    pub symbol: Option<String>,

    #[arg(long, value_name = "N")]
    pub line: Option<usize>,

    #[arg(long, value_name = "MODE", num_args = 0..=1, default_missing_value = "all")]
    pub include_imports: Option<IncludeImports>,

    #[arg(long)]
    pub include_tests: bool,

    #[arg(long)]
    pub before_symbol: bool,

    #[arg(long)]
    pub after_symbol: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum IncludeImports {
    All,
    Matched,
}
