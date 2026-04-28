use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct DriftData {
    pub operation: DriftOperation,
    pub name: Option<String>,
    pub mark_path: Option<String>,
    pub hash: bool,
    pub files: usize,
    pub changes: Vec<FileChange>,
    pub marks: Vec<MarkEntry>,
    pub removed: usize,
    pub command: Option<RunCommand>,
    pub exit: Option<i32>,
    pub duration_ms: Option<u64>,
}

impl DriftData {
    #[must_use]
    pub fn ok(&self) -> bool {
        self.exit.is_none_or(|exit| exit == 0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DriftOperation {
    Mark,
    Diff,
    Run,
    List,
    Reset,
}

impl DriftOperation {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Mark => "mark",
            Self::Diff => "diff",
            Self::Run => "run",
            Self::List => "list",
            Self::Reset => "reset",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct FileChange {
    pub path: String,
    pub action: ChangeAction,
    pub size_before: Option<u64>,
    pub size_after: Option<u64>,
    pub size_delta: i64,
    pub hash: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ChangeAction {
    Created,
    Modified,
    Deleted,
}

impl ChangeAction {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Modified => "modified",
            Self::Deleted => "deleted",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct MarkEntry {
    pub name: String,
    pub path: String,
    pub files: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RunCommand {
    pub program: String,
    pub args: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotRecord {
    pub path: String,
    pub size: u64,
    pub mtime_ns: Option<u128>,
    pub hash: Option<String>,
}
