use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DocData {
    pub which: Option<WhichReport>,
    pub path: Option<PathReport>,
    pub env: Option<EnvReport>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WhichReport {
    pub cmd: String,
    pub found: bool,
    pub primary: Option<String>,
    pub matches: Vec<CommandMatch>,
    pub version: VersionProbe,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CommandMatch {
    pub path: String,
    pub manager: Option<String>,
    pub source: String,
    pub executable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct VersionProbe {
    pub attempted: bool,
    pub ok: bool,
    pub timed_out: bool,
    pub command: Option<String>,
    pub output: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PathReport {
    pub entries: Vec<PathEntry>,
    pub duplicates: Vec<PathDuplicate>,
    pub missing: Vec<String>,
    pub broken_symlinks: Vec<String>,
    pub ordering_issues: Vec<OrderingIssue>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PathEntry {
    pub index: usize,
    pub path: String,
    pub exists: bool,
    pub is_dir: bool,
    pub is_symlink: bool,
    pub canonical: Option<String>,
    pub manager: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PathDuplicate {
    pub path: String,
    pub first_index: usize,
    pub duplicate_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OrderingIssue {
    pub kind: String,
    pub path: String,
    pub index: usize,
    pub earlier_index: usize,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EnvReport {
    pub total: usize,
    pub vars: Vec<EnvVarReport>,
    pub secret_like: Vec<EnvVarReport>,
    pub suspicious: Vec<EnvSuspicion>,
    pub show_secrets: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EnvVarReport {
    pub name: String,
    pub value: String,
    pub secret_like: bool,
    pub empty: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EnvSuspicion {
    pub name: String,
    pub reason: String,
}
