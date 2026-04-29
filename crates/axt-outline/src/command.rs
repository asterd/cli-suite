use std::fs;

use axt_core::CommandContext;
use camino::{Utf8Path, Utf8PathBuf};
use walkdir::WalkDir;

use crate::{
    cli::{Args, SortArg},
    error::{OutlineError, Result},
    model::{Language, OutlineData, OutlineSummary, OutlineWarning, WarningCode},
    tree::outline_file,
};

pub fn run(args: &Args, ctx: &CommandContext) -> Result<OutlineData> {
    let mut warnings = Vec::new();
    let mut files = Vec::new();
    for path in &args.paths {
        let absolute = absolutize(path, &ctx.cwd);
        if !absolute.exists() {
            return Err(OutlineError::PathNotFound(path.clone()));
        }
        collect_path(args, &absolute, &ctx.cwd, &mut files, &mut warnings)?;
    }
    files.sort();
    files.dedup();

    let mut symbols = Vec::new();
    let mut parsed_files = 0;
    let mut source_bytes = 0;
    let public_only = args.public_only && !args.private;
    for file in files {
        let language = language_for_path(&file);
        let Some(language) = language else {
            continue;
        };
        let result = outline_file(&file, &ctx.cwd, language, public_only);
        match result {
            Ok(mut outline) => {
                parsed_files += 1;
                source_bytes += outline.source_bytes;
                symbols.append(&mut outline.symbols);
            }
            Err(message) => warnings.push(OutlineWarning {
                code: WarningCode::ParseError,
                path: Some(relative_path(&file, &ctx.cwd)),
                message,
            }),
        }
    }

    if parsed_files == 0 {
        return Err(OutlineError::NoSupportedFiles);
    }

    sort_symbols(&mut symbols, args.sort);
    let next = symbols
        .first()
        .map(|symbol| {
            vec![format!(
                "axt-slice {} --symbol {} --agent",
                symbol.path, symbol.name
            )]
        })
        .unwrap_or_default();
    let summary = OutlineSummary {
        files: parsed_files,
        symbols: symbols.len(),
        warnings: warnings.len(),
        source_bytes,
        signature_bytes: symbols.iter().map(|symbol| symbol.signature.len()).sum(),
        truncated: false,
    };
    Ok(OutlineData {
        root: ".".into(),
        summary,
        symbols,
        warnings,
        next,
    })
}

fn collect_path(
    args: &Args,
    path: &Utf8Path,
    cwd: &Utf8Path,
    files: &mut Vec<Utf8PathBuf>,
    warnings: &mut Vec<OutlineWarning>,
) -> Result<()> {
    if path.is_file() {
        collect_file(selected_language(args), path, cwd, files, warnings);
        return Ok(());
    }

    let walker = WalkDir::new(path)
        .follow_links(false)
        .max_depth(args.max_depth)
        .sort_by_file_name();
    for entry in walker {
        let entry = entry?;
        if entry.file_type().is_file() {
            let file = Utf8PathBuf::from_path_buf(entry.path().to_path_buf())
                .map_err(|path| OutlineError::Io {
                    path: path.to_string_lossy().to_string().into(),
                    source: std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "path is not valid UTF-8",
                    ),
                })?;
            collect_file(selected_language(args), &file, cwd, files, warnings);
        }
    }
    Ok(())
}

fn collect_file(
    selected: Option<Language>,
    path: &Utf8Path,
    cwd: &Utf8Path,
    files: &mut Vec<Utf8PathBuf>,
    warnings: &mut Vec<OutlineWarning>,
) {
    if let Some(language) = language_for_path(path) {
        if selected.is_some_and(|selected| selected != language) {
            warnings.push(unsupported_warning(path, cwd));
            return;
        }
        files.push(path.to_owned());
    } else {
        warnings.push(unsupported_warning(path, cwd));
    }
}

fn unsupported_warning(path: &Utf8Path, cwd: &Utf8Path) -> OutlineWarning {
    OutlineWarning {
        code: WarningCode::UnsupportedLanguage,
        path: Some(relative_path(path, cwd)),
        message: "file extension is not supported by axt-outline".to_owned(),
    }
}

fn selected_language(args: &Args) -> Option<Language> {
    args.lang.map(crate::cli::LanguageArg::into_language)
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

pub fn read_to_string(path: &Utf8Path) -> Result<String> {
    fs::read_to_string(path).map_err(|source| OutlineError::Io {
        path: path.to_owned(),
        source,
    })
}

pub fn relative_path(path: &Utf8Path, cwd: &Utf8Path) -> Utf8PathBuf {
    path.strip_prefix(cwd).map_or_else(|_| path.to_owned(), Utf8Path::to_owned)
}

fn sort_symbols(symbols: &mut [crate::model::Symbol], sort: SortArg) {
    match sort {
        SortArg::Source => {}
        SortArg::Path => symbols.sort_by(|left, right| {
            left.path
                .cmp(&right.path)
                .then(left.range.start_line.cmp(&right.range.start_line))
                .then(left.name.cmp(&right.name))
        }),
        SortArg::Name => symbols.sort_by(|left, right| {
            left.name
                .cmp(&right.name)
                .then(left.path.cmp(&right.path))
                .then(left.range.start_line.cmp(&right.range.start_line))
        }),
        SortArg::Kind => symbols.sort_by(|left, right| {
            left.kind
                .as_str()
                .cmp(right.kind.as_str())
                .then(left.path.cmp(&right.path))
                .then(left.range.start_line.cmp(&right.range.start_line))
        }),
    }
}
