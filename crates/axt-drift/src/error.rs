use camino::Utf8PathBuf;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, DriftError>;

#[derive(Debug, Error)]
pub enum DriftError {
    #[error("missing subcommand")]
    MissingSubcommand,

    #[error("no command provided; use `axt-drift run -- <COMMAND> [ARGS]...`")]
    MissingCommand,

    #[error("invalid mark name: {0}")]
    InvalidName(String),

    #[error("mark not found: {0}")]
    MarkNotFound(String),

    #[error("path is not valid UTF-8: {0:?}")]
    PathNotUtf8(std::path::PathBuf),

    #[error("failed to access {path}: {source}")]
    Io {
        path: Utf8PathBuf,
        source: std::io::Error,
    },

    #[error("failed to execute command: {0}")]
    Execute(std::io::Error),

    #[error("command exceeded max duration of {duration_ms} ms")]
    Timeout { duration_ms: u64 },

    #[error("failed to parse snapshot {path}: line {line}: {source}")]
    SnapshotParse {
        path: Utf8PathBuf,
        line: usize,
        source: serde_json::Error,
    },

    #[error("failed to serialize snapshot: {0}")]
    Serialize(#[from] serde_json::Error),
}
