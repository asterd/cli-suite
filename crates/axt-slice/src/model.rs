use camino::Utf8PathBuf;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SliceData {
    pub path: Utf8PathBuf,
    pub language: Language,
    pub selection: Selection,
    pub status: SliceStatus,
    pub summary: SliceSummary,
    pub symbol: Option<SliceSymbol>,
    pub range: Option<SourceRange>,
    pub spans: Vec<SourceRange>,
    pub source: Option<String>,
    pub candidates: Vec<SliceCandidate>,
    pub warnings: Vec<SliceWarning>,
    pub next: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Selection {
    pub kind: SelectionKind,
    pub query: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SelectionKind {
    Symbol,
    Line,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SliceStatus {
    Selected,
    Ambiguous,
    NotFound,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SliceSummary {
    pub matches: usize,
    pub candidates: usize,
    pub source_bytes: usize,
    pub truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SliceSymbol {
    pub name: String,
    pub qualified_name: String,
    pub kind: SymbolKind,
    pub visibility: Visibility,
    pub range: SourceRange,
    pub parent: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SliceCandidate {
    pub name: String,
    pub qualified_name: String,
    pub kind: SymbolKind,
    pub visibility: Visibility,
    pub range: SourceRange,
    pub parent: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Language {
    Go,
    Java,
    Javascript,
    Php,
    Python,
    Rust,
    Typescript,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SymbolKind {
    Constructor,
    Class,
    Const,
    Enum,
    Fn,
    Impl,
    Interface,
    Macro,
    Method,
    Mod,
    Namespace,
    Package,
    Static,
    Struct,
    Trait,
    Type,
    Use,
    Var,
}

impl SymbolKind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Class => "class",
            Self::Constructor => "constructor",
            Self::Const => "const",
            Self::Enum => "enum",
            Self::Fn => "fn",
            Self::Impl => "impl",
            Self::Interface => "interface",
            Self::Macro => "macro",
            Self::Method => "method",
            Self::Mod => "mod",
            Self::Namespace => "namespace",
            Self::Package => "package",
            Self::Static => "static",
            Self::Struct => "struct",
            Self::Trait => "trait",
            Self::Type => "type",
            Self::Use => "use",
            Self::Var => "var",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Visibility {
    Package,
    Private,
    Protected,
    Pub,
    Crate,
    Restricted,
}

impl Visibility {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Package => "package",
            Self::Private => "private",
            Self::Protected => "protected",
            Self::Pub => "pub",
            Self::Crate => "crate",
            Self::Restricted => "restricted",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceRange {
    pub start_line: usize,
    pub end_line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SliceWarning {
    pub code: WarningCode,
    pub path: Option<Utf8PathBuf>,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WarningCode {
    Truncated,
}

impl WarningCode {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Truncated => "truncated",
        }
    }
}
