use std::{collections::HashSet, fs};

use axt_core::CommandContext;
use camino::{Utf8Path, Utf8PathBuf};

use crate::{
    cli::{Args, IncludeImports},
    error::{Result, SliceError},
    model::{
        Language, Selection, SelectionKind, SliceCandidate, SliceData, SliceStatus, SliceSummary,
        SliceSymbol, SourceRange,
    },
    tree::{parse_source, ParsedFile, ParsedSymbol, SourceSpan},
};

pub fn run(args: &Args, ctx: &CommandContext) -> Result<SliceData> {
    let file = args.file.as_ref().ok_or(SliceError::MissingFile)?;
    let absolute = absolutize(file, &ctx.cwd);
    if !absolute.exists() {
        return Err(SliceError::PathNotFound(file.clone()));
    }
    if !absolute.is_file() {
        return Err(SliceError::NotAFile(file.clone()));
    }
    let language =
        language_for_path(&absolute).ok_or_else(|| SliceError::UnsupportedLanguage(file.clone()))?;
    let source = read_source(&absolute)?;
    let relative = relative_path(&absolute, &ctx.cwd);
    let parsed = parse_source(&absolute, &relative, language, source).map_err(SliceError::Parse)?;
    select(args, relative, language, parsed)
}

fn select(
    args: &Args,
    path: Utf8PathBuf,
    language: Language,
    parsed: ParsedFile,
) -> Result<SliceData> {
    let selection = selection(args)?;
    let mut symbols = parsed.symbols;
    symbols.sort_by_key(|symbol| (symbol.symbol_span.start_byte, symbol.symbol_span.end_byte));

    let matches = match &selection.kind {
        SelectionKind::Symbol => symbols
            .iter()
            .filter(|symbol| matches_query(&selection.query, symbol))
            .collect::<Vec<_>>(),
        SelectionKind::Line => {
            let line = selection
                .query
                .parse::<usize>()
                .map_err(|_| SliceError::InvalidLine)?;
            symbols
                .iter()
                .filter(|symbol| contains_line(&symbol.symbol_span.range, line))
                .max_by_key(|symbol| {
                    (
                        symbol.symbol_span.range.start_line,
                        usize::MAX - symbol.symbol_span.range.end_line,
                    )
                })
                .map_or_else(Vec::new, |symbol| vec![symbol])
        }
    };

    if matches.is_empty() && matches!(selection.kind, SelectionKind::Line) {
        let line = selection
            .query
            .parse::<usize>()
            .map_err(|_| SliceError::InvalidLine)?;
        let extended = symbols
            .iter()
            .filter(|symbol| contains_line(&symbol.span.range, line))
            .max_by_key(|symbol| {
                (
                    symbol.span.range.start_line,
                    usize::MAX - symbol.span.range.end_line,
                )
            })
            .map_or_else(Vec::new, |symbol| vec![symbol]);
        let ctx = SelectionContext {
            path,
            language,
            source: &parsed.source,
            imports: &parsed.imports,
            symbols: &symbols,
        };
        return Ok(selected_or_candidates(args, &ctx, selection, &extended));
    }

    let ctx = SelectionContext {
        path,
        language,
        source: &parsed.source,
        imports: &parsed.imports,
        symbols: &symbols,
    };
    Ok(selected_or_candidates(args, &ctx, selection, &matches))
}

struct SelectionContext<'a> {
    path: Utf8PathBuf,
    language: Language,
    source: &'a str,
    imports: &'a [SourceSpan],
    symbols: &'a [ParsedSymbol],
}

fn selected_or_candidates(
    args: &Args,
    ctx: &SelectionContext<'_>,
    selection: Selection,
    matches: &[&ParsedSymbol],
) -> SliceData {
    if matches.len() == 1 {
        let selected = matches[0];
        let spans = selected_spans(args, ctx.source, selected, ctx.imports, ctx.symbols);
        let source_text = extract_spans(ctx.source, &spans);
        let source_bytes = source_text.len();
        return SliceData {
            path: ctx.path.clone(),
            language: ctx.language,
            selection,
            status: SliceStatus::Selected,
            summary: SliceSummary {
                matches: 1,
                candidates: 0,
                source_bytes,
                truncated: false,
            },
            symbol: Some(slice_symbol(selected)),
            range: Some(selected.span.range),
            spans: spans.iter().map(|span| span.range).collect(),
            source: Some(source_text),
            candidates: Vec::new(),
            warnings: Vec::new(),
            next: Vec::new(),
        };
    }

    let candidates = matches
        .iter()
        .copied()
        .map(candidate)
        .collect::<Vec<_>>();
    let status = if candidates.is_empty() {
        SliceStatus::NotFound
    } else {
        SliceStatus::Ambiguous
    };
    let next = if candidates.is_empty() {
        vec!["axt-outline <path> --agent".to_owned()]
    } else {
        vec![format!("axt-outline {} --agent", ctx.path)]
    };
    SliceData {
        path: ctx.path.clone(),
        language: ctx.language,
        selection,
        status,
        summary: SliceSummary {
            matches: matches.len(),
            candidates: candidates.len(),
            source_bytes: 0,
            truncated: false,
        },
        symbol: None,
        range: None,
        spans: Vec::new(),
        source: None,
        candidates,
        warnings: Vec::new(),
        next,
    }
}

fn selected_spans(
    args: &Args,
    source: &str,
    selected: &ParsedSymbol,
    imports: &[SourceSpan],
    symbols: &[ParsedSymbol],
) -> Vec<SourceSpan> {
    let mut spans = Vec::new();
    if let Some(mode) = args.include_imports {
        spans.extend(import_spans(mode, source, selected, imports));
    }
    if args.before_symbol {
        if let Some(before) = adjacent_before(selected, symbols) {
            spans.push(before.span);
        }
    }
    spans.push(selected.span);
    if args.after_symbol {
        if let Some(after) = adjacent_after(selected, symbols) {
            spans.push(after.span);
        }
    }
    if args.include_tests {
        spans.extend(
            symbols
                .iter()
                .filter(|symbol| symbol.is_test && symbol.span != selected.span)
                .map(|symbol| symbol.span),
        );
    }
    spans.sort_by_key(|span| (span.start_byte, span.end_byte));
    spans.dedup_by_key(|span| (span.start_byte, span.end_byte));
    spans
}

fn import_spans(
    mode: IncludeImports,
    source: &str,
    selected: &ParsedSymbol,
    imports: &[SourceSpan],
) -> Vec<SourceSpan> {
    let prior_imports = imports
        .iter()
        .copied()
        .filter(|span| span.end_byte <= selected.span.start_byte);
    match mode {
        IncludeImports::All => prior_imports.collect(),
        IncludeImports::Matched => {
            let used = collect_identifiers(&source[selected.span.start_byte..selected.span.end_byte]);
            prior_imports
                .filter(|span| import_matches(source, *span, &used))
                .collect()
        }
    }
}

fn import_matches(source: &str, span: SourceSpan, used: &HashSet<String>) -> bool {
    collect_identifiers(&source[span.start_byte..span.end_byte])
        .into_iter()
        .filter(|identifier| !IMPORT_WORDS.contains(&identifier.as_str()))
        .any(|identifier| used.contains(&identifier))
}

fn collect_identifiers(source: &str) -> HashSet<String> {
    let mut identifiers = HashSet::new();
    let mut current = String::new();
    let mut quote = None;
    for ch in source.chars() {
        if let Some(end_quote) = quote {
            if ch == end_quote {
                quote = None;
            }
            continue;
        }
        if matches!(ch, '"' | '\'' | '`') {
            push_identifier(&mut identifiers, &mut current);
            quote = Some(ch);
        } else if is_identifier_continue(ch) {
            current.push(ch);
        } else {
            push_identifier(&mut identifiers, &mut current);
        }
    }
    push_identifier(&mut identifiers, &mut current);
    identifiers
}

fn push_identifier(identifiers: &mut HashSet<String>, current: &mut String) {
    if current
        .chars()
        .next()
        .is_some_and(is_identifier_start)
        && !IMPORT_WORDS.contains(&current.as_str())
    {
        identifiers.insert(current.clone());
    }
    current.clear();
}

const fn is_identifier_start(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphabetic()
}

const fn is_identifier_continue(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphanumeric()
}

const IMPORT_WORDS: &[&str] = &[
    "as",
    "crate",
    "from",
    "import",
    "package",
    "pub",
    "self",
    "super",
    "type",
    "use",
];

fn adjacent_before<'a>(
    selected: &ParsedSymbol,
    symbols: &'a [ParsedSymbol],
) -> Option<&'a ParsedSymbol> {
    symbols
        .iter()
        .filter(|symbol| symbol.span.end_byte <= selected.span.start_byte)
        .max_by_key(|symbol| symbol.span.end_byte)
}

fn adjacent_after<'a>(
    selected: &ParsedSymbol,
    symbols: &'a [ParsedSymbol],
) -> Option<&'a ParsedSymbol> {
    symbols
        .iter()
        .filter(|symbol| symbol.span.start_byte >= selected.span.end_byte)
        .min_by_key(|symbol| symbol.span.start_byte)
}

fn extract_spans(source: &str, spans: &[SourceSpan]) -> String {
    let mut output = String::new();
    for span in spans {
        if !output.is_empty() && !output.ends_with('\n') {
            output.push('\n');
        }
        output.push_str(&source[span.start_byte..span.end_byte]);
    }
    output
}

fn selection(args: &Args) -> Result<Selection> {
    if let Some(symbol) = &args.symbol {
        return Ok(Selection {
            kind: SelectionKind::Symbol,
            query: symbol.clone(),
        });
    }
    let line = args.line.ok_or(SliceError::MissingSelector)?;
    if line == 0 {
        return Err(SliceError::InvalidLine);
    }
    Ok(Selection {
        kind: SelectionKind::Line,
        query: line.to_string(),
    })
}

fn matches_query(query: &str, symbol: &ParsedSymbol) -> bool {
    query == symbol.name
        || query == symbol.qualified_name
        || query == format!("{}::{}", symbol.kind.as_str(), symbol.name)
        || query == format!("{}::{}", symbol.kind.as_str(), symbol.qualified_name)
}

const fn contains_line(range: &SourceRange, line: usize) -> bool {
    range.start_line <= line && line <= range.end_line
}

fn slice_symbol(symbol: &ParsedSymbol) -> SliceSymbol {
    SliceSymbol {
        name: symbol.name.clone(),
        qualified_name: symbol.qualified_name.clone(),
        kind: symbol.kind,
        visibility: symbol.visibility,
        range: symbol.symbol_span.range,
        parent: symbol.parent.clone(),
    }
}

fn candidate(symbol: &ParsedSymbol) -> SliceCandidate {
    SliceCandidate {
        name: symbol.name.clone(),
        qualified_name: symbol.qualified_name.clone(),
        kind: symbol.kind,
        visibility: symbol.visibility,
        range: symbol.symbol_span.range,
        parent: symbol.parent.clone(),
    }
}

fn read_source(path: &Utf8Path) -> Result<String> {
    let bytes = fs::read(path).map_err(|source| SliceError::Io {
        path: path.to_owned(),
        source,
    })?;
    if !axt_fs::is_text_bytes(&bytes) {
        return Err(SliceError::NonUtf8(path.to_owned()));
    }
    String::from_utf8(bytes).map_err(|_| SliceError::NonUtf8(path.to_owned()))
}

fn language_for_path(path: &Utf8Path) -> Option<Language> {
    match path.extension()? {
        "go" => Some(Language::Go),
        "java" => Some(Language::Java),
        "js" | "jsx" | "mjs" | "cjs" => Some(Language::Javascript),
        "php" => Some(Language::Php),
        "py" => Some(Language::Python),
        "rs" => Some(Language::Rust),
        "ts" | "tsx" | "mts" | "cts" => Some(Language::Typescript),
        _ => None,
    }
}

fn absolutize(path: &Utf8Path, cwd: &Utf8Path) -> Utf8PathBuf {
    if path.is_absolute() {
        path.to_owned()
    } else {
        cwd.join(path)
    }
}

fn relative_path(path: &Utf8Path, cwd: &Utf8Path) -> Utf8PathBuf {
    path.strip_prefix(cwd).map_or_else(|_| path.to_owned(), Utf8Path::to_owned)
}
