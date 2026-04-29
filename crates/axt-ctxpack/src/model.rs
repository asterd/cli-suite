use camino::Utf8PathBuf;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CtxpackData {
    pub root: Utf8PathBuf,
    pub patterns: Vec<SearchPattern>,
    pub summary: CtxpackSummary,
    pub hits: Vec<SearchHit>,
    pub warnings: Vec<CtxpackWarning>,
    pub next: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchPattern {
    pub name: String,
    pub query: String,
    pub kind: PatternKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PatternKind {
    Regex,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CtxpackSummary {
    pub roots: usize,
    pub files_scanned: usize,
    pub files_matched: usize,
    pub hits: usize,
    pub warnings: usize,
    pub bytes_scanned: u64,
    pub truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchHit {
    pub pattern: String,
    pub path: Utf8PathBuf,
    pub line: usize,
    pub column: usize,
    pub byte_range: ByteRange,
    pub kind: HitKind,
    pub classification_source: ClassificationSource,
    pub language: Option<String>,
    pub node_kind: Option<String>,
    pub enclosing_symbol: Option<String>,
    pub ast_path: Vec<String>,
    pub matched_text: String,
    pub snippet: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ByteRange {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HitKind {
    Code,
    Comment,
    String,
    Test,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClassificationSource {
    Ast,
    Heuristic,
    Unknown,
}

impl ClassificationSource {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ast => "ast",
            Self::Heuristic => "heuristic",
            Self::Unknown => "unknown",
        }
    }
}

impl HitKind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Code => "code",
            Self::Comment => "comment",
            Self::String => "string",
            Self::Test => "test",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CtxpackWarning {
    pub code: WarningCode,
    pub path: Option<Utf8PathBuf>,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WarningCode {
    BinarySkipped,
    NonUtf8Skipped,
    PermissionDenied,
    PathNotUtf8,
    Walk,
}

impl WarningCode {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::BinarySkipped => "binary_skipped",
            Self::NonUtf8Skipped => "non_utf8_skipped",
            Self::PermissionDenied => "permission_denied",
            Self::PathNotUtf8 => "path_not_utf8",
            Self::Walk => "walk",
        }
    }
}
