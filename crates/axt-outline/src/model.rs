use camino::Utf8PathBuf;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutlineData {
    pub root: Utf8PathBuf,
    pub summary: OutlineSummary,
    pub symbols: Vec<Symbol>,
    pub warnings: Vec<OutlineWarning>,
    pub next: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutlineSummary {
    pub files: usize,
    pub symbols: usize,
    pub warnings: usize,
    pub source_bytes: usize,
    pub signature_bytes: usize,
    pub truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Symbol {
    pub path: Utf8PathBuf,
    pub language: Language,
    pub kind: SymbolKind,
    pub visibility: Visibility,
    pub name: String,
    pub signature: String,
    pub docs: Option<String>,
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

impl Language {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Go => "go",
            Self::Java => "java",
            Self::Javascript => "javascript",
            Self::Php => "php",
            Self::Python => "python",
            Self::Rust => "rust",
            Self::Typescript => "typescript",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SymbolKind {
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
    Property,
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
            Self::Property => "property",
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

    #[must_use]
    pub const fn is_publicish(self) -> bool {
        matches!(self, Self::Pub | Self::Crate | Self::Restricted)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceRange {
    pub start_line: usize,
    pub end_line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutlineWarning {
    pub code: WarningCode,
    pub path: Option<Utf8PathBuf>,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WarningCode {
    ParseError,
    UnsupportedLanguage,
}

impl WarningCode {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ParseError => "parse_error",
            Self::UnsupportedLanguage => "unsupported_language",
        }
    }
}
