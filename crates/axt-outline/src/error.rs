use camino::Utf8PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum OutlineError {
    #[error("path does not exist: {0}")]
    PathNotFound(Utf8PathBuf),

    #[error("no supported source files found")]
    NoSupportedFiles,

    #[error("failed to read {path}: {source}")]
    Io {
        path: Utf8PathBuf,
        source: std::io::Error,
    },

    #[error("directory walk failed: {0}")]
    Walk(#[from] walkdir::Error),

    #[error("failed to render output: {0}")]
    Output(#[from] axt_output::OutputError),
}

pub type Result<T> = std::result::Result<T, OutlineError>;
