use camino::Utf8Path;
use tree_sitter::{Language as TsLanguage, Node, Parser};

use crate::model::{ClassificationSource, HitKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HitClassification {
    pub kind: HitKind,
    pub source: ClassificationSource,
    pub language: Option<String>,
    pub node_kind: Option<String>,
    pub enclosing_symbol: Option<String>,
    pub ast_path: Vec<String>,
}

pub fn classify_hit(path: &Utf8Path, source: &str, start: usize, end: usize, line: &str) -> HitClassification {
    if let Some(language) = Language::detect(path) {
        if let Some(classification) = classify_with_tree_sitter(language, source, start, end) {
            return classification;
        }
    }
    classify_with_heuristics(path, line)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Language {
    Go,
    Java,
    Javascript,
    Php,
    Python,
    Rust,
    Typescript,
}

impl Language {
    fn detect(path: &Utf8Path) -> Option<Self> {
        match path.extension()? {
            "go" => Some(Self::Go),
            "java" => Some(Self::Java),
            "js" | "jsx" | "mjs" | "cjs" => Some(Self::Javascript),
            "php" => Some(Self::Php),
            "py" => Some(Self::Python),
            "rs" => Some(Self::Rust),
            "ts" | "tsx" | "mts" | "cts" => Some(Self::Typescript),
            _ => None,
        }
    }

    const fn name(self) -> &'static str {
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

    fn parser_language(self) -> TsLanguage {
        match self {
            Self::Go => tree_sitter_go::LANGUAGE.into(),
            Self::Java => tree_sitter_java::LANGUAGE.into(),
            Self::Javascript => tree_sitter_javascript::LANGUAGE.into(),
            Self::Php => tree_sitter_php::LANGUAGE_PHP.into(),
            Self::Python => tree_sitter_python::LANGUAGE.into(),
            Self::Rust => tree_sitter_rust::LANGUAGE.into(),
            Self::Typescript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        }
    }
}

fn classify_with_tree_sitter(
    language: Language,
    source: &str,
    start: usize,
    end: usize,
) -> Option<HitClassification> {
    let mut parser = Parser::new();
    parser.set_language(&language.parser_language()).ok()?;
    let tree = parser.parse(source, None)?;
    if tree.root_node().has_error() {
        return None;
    }

    let node = deepest_covering_node(tree.root_node(), start, end);
    let ast_path = ast_path(node);
    let kind = ast_kind(language, node, source);
    Some(HitClassification {
        kind,
        source: ClassificationSource::Ast,
        language: Some(language.name().to_owned()),
        node_kind: Some(node.kind().to_owned()),
        enclosing_symbol: enclosing_symbol(language, node, source),
        ast_path,
    })
}

fn deepest_covering_node(node: Node<'_>, start: usize, end: usize) -> Node<'_> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.start_byte() <= start && end <= child.end_byte() {
            return deepest_covering_node(child, start, end);
        }
    }
    node
}

fn ast_path(node: Node<'_>) -> Vec<String> {
    let mut kinds = Vec::new();
    let mut current = Some(node);
    while let Some(item) = current {
        kinds.push(item.kind().to_owned());
        current = item.parent();
    }
    kinds
}

fn ast_kind(language: Language, node: Node<'_>, source: &str) -> HitKind {
    if has_ancestor(node, is_comment_kind) {
        HitKind::Comment
    } else if is_test_context(language, node, source) {
        HitKind::Test
    } else if has_ancestor(node, is_string_kind) {
        HitKind::String
    } else if node.kind() == "ERROR" {
        HitKind::Unknown
    } else {
        HitKind::Code
    }
}

fn has_ancestor(mut node: Node<'_>, predicate: fn(&str) -> bool) -> bool {
    loop {
        if predicate(node.kind()) {
            return true;
        }
        let Some(parent) = node.parent() else {
            return false;
        };
        node = parent;
    }
}

fn is_comment_kind(kind: &str) -> bool {
    kind.contains("comment")
}

fn is_string_kind(kind: &str) -> bool {
    matches!(
        kind,
        "string"
            | "string_literal"
            | "raw_string_literal"
            | "interpreted_string_literal"
            | "template_string"
            | "template_substitution"
            | "character_literal"
    )
}

fn is_test_context(language: Language, mut node: Node<'_>, source: &str) -> bool {
    loop {
        if is_test_node(language, node, source) {
            return true;
        }
        let Some(parent) = node.parent() else {
            return false;
        };
        node = parent;
    }
}

fn is_test_node(language: Language, node: Node<'_>, source: &str) -> bool {
    match language {
        Language::Rust => is_rust_test_node(node, source),
        Language::Go => named_node_text(node, source)
            .is_some_and(|name| name.starts_with("Test") || name.starts_with("Benchmark")),
        Language::Python => named_node_text(node, source).is_some_and(|name| name.starts_with("test_")),
        Language::Javascript | Language::Typescript => is_js_test_node(node, source),
        Language::Java => named_node_text(node, source).is_some_and(|name| name.starts_with("test")),
        Language::Php => named_node_text(node, source).is_some_and(|name| name.starts_with("test")),
    }
}

fn is_rust_test_node(node: Node<'_>, source: &str) -> bool {
    match node.kind() {
        "function_item" => {
            named_node_text(node, source).is_some_and(|name| name.starts_with("test"))
                || preceding_source(source, node.start_byte()).contains("#[test]")
        }
        "mod_item" => {
            named_node_text(node, source).is_some_and(|name| name == "tests")
                || preceding_source(source, node.start_byte()).contains("cfg(test)")
        }
        _ => false,
    }
}

fn is_js_test_node(node: Node<'_>, source: &str) -> bool {
    if node.kind() != "call_expression" {
        return false;
    }
    let Some(function) = node.child_by_field_name("function") else {
        return false;
    };
    let name = node_text(function, source);
    matches!(name.as_str(), "test" | "it" | "describe")
}

fn preceding_source(source: &str, start: usize) -> &str {
    let search_start = start.saturating_sub(512);
    source.get(search_start..start).unwrap_or("")
}

fn enclosing_symbol(language: Language, mut node: Node<'_>, source: &str) -> Option<String> {
    loop {
        if is_symbol_node(language, node.kind()) {
            if let Some(name) = named_node_text(node, source) {
                return Some(name);
            }
        }
        let parent = node.parent()?;
        node = parent;
    }
}

fn is_symbol_node(language: Language, kind: &str) -> bool {
    match language {
        Language::Go => matches!(kind, "function_declaration" | "method_declaration" | "type_declaration"),
        Language::Java => matches!(kind, "class_declaration" | "interface_declaration" | "method_declaration"),
        Language::Javascript | Language::Typescript => matches!(
            kind,
            "class_declaration" | "function_declaration" | "method_definition" | "lexical_declaration"
        ),
        Language::Php => matches!(kind, "class_declaration" | "function_definition" | "method_declaration"),
        Language::Python => matches!(kind, "class_definition" | "function_definition"),
        Language::Rust => matches!(
            kind,
            "function_item" | "impl_item" | "mod_item" | "struct_item" | "trait_item" | "enum_item"
        ),
    }
}

fn named_node_text(node: Node<'_>, source: &str) -> Option<String> {
    node.child_by_field_name("name")
        .map(|name| node_text(name, source))
        .or_else(|| descendant_text(node, source, &["identifier", "type_identifier"]))
}

fn descendant_text(node: Node<'_>, source: &str, kinds: &[&str]) -> Option<String> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if kinds.contains(&child.kind()) {
            return Some(node_text(child, source));
        }
        if let Some(text) = descendant_text(child, source, kinds) {
            return Some(text);
        }
    }
    None
}

fn node_text(node: Node<'_>, source: &str) -> String {
    source
        .get(node.start_byte()..node.end_byte())
        .unwrap_or("")
        .to_owned()
}

fn classify_with_heuristics(path: &Utf8Path, line: &str) -> HitClassification {
    let lower_path = path.as_str().to_ascii_lowercase();
    let trimmed = line.trim_start();
    let kind = if lower_path.contains("test")
        || lower_path.contains("spec")
        || trimmed.starts_with("#[test]")
        || trimmed.starts_with("test(")
        || trimmed.starts_with("it(")
    {
        HitKind::Test
    } else if trimmed.starts_with("//")
        || trimmed.starts_with("/*")
        || trimmed.starts_with('*')
        || trimmed.starts_with('#')
    {
        HitKind::Comment
    } else if looks_inside_string(line) {
        HitKind::String
    } else if trimmed.is_empty() {
        HitKind::Unknown
    } else {
        HitKind::Code
    };

    HitClassification {
        kind,
        source: if matches!(kind, HitKind::Unknown) {
            ClassificationSource::Unknown
        } else {
            ClassificationSource::Heuristic
        },
        language: None,
        node_kind: None,
        enclosing_symbol: None,
        ast_path: Vec::new(),
    }
}

fn looks_inside_string(line: &str) -> bool {
    let single = line.matches('\'').count();
    let double = line.matches('"').count();
    single >= 2 || double >= 2
}
