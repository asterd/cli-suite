use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RunData {
    pub command: RunCommand,
    pub cwd: String,
    pub exit: Option<i32>,
    pub timed_out: bool,
    pub duration_ms: u64,
    pub stdout: StreamSummary,
    pub stderr: StreamSummary,
    pub changed_count: usize,
    pub changed: Vec<FileChange>,
    pub saved: Option<SavedRun>,
    pub truncated: bool,
}

impl RunData {
    #[must_use]
    pub fn ok(&self) -> bool {
        self.exit == Some(0) && !self.timed_out
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RunCommand {
    pub program: String,
    pub args: Vec<String>,
    pub shell: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct StreamSummary {
    pub bytes: u64,
    pub lines: usize,
    pub truncated: bool,
    pub log: Option<String>,
    #[serde(skip)]
    #[schemars(skip)]
    pub tail: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct FileChange {
    pub path: String,
    pub action: ChangeAction,
    pub bytes: Option<u64>,
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
pub struct SavedRun {
    pub name: String,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredRun {
    pub schema: String,
    pub created_at: String,
    pub data: RunData,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_data_schema_is_generated_by_schemars() {
        let mut schema = schemars::schema_for!(RunData);
        assert_eq!(schema.schema.metadata().title.as_deref(), Some("RunData"));
    }
}
