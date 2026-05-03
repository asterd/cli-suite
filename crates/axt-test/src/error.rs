use thiserror::Error;

pub type Result<T> = std::result::Result<T, TestError>;

#[derive(Debug, Error)]
pub enum TestError {
    #[error("multiple frameworks detected; pass --single to refuse or --framework to force one")]
    MultipleFrameworks,

    #[error("no supported test framework detected")]
    NoFramework,

    #[error("framework command `{command}` is unavailable")]
    MissingTool { command: String },

    #[error("failed to run framework command `{command}`: {source}")]
    Command {
        command: String,
        source: std::io::Error,
    },

    #[error("framework command `{command}` exceeded max duration of {duration_ms} ms")]
    Timeout { command: String, duration_ms: u64 },

    #[error("failed to read test output: {0}")]
    Io(String),

    #[error("test parser panic for {framework}: {message}")]
    ParserPanic { framework: String, message: String },

    #[error("test parser defaulted fields for {framework}: {fields}")]
    ParserDefaulted { framework: String, fields: String },

    #[error(transparent)]
    Output(#[from] axt_output::OutputError),

    #[error("git repository is required for changed-file filtering")]
    GitUnavailable,

    #[error("failed to read git changes: {0}")]
    Git(String),
}
