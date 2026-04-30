use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogdxData {
    pub sources: Vec<SourceSummary>,
    pub summary: LogdxSummary,
    pub groups: Vec<LogGroup>,
    pub timeline: Vec<TimelineBucket>,
    pub warnings: Vec<LogdxWarning>,
    pub next: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceSummary {
    pub path: String,
    pub lines: usize,
    pub bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogdxSummary {
    pub lines: usize,
    pub groups: usize,
    pub errors: usize,
    pub warnings: usize,
    pub bytes_scanned: u64,
    pub truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogGroup {
    pub fingerprint: String,
    pub severity: Severity,
    pub count: usize,
    pub first: Occurrence,
    pub last: Occurrence,
    pub message: String,
    pub stack: Vec<String>,
    pub snippets: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Occurrence {
    pub source: String,
    pub line: usize,
    pub timestamp: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimelineBucket {
    pub bucket: String,
    pub trace: usize,
    pub debug: usize,
    pub info: usize,
    pub warn: usize,
    pub error: usize,
    pub fatal: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogdxWarning {
    pub code: WarningCode,
    pub path: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WarningCode {
    TimeUnparseable,
    InputTruncated,
    InvalidUtf8,
}

impl WarningCode {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::TimeUnparseable => "time_unparseable",
            Self::InputTruncated => "input_truncated",
            Self::InvalidUtf8 => "invalid_utf8",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
    Fatal,
}

impl Severity {
    #[must_use]
    pub const fn rank(self) -> u8 {
        match self {
            Self::Trace => 0,
            Self::Debug => 1,
            Self::Info => 2,
            Self::Warn => 3,
            Self::Error => 4,
            Self::Fatal => 5,
        }
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Trace => "trace",
            Self::Debug => "debug",
            Self::Info => "info",
            Self::Warn => "warn",
            Self::Error => "error",
            Self::Fatal => "fatal",
        }
    }

    #[must_use]
    pub const fn is_error(self) -> bool {
        matches!(self, Self::Error | Self::Fatal)
    }
}
