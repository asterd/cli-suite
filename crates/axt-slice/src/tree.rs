use camino::{Utf8Path, Utf8PathBuf};
use tree_sitter::{Language as TsLanguage, Node, Parser};

use crate::model::{
    Language, SourceRange, SymbolKind, Visibility,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedFile {
    pub source: String,
    pub symbols: Vec<ParsedSymbol>,
    pub imports: Vec<SourceSpan>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedSymbol {
    pub name: String,
    pub qualified_name: String,
    pub kind: SymbolKind,
    pub visibility: Visibility,
    pub parent: Option<String>,
    pub span: SourceSpan,
    pub symbol_span: SourceSpan,
    pub is_test: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceSpan {
    pub start_byte: usize,
    pub end_byte: usize,
    pub range: SourceRange,
}

pub fn parse_source(
    path: &Utf8Path,
    relative: &Utf8PathBuf,
    language: Language,
    source: String,
) -> std::result::Result<ParsedFile, String> {
    let spec = spec(language, path);
    let mut parser = Parser::new();
    parser
        .set_language(&spec.ts_language)
        .map_err(|err| err.to_string())?;
    let tree = parser
        .parse(&source, None)
        .ok_or_else(|| "tree-sitter parser returned no tree".to_owned())?;
    if tree.root_node().has_error() {
        return Err(format!("{path}: source contains syntax errors"));
    }

    let mut symbols = Vec::new();
    let mut imports = Vec::new();
    visit(
        tree.root_node(),
        &source,
        &spec,
        None,
        false,
        &mut symbols,
        &mut imports,
    );
    let _ = relative;
    Ok(ParsedFile {
        source,
        symbols,
        imports,
    })
}

#[derive(Debug, Clone)]
struct LanguageSpec {
    language: Language,
    ts_language: TsLanguage,
    declarations: &'static [Declaration],
    import_kinds: &'static [&'static str],
    container_kinds: &'static [&'static str],
}

#[derive(Debug, Clone, Copy)]
struct Declaration {
    node_kind: &'static str,
    symbol_kind: SymbolKind,
    default_visibility: Visibility,
}

fn spec(language: Language, path: &Utf8Path) -> LanguageSpec {
    match language {
        Language::Go => LanguageSpec {
            language,
            ts_language: tree_sitter_go::LANGUAGE.into(),
            declarations: GO_DECLS,
            import_kinds: GO_IMPORTS,
            container_kinds: GO_CONTAINERS,
        },
        Language::Java => LanguageSpec {
            language,
            ts_language: tree_sitter_java::LANGUAGE.into(),
            declarations: JAVA_DECLS,
            import_kinds: JAVA_IMPORTS,
            container_kinds: JAVA_CONTAINERS,
        },
        Language::Javascript => LanguageSpec {
            language,
            ts_language: tree_sitter_javascript::LANGUAGE.into(),
            declarations: JS_DECLS,
            import_kinds: JS_IMPORTS,
            container_kinds: JS_CONTAINERS,
        },
        Language::Php => LanguageSpec {
            language,
            ts_language: tree_sitter_php::LANGUAGE_PHP.into(),
            declarations: PHP_DECLS,
            import_kinds: PHP_IMPORTS,
            container_kinds: PHP_CONTAINERS,
        },
        Language::Python => LanguageSpec {
            language,
            ts_language: tree_sitter_python::LANGUAGE.into(),
            declarations: PY_DECLS,
            import_kinds: PY_IMPORTS,
            container_kinds: PY_CONTAINERS,
        },
        Language::Rust => LanguageSpec {
            language,
            ts_language: tree_sitter_rust::LANGUAGE.into(),
            declarations: RUST_DECLS,
            import_kinds: RUST_IMPORTS,
            container_kinds: RUST_CONTAINERS,
        },
        Language::Typescript => LanguageSpec {
            language,
            ts_language: if path.extension() == Some("tsx") {
                tree_sitter_typescript::LANGUAGE_TSX.into()
            } else {
                tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()
            },
            declarations: TS_DECLS,
            import_kinds: TS_IMPORTS,
            container_kinds: TS_CONTAINERS,
        },
    }
}

const RUST_DECLS: &[Declaration] = &[
    decl("const_item", SymbolKind::Const, Visibility::Private),
    decl("enum_item", SymbolKind::Enum, Visibility::Private),
    decl("function_item", SymbolKind::Fn, Visibility::Private),
    decl("function_signature_item", SymbolKind::Fn, Visibility::Private),
    decl("impl_item", SymbolKind::Impl, Visibility::Private),
    decl("macro_definition", SymbolKind::Macro, Visibility::Private),
    decl("mod_item", SymbolKind::Mod, Visibility::Private),
    decl("static_item", SymbolKind::Static, Visibility::Private),
    decl("struct_item", SymbolKind::Struct, Visibility::Private),
    decl("trait_item", SymbolKind::Trait, Visibility::Private),
    decl("type_item", SymbolKind::Type, Visibility::Private),
];
const RUST_IMPORTS: &[&str] = &["use_declaration"];
const RUST_CONTAINERS: &[&str] = &["impl_item", "mod_item", "trait_item"];

const TS_DECLS: &[Declaration] = &[
    decl("class_declaration", SymbolKind::Class, Visibility::Pub),
    decl("enum_declaration", SymbolKind::Enum, Visibility::Pub),
    decl("function_declaration", SymbolKind::Fn, Visibility::Pub),
    decl("generator_function_declaration", SymbolKind::Fn, Visibility::Pub),
    decl("interface_declaration", SymbolKind::Interface, Visibility::Pub),
    decl("lexical_declaration", SymbolKind::Const, Visibility::Pub),
    decl("method_definition", SymbolKind::Method, Visibility::Pub),
    decl("method_signature", SymbolKind::Method, Visibility::Pub),
    decl("type_alias_declaration", SymbolKind::Type, Visibility::Pub),
    decl("variable_declaration", SymbolKind::Var, Visibility::Pub),
];
const TS_IMPORTS: &[&str] = &["import_statement"];
const TS_CONTAINERS: &[&str] = &["class_declaration", "interface_declaration"];

const JS_DECLS: &[Declaration] = &[
    decl("class_declaration", SymbolKind::Class, Visibility::Pub),
    decl("function_declaration", SymbolKind::Fn, Visibility::Pub),
    decl("generator_function_declaration", SymbolKind::Fn, Visibility::Pub),
    decl("lexical_declaration", SymbolKind::Const, Visibility::Pub),
    decl("method_definition", SymbolKind::Method, Visibility::Pub),
    decl("variable_declaration", SymbolKind::Var, Visibility::Pub),
];
const JS_IMPORTS: &[&str] = &["import_statement"];
const JS_CONTAINERS: &[&str] = &["class_declaration"];

const PY_DECLS: &[Declaration] = &[
    decl("class_definition", SymbolKind::Class, Visibility::Pub),
    decl("function_definition", SymbolKind::Fn, Visibility::Pub),
];
const PY_IMPORTS: &[&str] = &["import_statement", "import_from_statement"];
const PY_CONTAINERS: &[&str] = &["class_definition"];

const GO_DECLS: &[Declaration] = &[
    decl("const_declaration", SymbolKind::Const, Visibility::Private),
    decl("function_declaration", SymbolKind::Fn, Visibility::Private),
    decl("method_declaration", SymbolKind::Method, Visibility::Private),
    decl("type_declaration", SymbolKind::Type, Visibility::Private),
    decl("var_declaration", SymbolKind::Var, Visibility::Private),
];
const GO_IMPORTS: &[&str] = &["package_clause", "import_declaration"];
const GO_CONTAINERS: &[&str] = &[];

const JAVA_DECLS: &[Declaration] = &[
    decl("class_declaration", SymbolKind::Class, Visibility::Package),
    decl("constructor_declaration", SymbolKind::Constructor, Visibility::Package),
    decl("enum_declaration", SymbolKind::Enum, Visibility::Package),
    decl("interface_declaration", SymbolKind::Interface, Visibility::Package),
    decl("method_declaration", SymbolKind::Method, Visibility::Package),
];
const JAVA_IMPORTS: &[&str] = &["package_declaration", "import_declaration"];
const JAVA_CONTAINERS: &[&str] = &["class_declaration", "enum_declaration", "interface_declaration"];

const PHP_DECLS: &[Declaration] = &[
    decl("class_declaration", SymbolKind::Class, Visibility::Pub),
    decl("constructor_declaration", SymbolKind::Constructor, Visibility::Pub),
    decl("function_definition", SymbolKind::Fn, Visibility::Pub),
    decl("interface_declaration", SymbolKind::Interface, Visibility::Pub),
    decl("method_declaration", SymbolKind::Method, Visibility::Pub),
    decl("namespace_definition", SymbolKind::Namespace, Visibility::Pub),
    decl("trait_declaration", SymbolKind::Trait, Visibility::Pub),
];
const PHP_IMPORTS: &[&str] = &["namespace_use_declaration", "namespace_definition"];
const PHP_CONTAINERS: &[&str] = &["class_declaration", "interface_declaration", "trait_declaration"];

const fn decl(
    node_kind: &'static str,
    symbol_kind: SymbolKind,
    default_visibility: Visibility,
) -> Declaration {
    Declaration {
        node_kind,
        symbol_kind,
        default_visibility,
    }
}

fn visit(
    node: Node<'_>,
    source: &str,
    spec: &LanguageSpec,
    parent: Option<&str>,
    parent_is_test: bool,
    symbols: &mut Vec<ParsedSymbol>,
    imports: &mut Vec<SourceSpan>,
) {
    if spec.import_kinds.contains(&node.kind())
        && node.parent().is_some_and(|p| {
            matches!(
                p.kind(),
                "source_file" | "program" | "module" | "translation_unit"
            )
        })
    {
        imports.push(span_for_node(node, source, false));
    }

    let declaration = declaration_for(spec, node.kind());
    let symbol_name = declaration.and_then(|decl| name_for(spec.language, node, source, decl));
    let mut next_parent = parent;
    let mut next_parent_is_test = parent_is_test;

    if let (Some(decl), Some(name)) = (declaration, symbol_name.as_deref()) {
        let visibility = visibility_for(spec.language, node, source, decl.default_visibility, name);
        let kind = kind_for(spec.language, node, source, decl.symbol_kind);
        let symbol_parent = symbol_parent(spec.language, node, source, parent);
        let qualified_name = symbol_parent
            .as_deref()
            .map_or_else(|| name.to_owned(), |p| format!("{p}::{name}"));
        let symbol_span = span_for_node(node, source, false);
        let span = span_for_node(node, source, true);
        let is_test = parent_is_test || is_test_symbol(spec.language, node, source, name);
        symbols.push(ParsedSymbol {
            name: name.to_owned(),
            qualified_name,
            kind,
            visibility,
            parent: symbol_parent,
            span,
            symbol_span,
            is_test,
        });
        if spec.container_kinds.contains(&node.kind()) {
            next_parent = Some(name);
            next_parent_is_test = is_test;
        } else {
            return;
        }
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        visit(
            child,
            source,
            spec,
            next_parent,
            next_parent_is_test,
            symbols,
            imports,
        );
    }
}

fn symbol_parent(
    language: Language,
    node: Node<'_>,
    source: &str,
    parent: Option<&str>,
) -> Option<String> {
    if matches!(language, Language::Go) && node.kind() == "method_declaration" {
        return go_receiver_parent(node, source).or_else(|| parent.map(str::to_owned));
    }
    parent.map(str::to_owned)
}

fn go_receiver_parent(node: Node<'_>, source: &str) -> Option<String> {
    let sig = signature(Language::Go, node, source);
    let receiver_start = sig.find("func (")? + "func (".len();
    let receiver_end = sig[receiver_start..].find(')')? + receiver_start;
    let receiver = sig[receiver_start..receiver_end].trim();
    let ty = receiver.split_whitespace().last()?;
    let ty = ty.trim_start_matches(['*', '&', '[', ']']);
    if ty.is_empty() {
        None
    } else {
        Some(ty.to_owned())
    }
}

fn declaration_for(spec: &LanguageSpec, node_kind: &str) -> Option<Declaration> {
    spec.declarations
        .iter()
        .copied()
        .find(|decl| decl.node_kind == node_kind)
}

fn name_for(
    language: Language,
    node: Node<'_>,
    source: &str,
    decl: Declaration,
) -> Option<String> {
    if matches!(decl.symbol_kind, SymbolKind::Impl) {
        return Some(text(node.child_by_field_name("type")?, source));
    }
    if matches!(decl.symbol_kind, SymbolKind::Constructor) {
        return constructor_name(node, source);
    }
    if matches!(decl.symbol_kind, SymbolKind::Namespace) {
        return package_name(node, source);
    }
    if matches!(language, Language::Go)
        && matches!(decl.node_kind, "type_declaration" | "const_declaration" | "var_declaration")
    {
        return go_decl_name(node, source);
    }
    if matches!(language, Language::Typescript | Language::Javascript)
        && matches!(decl.node_kind, "lexical_declaration" | "variable_declaration")
    {
        return descendant_text(node, source, &["identifier"]);
    }
    node.child_by_field_name("name")
        .map(|name| text(name, source))
        .or_else(|| descendant_text(node, source, NAME_KINDS))
}

fn constructor_name(node: Node<'_>, source: &str) -> Option<String> {
    node.child_by_field_name("name")
        .map(|name| text(name, source))
        .or_else(|| Some("new".to_owned()))
}


fn package_name(node: Node<'_>, source: &str) -> Option<String> {
    descendant_text(
        node,
        source,
        &[
            "identifier",
            "package_identifier",
            "scoped_identifier",
            "namespace_name",
            "qualified_name",
        ],
    )
}

fn go_decl_name(node: Node<'_>, source: &str) -> Option<String> {
    descendant_text(
        node,
        source,
        &[
            "type_identifier",
            "field_identifier",
            "identifier",
            "package_identifier",
        ],
    )
}

const NAME_KINDS: &[&str] = &[
    "identifier",
    "type_identifier",
    "field_identifier",
    "property_identifier",
    "constant",
    "name",
];

fn descendant_text(node: Node<'_>, source: &str, kinds: &[&str]) -> Option<String> {
    if kinds.contains(&node.kind()) {
        return Some(text(node, source));
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if let Some(found) = descendant_text(child, source, kinds) {
            return Some(found);
        }
    }
    None
}

fn kind_for(
    language: Language,
    node: Node<'_>,
    source: &str,
    default_kind: SymbolKind,
) -> SymbolKind {
    if matches!(language, Language::Go) && node.kind() == "type_declaration" {
        let sig = signature(language, node, source);
        if sig.contains(" struct") {
            SymbolKind::Struct
        } else if sig.contains(" interface") {
            SymbolKind::Interface
        } else {
            SymbolKind::Type
        }
    } else if matches!(language, Language::Python)
        && node.kind() == "function_definition"
        && node.parent().is_some_and(|parent| parent.kind() == "block")
    {
        SymbolKind::Method
    } else {
        default_kind
    }
}

fn visibility_for(
    language: Language,
    node: Node<'_>,
    source: &str,
    default_visibility: Visibility,
    name: &str,
) -> Visibility {
    match language {
        Language::Go => go_visibility(name),
        Language::Python => python_visibility(name),
        Language::Rust => rust_visibility(node, source),
        Language::Java | Language::Php | Language::Javascript | Language::Typescript => {
            keyword_visibility(node, source, default_visibility)
        }
    }
}

fn rust_visibility(node: Node<'_>, source: &str) -> Visibility {
    let sig = signature(Language::Rust, node, source);
    if sig.starts_with("pub(crate)") {
        Visibility::Crate
    } else if sig.starts_with("pub(") {
        Visibility::Restricted
    } else if sig.starts_with("pub ") {
        Visibility::Pub
    } else {
        Visibility::Private
    }
}

fn keyword_visibility(node: Node<'_>, source: &str, default_visibility: Visibility) -> Visibility {
    let sig = signature(Language::Javascript, node, source);
    if sig.contains("private ") || sig.starts_with('#') {
        Visibility::Private
    } else if sig.contains("protected ") {
        Visibility::Protected
    } else if sig.contains("public ") || sig.contains("export ") {
        Visibility::Pub
    } else {
        default_visibility
    }
}

fn python_visibility(name: &str) -> Visibility {
    if name.starts_with('_') && !name.starts_with("__") {
        Visibility::Private
    } else {
        Visibility::Pub
    }
}

fn go_visibility(name: &str) -> Visibility {
    if name.chars().next().is_some_and(char::is_uppercase) {
        Visibility::Pub
    } else {
        Visibility::Private
    }
}

fn signature(language: Language, node: Node<'_>, source: &str) -> String {
    let end = if matches!(language, Language::Python) {
        line_end(source, node.start_byte())
    } else {
        body_start(node).unwrap_or_else(|| node.end_byte())
    };
    source[node.start_byte()..end]
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
        .trim_end_matches(';')
        .trim()
        .to_owned()
}

fn body_start(node: Node<'_>) -> Option<usize> {
    let mut cursor = node.walk();
    let found = node
        .named_children(&mut cursor)
        .find(|child| BODY_KINDS.contains(&child.kind()))
        .map(|child| child.start_byte());
    found
}

const BODY_KINDS: &[&str] = &[
    "block",
    "class_body",
    "compound_statement",
    "declaration_list",
    "enum_body",
    "enum_variant_list",
    "field_declaration_list",
    "interface_body",
    "statement_block",
    "trait_body",
];

fn is_test_symbol(language: Language, node: Node<'_>, source: &str, name: &str) -> bool {
    if name.contains("test") || name.starts_with("it_") || name.starts_with("should_") {
        return true;
    }
    let prefix = &source[extended_start(source, node.start_byte())..node.start_byte()];
    match language {
        Language::Rust => prefix.contains("#[test]") || prefix.contains("cfg(test)"),
        Language::Go => name.starts_with("Test"),
        Language::Javascript | Language::Typescript => name.ends_with("test") || name.ends_with("spec"),
        Language::Python => name.starts_with("test_"),
        Language::Java | Language::Php => prefix.contains("@Test"),
    }
}

fn span_for_node(node: Node<'_>, source: &str, extend: bool) -> SourceSpan {
    let start = if extend {
        extended_start(source, node.start_byte())
    } else {
        line_start(source, node.start_byte())
    };
    let end = include_line_ending(source, node.end_byte());
    SourceSpan {
        start_byte: start,
        end_byte: end,
        range: SourceRange {
            start_line: line_for_byte(source, start),
            end_line: line_for_byte(source, end.saturating_sub(1)),
        },
    }
}

fn extended_start(source: &str, start_byte: usize) -> usize {
    let mut current = line_start(source, start_byte);
    while current > 0 {
        let previous_end = current.saturating_sub(1);
        let previous_start = line_start(source, previous_end);
        let line = &source[previous_start..current];
        if is_leading_metadata_line(line) {
            current = previous_start;
        } else {
            break;
        }
    }
    current
}

fn is_leading_metadata_line(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.starts_with("///")
        || trimmed.starts_with("//")
        || trimmed.starts_with('#')
        || trimmed.starts_with("#[")
        || trimmed.starts_with('@')
        || trimmed.starts_with("/*")
        || trimmed.starts_with('*')
}

fn text(node: Node<'_>, source: &str) -> String {
    source[node.start_byte()..node.end_byte()].to_owned()
}

fn line_start(source: &str, index: usize) -> usize {
    source[..index].rfind('\n').map_or(0, |newline| newline + 1)
}

fn line_end(source: &str, index: usize) -> usize {
    source[index..]
        .find('\n')
        .map_or_else(|| source.len(), |newline| index + newline)
}

fn include_line_ending(source: &str, index: usize) -> usize {
    source[index..]
        .find('\n')
        .map_or(source.len(), |newline| index + newline + 1)
}

fn line_for_byte(source: &str, byte: usize) -> usize {
    source[..byte].bytes().filter(|byte| *byte == b'\n').count() + 1
}
