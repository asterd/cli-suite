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

    #[arg(long, value_name = "KIND", default_value_t = KindFilter::All)]
    pub kind: KindFilter,

    #[arg(long)]
    pub include_hidden: bool,

    #[arg(long)]
    pub no_ignore: bool,

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
}

impl Args {
    #[must_use]
    pub const fn git_enabled(&self) -> bool {
        !self.no_git
    }

    #[must_use]
    pub const fn files_only(&self) -> bool {
        matches!(self.kind, KindFilter::File)
    }

    #[must_use]
    pub const fn dirs_only(&self) -> bool {
        matches!(self.kind, KindFilter::Dir)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum KindFilter {
    All,
    File,
    Dir,
}

impl std::fmt::Display for KindFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::All => f.write_str("all"),
            Self::File => f.write_str("file"),
            Self::Dir => f.write_str("dir"),
        }
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
