use camino::Utf8PathBuf;
use clap::{Parser, ValueEnum};

#[derive(Debug, Parser)]
#[command(name = "axt-outline")]
#[command(about = "Emit compact source outlines without function bodies.")]
#[command(version)]
pub struct Args {
    #[command(flatten)]
    pub common: axt_core::CommonArgs,

    #[arg(value_name = "PATH", default_value = ".")]
    pub paths: Vec<Utf8PathBuf>,

    #[arg(long, value_enum)]
    pub lang: Option<LanguageArg>,

    #[arg(long)]
    pub public_only: bool,

    #[arg(long)]
    pub private: bool,

    #[arg(long)]
    pub tests: bool,

    #[arg(long, default_value_t = 16, value_name = "N")]
    pub max_depth: usize,

    #[arg(long, value_enum, default_value_t = SortArg::Path)]
    pub sort: SortArg,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum LanguageArg {
    Go,
    Java,
    Javascript,
    Php,
    Python,
    Rust,
    Typescript,
}

impl LanguageArg {
    #[must_use]
    pub const fn into_language(self) -> crate::model::Language {
        match self {
            Self::Go => crate::model::Language::Go,
            Self::Java => crate::model::Language::Java,
            Self::Javascript => crate::model::Language::Javascript,
            Self::Php => crate::model::Language::Php,
            Self::Python => crate::model::Language::Python,
            Self::Rust => crate::model::Language::Rust,
            Self::Typescript => crate::model::Language::Typescript,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum SortArg {
    Path,
    Name,
    Kind,
    Source,
}
