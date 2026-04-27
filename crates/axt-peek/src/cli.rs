use camino::Utf8PathBuf;
use clap::{Parser, ValueEnum};

#[derive(Debug, Parser)]
#[command(name = "axt-peek")]
#[command(about = "Directory and repository snapshot command.")]
#[command(version)]
pub struct Args {
    #[command(flatten)]
    pub common: axt_core::CommonArgs,

    #[arg(value_name = "PATHS", default_value = ".")]
    pub paths: Vec<Utf8PathBuf>,

    #[arg(long, default_value_t = 2, value_name = "N")]
    pub depth: usize,

    #[arg(long, conflicts_with = "dirs_only")]
    pub files_only: bool,

    #[arg(long, conflicts_with = "files_only")]
    pub dirs_only: bool,

    #[arg(long)]
    pub include_hidden: bool,

    #[arg(long)]
    pub no_ignore: bool,

    #[arg(long, conflicts_with = "no_git")]
    pub git: bool,

    #[arg(long)]
    pub no_git: bool,

    #[arg(long)]
    pub changed: bool,

    #[arg(long, value_name = "REF")]
    pub changed_since: Option<String>,

    #[arg(long = "type", value_name = "KIND")]
    pub type_filter: Option<TypeFilter>,

    #[arg(long, value_name = "LANG")]
    pub lang: Option<String>,

    #[arg(long, default_value_t = HashMode::None)]
    pub hash: HashMode,

    #[arg(long)]
    pub summary_only: bool,

    #[arg(long, default_value_t = SortKey::Name)]
    pub sort: SortKey,

    #[arg(long)]
    pub reverse: bool,

    #[arg(long, value_name = "SIZE")]
    pub max_file_size: Option<u64>,

    #[arg(long)]
    pub follow_symlinks: bool,

    #[arg(long)]
    pub cross_fs: bool,

    #[arg(long, default_value_t = ColorArg::Auto)]
    pub color: ColorArg,

    #[arg(long)]
    pub quiet: bool,

    #[arg(long, short = 'v', action = clap::ArgAction::Count)]
    pub verbose: u8,
}

impl Args {
    #[must_use]
    pub const fn git_enabled(&self) -> bool {
        !self.no_git
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum HashMode {
    None,
    Blake3,
}

impl std::fmt::Display for HashMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => f.write_str("none"),
            Self::Blake3 => f.write_str("blake3"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum SortKey {
    Name,
    Size,
    Mtime,
    Git,
    Type,
}

impl std::fmt::Display for SortKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Name => f.write_str("name"),
            Self::Size => f.write_str("size"),
            Self::Mtime => f.write_str("mtime"),
            Self::Git => f.write_str("git"),
            Self::Type => f.write_str("type"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum TypeFilter {
    Text,
    Binary,
    Image,
    Archive,
    Code,
    Config,
    Data,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ColorArg {
    Auto,
    Always,
    Never,
}

impl std::fmt::Display for ColorArg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Auto => f.write_str("auto"),
            Self::Always => f.write_str("always"),
            Self::Never => f.write_str("never"),
        }
    }
}
