use thiserror::Error;

pub type Result<T> = std::result::Result<T, PortError>;

#[derive(Debug, Error)]
pub enum PortError {
    #[error("missing subcommand")]
    MissingSubcommand,

    #[allow(dead_code)]
    #[error("failed to inspect ports: {0}")]
    Inspect(String),

    #[error("failed to run platform command `{command}`: {source}")]
    Command {
        command: &'static str,
        source: std::io::Error,
    },
}
