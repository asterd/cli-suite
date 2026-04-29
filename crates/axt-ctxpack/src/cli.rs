use camino::Utf8PathBuf;
use clap::Parser;

#[derive(Debug, Parser)]
#[command(name = "axt-ctxpack")]
#[command(about = "Search local files for multiple named regex patterns.")]
#[command(version)]
pub struct Args {
    #[command(flatten)]
    pub common: axt_core::CommonArgs,

    #[arg(value_name = "ROOT", default_value = ".")]
    pub roots: Vec<Utf8PathBuf>,

    #[arg(long = "pattern", value_name = "NAME=REGEX")]
    pub patterns: Vec<String>,

    #[arg(long = "files", value_name = "GLOB")]
    pub files: Vec<String>,

    #[arg(long = "include", value_name = "GLOB")]
    pub includes: Vec<String>,

    #[arg(long, default_value_t = 0, value_name = "N")]
    pub context: usize,

    #[arg(long, default_value_t = 16, value_name = "N")]
    pub max_depth: usize,

    #[arg(long)]
    pub hidden: bool,

    #[arg(long)]
    pub no_ignore: bool,
}
