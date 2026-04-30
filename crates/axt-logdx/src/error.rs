use std::io;

use camino::Utf8PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum LogdxError {
    #[error("no input provided; pass one or more paths or --stdin")]
    NoInput,
    #[error("path not found: {0}")]
    PathNotFound(Utf8PathBuf),
    #[error("invalid {field} timestamp {value:?}; expected RFC3339")]
    InvalidTime { field: &'static str, value: String },
    #[error("failed to read {path}: {source}")]
    Io { path: String, source: io::Error },
    #[error(transparent)]
    Output(#[from] axt_output::OutputError),
}
