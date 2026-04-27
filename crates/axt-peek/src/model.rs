use schemars::JsonSchema;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct PeekData {
    pub root: String,
    pub summary: Summary,
    pub entries: Vec<Entry>,
    #[serde(skip)]
    #[schemars(skip)]
    pub warnings: Vec<PeekWarning>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct Summary {
    pub files: usize,
    pub dirs: usize,
    pub bytes: u64,
    pub git_state: GitState,
    pub modified: usize,
    pub untracked: usize,
    pub ignored: usize,
    pub truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct Entry {
    pub path: String,
    pub kind: EntryKind,
    pub bytes: u64,
    pub language: Option<String>,
    pub mime: Option<String>,
    pub encoding: Option<Encoding>,
    pub newline: Option<NewlineStyle>,
    pub is_generated: bool,
    pub git: GitStatus,
    pub mtime: Option<String>,
    pub hash: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeekWarning {
    pub code: WarningCode,
    pub path: Option<String>,
    pub reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WarningCode {
    PermissionDenied,
    SymlinkLoop,
    PathNotUtf8,
    GitCapped,
}

impl WarningCode {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PermissionDenied => "permission_denied",
            Self::SymlinkLoop => "symlink_loop",
            Self::PathNotUtf8 => "path_not_utf8",
            Self::GitCapped => "git_capped",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EntryKind {
    File,
    Dir,
    Symlink,
    Other,
}

impl EntryKind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::File => "file",
            Self::Dir => "dir",
            Self::Symlink => "symlink",
            Self::Other => "other",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GitState {
    Clean,
    Dirty,
    None,
}

impl GitState {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Clean => "clean",
            Self::Dirty => "dirty",
            Self::None => "none",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GitStatus {
    Clean,
    Modified,
    Untracked,
    Added,
    Deleted,
    Renamed,
    Mixed,
    None,
}

impl GitStatus {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Clean => "clean",
            Self::Modified => "modified",
            Self::Untracked => "untracked",
            Self::Added => "added",
            Self::Deleted => "deleted",
            Self::Renamed => "renamed",
            Self::Mixed => "mixed",
            Self::None => "none",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Encoding {
    #[serde(rename = "utf-8")]
    Utf8,
    #[serde(rename = "utf-16")]
    Utf16,
    Latin1,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum NewlineStyle {
    Lf,
    Crlf,
    Mixed,
    None,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn peek_data_schema_is_generated_by_schemars() {
        let mut schema = schemars::schema_for!(PeekData);
        assert_eq!(schema.schema.metadata().title.as_deref(), Some("PeekData"));
    }
}
