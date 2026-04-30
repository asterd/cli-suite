use camino::Utf8PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CtxpackError {
    #[error("path does not exist: {0}")]
    PathNotFound(Utf8PathBuf),

    #[error("invalid pattern {pattern}: expected NAME=REGEX")]
    InvalidPatternShape { pattern: String },

    #[error("at least one --pattern NAME=REGEX is required")]
    MissingPattern,

    #[error("invalid pattern name: {0}")]
    InvalidPatternName(String),

    #[error("duplicate pattern name: {0}")]
    DuplicatePatternName(String),

    #[error("invalid regex for pattern {name}: {source}")]
    InvalidRegex {
        name: String,
        source: regex::Error,
    },

    #[error("regex for pattern {name} exceeds maximum length of {max_len} bytes")]
    RegexTooLong { name: String, max_len: usize },

    #[error("invalid include glob {glob}: {source}")]
    InvalidGlob {
        glob: String,
        source: globset::Error,
    },

    #[error("filesystem error: {0}")]
    Fs(#[from] axt_fs::FsError),

    #[error("failed to read {path}: {source}")]
    Io {
        path: Utf8PathBuf,
        source: std::io::Error,
    },

    #[error("failed to render output: {0}")]
    Output(#[from] axt_output::OutputError),
}

pub type Result<T> = std::result::Result<T, CtxpackError>;
