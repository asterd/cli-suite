use thiserror::Error;

pub type Result<T> = std::result::Result<T, DocError>;

#[derive(Debug, Error)]
pub enum DocError {
    #[error("no subcommand provided; use `which`, `path`, `env`, or `all`")]
    MissingSubcommand,

    #[error("path is not valid UTF-8: {0:?}")]
    PathNotUtf8(std::path::PathBuf),

    #[error("failed to probe command {cmd}: {source}")]
    Probe { cmd: String, source: std::io::Error },
}
