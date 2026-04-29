use camino::{Utf8Path, Utf8PathBuf};
use tree_sitter::{Language as TsLanguage, Node, Parser};

use crate::{
    command::{read_to_string, relative_path},
    model::{Language, SourceRange, Symbol, SymbolKind, Visibility},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileOutline {
    pub source_bytes: usize,
    pub symbols: Vec<Symbol>,
}

pub fn outline_file(
    path: &Utf8Path,
    cwd: &Utf8Path,
    language: Language,
    public_only: bool,
) -> std::result::Result<FileOutline, String> {
    let source = read_to_string(path).map_err(|err| err.to_string())?;
    let source_bytes = source.len();
    let spec = spec(language);
    let mut parser = Parser::new();
    parser
        .set_language(&spec.ts_language)
        .map_err(|err| err.to_string())?;
    let tree = parser
        .parse(&source, None)
        .ok_or_else(|| "tree-sitter parser returned no tree".to_owned())?;
    if tree.root_node().has_error() {
        return Err("source contains syntax errors".to_owned());
    }

    let mut symbols = Vec::new();
    let relative = relative_path(path, cwd);
    let root = tree.root_node();
    visit(root, &source, &relative, &spec, None, public_only, &mut symbols);
    Ok(FileOutline {
        source_bytes,
        symbols,
    })
}

#[derive(Debug, Clone)]
struct LanguageSpec {
    language: Language,
    ts_language: TsLanguage,
    declarations: &'static [Declaration],
    container_kinds: &'static [&'static str],
}

#[derive(Debug, Clone, Copy)]
struct Declaration {
    node_kind: &'static str,
    symbol_kind: SymbolKind,
    default_visibility: Visibility,
}

fn spec(language: Language) -> LanguageSpec {
    match language {
        Language::Go => LanguageSpec {
            language,
            ts_language: tree_sitter_go::LANGUAGE.into(),
            declarations: GO_DECLS,
            container_kinds: GO_CONTAINERS,
        },
        Language::Java => LanguageSpec {
            language,
            ts_language: tree_sitter_java::LANGUAGE.into(),
            declarations: JAVA_DECLS,
            container_kinds: JAVA_CONTAINERS,
        },
        Language::Javascript => LanguageSpec {
            language,
            ts_language: tree_sitter_javascript::LANGUAGE.into(),
            declarations: JS_DECLS,
            container_kinds: JS_CONTAINERS,
        },
        Language::Php => LanguageSpec {
            language,
            ts_language: tree_sitter_php::LANGUAGE_PHP.into(),
            declarations: PHP_DECLS,
            container_kinds: PHP_CONTAINERS,
        },
        Language::Python => LanguageSpec {
            language,
            ts_language: tree_sitter_python::LANGUAGE.into(),
            declarations: PY_DECLS,
            container_kinds: PY_CONTAINERS,
        },
        Language::Rust => LanguageSpec {
            language,
            ts_language: tree_sitter_rust::LANGUAGE.into(),
            declarations: RUST_DECLS,
            container_kinds: RUST_CONTAINERS,
        },
        Language::Typescript => LanguageSpec {
            language,
            ts_language: tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            declarations: TS_DECLS,
            container_kinds: TS_CONTAINERS,
        },
    }
}

const RUST_DECLS: &[Declaration] = &[
    decl("const_item", SymbolKind::Const, Visibility::Private),
    decl("enum_item", SymbolKind::Enum, Visibility::Private),
    decl("function_item", SymbolKind::Fn, Visibility::Private),
    decl("impl_item", SymbolKind::Impl, Visibility::Private),
    decl("macro_definition", SymbolKind::Macro, Visibility::Private),
    decl("mod_item", SymbolKind::Mod, Visibility::Private),
    decl("static_item", SymbolKind::Static, Visibility::Private),
    decl("struct_item", SymbolKind::Struct, Visibility::Private),
    decl("trait_item", SymbolKind::Trait, Visibility::Private),
    decl("type_item", SymbolKind::Type, Visibility::Private),
    decl("use_declaration", SymbolKind::Use, Visibility::Private),
];
const RUST_CONTAINERS: &[&str] = &["impl_item", "mod_item", "trait_item"];

const TS_DECLS: &[Declaration] = &[
    decl("class_declaration", SymbolKind::Class, Visibility::Pub),
    decl("enum_declaration", SymbolKind::Enum, Visibility::Pub),
    decl("function_declaration", SymbolKind::Fn, Visibility::Pub),
    decl("interface_declaration", SymbolKind::Interface, Visibility::Pub),
    decl("lexical_declaration", SymbolKind::Const, Visibility::Pub),
    decl("method_definition", SymbolKind::Method, Visibility::Pub),
    decl("method_signature", SymbolKind::Method, Visibility::Pub),
    decl("type_alias_declaration", SymbolKind::Type, Visibility::Pub),
    decl("variable_declaration", SymbolKind::Var, Visibility::Pub),
];
const TS_CONTAINERS: &[&str] = &["class_declaration", "interface_declaration"];

const JS_DECLS: &[Declaration] = &[
    decl("class_declaration", SymbolKind::Class, Visibility::Pub),
    decl("function_declaration", SymbolKind::Fn, Visibility::Pub),
    decl("lexical_declaration", SymbolKind::Const, Visibility::Pub),
    decl("method_definition", SymbolKind::Method, Visibility::Pub),
    decl("variable_declaration", SymbolKind::Var, Visibility::Pub),
];
const JS_CONTAINERS: &[&str] = &["class_declaration"];

const PY_DECLS: &[Declaration] = &[
    decl("class_definition", SymbolKind::Class, Visibility::Pub),
    decl("function_definition", SymbolKind::Fn, Visibility::Pub),
];
const PY_CONTAINERS: &[&str] = &["class_definition"];

const GO_DECLS: &[Declaration] = &[
    decl("const_declaration", SymbolKind::Const, Visibility::Private),
    decl("function_declaration", SymbolKind::Fn, Visibility::Private),
    decl("method_declaration", SymbolKind::Method, Visibility::Private),
    decl("package_clause", SymbolKind::Package, Visibility::Pub),
    decl("type_declaration", SymbolKind::Type, Visibility::Private),
    decl("var_declaration", SymbolKind::Var, Visibility::Private),
];
const GO_CONTAINERS: &[&str] = &[];

const JAVA_DECLS: &[Declaration] = &[
    decl("class_declaration", SymbolKind::Class, Visibility::Package),
    decl("enum_declaration", SymbolKind::Enum, Visibility::Package),
    decl("interface_declaration", SymbolKind::Interface, Visibility::Package),
    decl("method_declaration", SymbolKind::Method, Visibility::Package),
    decl("package_declaration", SymbolKind::Package, Visibility::Pub),
];
const JAVA_CONTAINERS: &[&str] = &["class_declaration", "enum_declaration", "interface_declaration"];

const PHP_DECLS: &[Declaration] = &[
    decl("class_declaration", SymbolKind::Class, Visibility::Pub),
    decl("function_definition", SymbolKind::Fn, Visibility::Pub),
    decl("interface_declaration", SymbolKind::Interface, Visibility::Pub),
    decl("method_declaration", SymbolKind::Method, Visibility::Pub),
    decl("namespace_definition", SymbolKind::Namespace, Visibility::Pub),
    decl("trait_declaration", SymbolKind::Trait, Visibility::Pub),
];
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
    path: &Utf8PathBuf,
    spec: &LanguageSpec,
    parent: Option<&str>,
    public_only: bool,
    symbols: &mut Vec<Symbol>,
) {
    let declaration = declaration_for(spec, node.kind());
    let symbol_name = declaration.and_then(|decl| name_for(spec.language, node, source, decl));
    let mut next_parent = parent;

    if let (Some(decl), Some(name)) = (declaration, symbol_name.as_deref()) {
        let visibility = visibility_for(spec.language, node, source, decl.default_visibility, name);
        if !public_only || visibility.is_publicish() {
            symbols.push(Symbol {
                path: path.clone(),
                language: spec.language,
                kind: kind_for(spec.language, node, source, decl.symbol_kind),
                visibility,
                name: name.to_owned(),
                signature: signature(spec.language, node, source),
                docs: docs_before(node, source),
                range: SourceRange {
                    start_line: node.start_position().row + 1,
                    end_line: node.end_position().row + 1,
                },
                parent: parent.map(str::to_owned),
            });
        }
        if spec.container_kinds.contains(&node.kind()) {
            next_parent = Some(name);
        } else {
            return;
        }
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        visit(child, source, path, spec, next_parent, public_only, symbols);
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
    if matches!(decl.symbol_kind, SymbolKind::Use) {
        return Some(
            signature(language, node, source)
                .trim_start_matches("use ")
                .to_owned(),
        );
    }
    if matches!(decl.symbol_kind, SymbolKind::Package | SymbolKind::Namespace) {
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
        Language::Go if node.kind() == "package_clause" => Visibility::Pub,
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

fn docs_before(node: Node<'_>, source: &str) -> Option<String> {
    let before = &source[..line_start(source, node.start_byte())];
    let mut docs = Vec::new();
    for line in before.lines().rev() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if docs.is_empty() {
                continue;
            }
            break;
        }
        if let Some(doc) = doc_line(trimmed) {
            docs.push(doc);
        } else {
            break;
        }
    }
    if docs.is_empty() {
        None
    } else {
        docs.reverse();
        Some(docs.join(" "))
    }
}

fn doc_line(line: &str) -> Option<String> {
    if line.starts_with("//!") || line.starts_with("/*!") {
        return None;
    }
    let stripped = line
        .strip_prefix("///")
        .or_else(|| line.strip_prefix("//"))
        .or_else(|| line.strip_prefix('#'))
        .or_else(|| line.strip_prefix('*'))?
        .trim()
        .trim_start_matches('*')
        .trim_matches('/')
        .trim();
    if stripped.is_empty() || stripped == "<?php" {
        None
    } else {
        Some(stripped.to_owned())
    }
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
