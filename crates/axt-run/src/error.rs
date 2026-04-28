use camino::Utf8PathBuf;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, RunError>;

#[derive(Debug, Error)]
pub enum RunError {
    #[error("no command provided; use `axt-run -- <COMMAND> [ARGS]...`")]
    MissingCommand,

    #[error("invalid KEY=VALUE environment assignment: {0}")]
    InvalidEnv(String),

    #[error("failed to read env file {path}: {source}")]
    EnvFileRead {
        path: Utf8PathBuf,
        source: std::io::Error,
    },

    #[error("failed to parse env file {path}: line {line} is not KEY=VALUE")]
    EnvFileParse { path: Utf8PathBuf, line: usize },

    #[error("path is not valid UTF-8: {0:?}")]
    PathNotUtf8(std::path::PathBuf),

    #[error("path does not exist: {0}")]
    PathNotFound(Utf8PathBuf),

    #[error("failed to access {path}: {source}")]
    Io {
        path: Utf8PathBuf,
        source: std::io::Error,
    },

    #[error("failed to execute command: {0}")]
    Execute(std::io::Error),

    #[error("invalid glob pattern {pattern}: {source}")]
    Glob {
        pattern: String,
        source: globset::Error,
    },

    #[error("failed to serialize run metadata: {0}")]
    Serialize(#[from] serde_json::Error),

    #[error("failed to render saved run summary: {0}")]
    Render(#[from] axt_output::OutputError),

    #[error("saved run not found: {0}")]
    RunNotFound(String),
}
