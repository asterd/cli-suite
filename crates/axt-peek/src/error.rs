use camino::Utf8PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PeekError {
    #[error("path does not exist: {0}")]
    PathNotFound(Utf8PathBuf),

    #[error("filesystem error: {0}")]
    Fs(#[from] axt_fs::FsError),

    #[error("git error: {0}")]
    Git(#[from] axt_git::GitError),

    #[error("failed to canonicalize {path}: {source}")]
    Canonicalize {
        path: Utf8PathBuf,
        source: std::io::Error,
    },

    #[error("canonical path is not valid UTF-8: {0:?}")]
    CanonicalPathNotUtf8(std::path::PathBuf),

    #[error("failed to format timestamp")]
    TimestampFormat(#[from] time::error::Format),

    #[error("timestamp is outside the supported range")]
    TimestampRange(#[from] time::error::ComponentRange),
}

pub type Result<T> = std::result::Result<T, PeekError>;
