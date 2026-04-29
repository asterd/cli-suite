use camino::Utf8PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SliceError {
    #[error("missing FILE")]
    MissingFile,

    #[error("exactly one selector is required: --symbol or --line")]
    MissingSelector,

    #[error("path does not exist: {0}")]
    PathNotFound(Utf8PathBuf),

    #[error("path is not a file: {0}")]
    NotAFile(Utf8PathBuf),

    #[error("unsupported source language: {0}")]
    UnsupportedLanguage(Utf8PathBuf),

    #[error("file is binary or not valid UTF-8: {0}")]
    NonUtf8(Utf8PathBuf),

    #[error("line must be greater than zero")]
    InvalidLine,

    #[error("failed to parse source: {0}")]
    Parse(String),

    #[error("failed to read {path}: {source}")]
    Io {
        path: Utf8PathBuf,
        source: std::io::Error,
    },

    #[error("failed to render output: {0}")]
    Output(#[from] axt_output::OutputError),
}

pub type Result<T> = std::result::Result<T, SliceError>;
