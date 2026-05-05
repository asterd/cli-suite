use std::{collections::BTreeSet, fs, time::Instant};

use axt_core::{compile_user_regex, CommandContext, CoreError, UserRegexError, UserRegexLimits};
use axt_fs::{ContentKind, FsWarningCode, WalkOptions};
use camino::{Utf8Path, Utf8PathBuf};
use globset::{Glob, GlobSet, GlobSetBuilder};
use regex::Regex;

use crate::{
    ast,
    cli::Args,
    error::{CtxpackError, Result},
    model::{
        ByteRange, CtxpackData, CtxpackSummary, CtxpackWarning, PatternKind, SearchHit,
        SearchPattern, WarningCode,
    },
};

struct CompiledPattern {
    spec: SearchPattern,
    regex: Regex,
}

struct SourceLine<'a> {
    number: usize,
    start: usize,
    text: &'a str,
}

pub fn run(args: &Args, ctx: &CommandContext) -> Result<CtxpackData> {
    let patterns = compile_patterns(&args.patterns)?;
    let includes = compile_includes(args)?;
    let mut warnings = Vec::new();
    let mut files = collect_files(args, ctx, includes.as_ref(), &mut warnings)?;
    files.sort();
    files.dedup();

    let mut hits = Vec::new();
    let mut matched_files = BTreeSet::new();
    let mut files_scanned = 0;
    let mut bytes_scanned = 0_u64;
    let mut truncated = false;
    let started = Instant::now();

    for file in files {
        if ctx
            .max_duration
            .is_some_and(|max_duration| started.elapsed() >= max_duration)
        {
            truncated = true;
            break;
        }
        if hits.len() >= ctx.limits.max_records {
            truncated = true;
            break;
        }
        let bytes = match fs::read(&file) {
            Ok(bytes) => bytes,
            Err(source) if source.kind() == std::io::ErrorKind::PermissionDenied => {
                warnings.push(CtxpackWarning {
                    code: WarningCode::PermissionDenied,
                    path: Some(relative_path(&file, &ctx.cwd)),
                    message: source.to_string(),
                });
                continue;
            }
            Err(source) => {
                return Err(CtxpackError::Io { path: file, source });
            }
        };
        if !axt_fs::is_text_bytes(&bytes) {
            warnings.push(CtxpackWarning {
                code: WarningCode::BinarySkipped,
                path: Some(relative_path(&file, &ctx.cwd)),
                message: "binary file skipped".to_owned(),
            });
            continue;
        }
        let text = match std::str::from_utf8(&bytes) {
            Ok(text) => text,
            Err(_err) => {
                warnings.push(CtxpackWarning {
                    code: WarningCode::NonUtf8Skipped,
                    path: Some(relative_path(&file, &ctx.cwd)),
                    message: "file is not valid UTF-8".to_owned(),
                });
                continue;
            }
        };
        files_scanned += 1;
        bytes_scanned += bytes.len() as u64;
        let before = hits.len();
        search_file(
            &file,
            &ctx.cwd,
            text,
            &patterns,
            args.context,
            ctx.limits.max_records,
            &mut hits,
        );
        if hits.len() > before {
            matched_files.insert(relative_path(&file, &ctx.cwd));
        }
        if hits.len() >= ctx.limits.max_records {
            truncated = true;
        }
    }

    let pattern_specs: Vec<SearchPattern> =
        patterns.into_iter().map(|pattern| pattern.spec).collect();
    let next = next_suggestions(&hits, &pattern_specs, args.context);
    let summary = CtxpackSummary {
        roots: args.roots.len(),
        files_scanned,
        files_matched: matched_files.len(),
        hits: hits.len(),
        warnings: warnings.len(),
        bytes_scanned,
        truncated,
    };
    Ok(CtxpackData {
        root: ".".into(),
        patterns: pattern_specs,
        summary,
        hits,
        warnings,
        next,
    })
}

fn compile_patterns(values: &[String]) -> Result<Vec<CompiledPattern>> {
    if values.is_empty() {
        return Err(CtxpackError::MissingPattern);
    }

    let mut patterns = Vec::new();
    let mut names = BTreeSet::new();
    for value in values {
        let Some((name, query)) = value.split_once('=') else {
            return Err(CtxpackError::InvalidPatternShape {
                pattern: value.clone(),
            });
        };
        if name.is_empty()
            || !name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
        {
            return Err(CtxpackError::InvalidPatternName(name.to_owned()));
        }
        if !names.insert(name.to_owned()) {
            return Err(CtxpackError::DuplicatePatternName(name.to_owned()));
        }
        let regex =
            compile_user_regex(query, UserRegexLimits::default()).map_err(|source| match source {
                UserRegexError::Core(CoreError::RegexTooLong { max_len }) => {
                    CtxpackError::RegexTooLong {
                        name: name.to_owned(),
                        max_len,
                    }
                }
                UserRegexError::Core(err) => CtxpackError::InvalidRegex {
                    name: name.to_owned(),
                    source: regex::Error::Syntax(err.to_string()),
                },
                UserRegexError::Regex(source) => CtxpackError::InvalidRegex {
                    name: name.to_owned(),
                    source,
                },
            })?;
        patterns.push(CompiledPattern {
            spec: SearchPattern {
                name: name.to_owned(),
                query: query.to_owned(),
                kind: PatternKind::Regex,
            },
            regex,
        });
    }
    Ok(patterns)
}

fn compile_includes(args: &Args) -> Result<Option<GlobSet>> {
    let globs = args.files.iter().chain(args.includes.iter());
    let mut builder = GlobSetBuilder::new();
    let mut count = 0_usize;
    for glob in globs {
        builder.add(Glob::new(glob).map_err(|source| CtxpackError::InvalidGlob {
            glob: glob.clone(),
            source,
        })?);
        count += 1;
    }
    if count == 0 {
        Ok(None)
    } else {
        Ok(Some(builder.build().map_err(|source| CtxpackError::InvalidGlob {
            glob: "<combined>".to_owned(),
            source,
        })?))
    }
}

fn collect_files(
    args: &Args,
    ctx: &CommandContext,
    includes: Option<&GlobSet>,
    warnings: &mut Vec<CtxpackWarning>,
) -> Result<Vec<Utf8PathBuf>> {
    let mut files = Vec::new();
    for root in &args.roots {
        let absolute = absolutize(root, &ctx.cwd);
        if !absolute.exists() {
            return Err(CtxpackError::PathNotFound(root.clone()));
        }
        if absolute.is_file() {
            if include_file(&absolute, &ctx.cwd, includes) {
                files.push(absolute);
            }
            continue;
        }
        let collection = axt_fs::collect_metadata_with_warnings(
            &absolute,
            WalkOptions {
                max_depth: Some(args.max_depth),
                files_only: true,
                dirs_only: false,
                include_hidden: args.hidden,
                no_ignore: args.no_ignore,
                cross_fs: false,
                follow_symlinks: false,
                max_file_size: None,
                hash: None,
            },
        )?;
        for warning in collection.warnings {
            warnings.push(fs_warning(&absolute, warning));
        }
        for entry in collection.entries {
            if entry.content == ContentKind::Binary {
                warnings.push(CtxpackWarning {
                    code: WarningCode::BinarySkipped,
                    path: Some(relative_path(&absolute.join(&entry.path), &ctx.cwd)),
                    message: "binary file skipped".to_owned(),
                });
                continue;
            }
            let file = absolute.join(entry.path);
            if include_file(&file, &ctx.cwd, includes) {
                files.push(file);
            }
        }
    }
    Ok(files)
}

fn fs_warning(root: &Utf8Path, warning: axt_fs::FsWarning) -> CtxpackWarning {
    let code = match warning.code {
        FsWarningCode::PermissionDenied => WarningCode::PermissionDenied,
        FsWarningCode::SymlinkLoop => WarningCode::Walk,
        FsWarningCode::PathNotUtf8 => WarningCode::PathNotUtf8,
    };
    CtxpackWarning {
        code,
        path: warning.path.map(|path| root.join(path)),
        message: warning.reason,
    }
}

fn include_file(path: &Utf8Path, cwd: &Utf8Path, includes: Option<&GlobSet>) -> bool {
    includes.is_none_or(|set| {
        let relative = relative_path(path, cwd);
        set.is_match(relative.as_std_path()) || set.is_match(path.as_std_path())
    })
}

fn search_file(
    file: &Utf8Path,
    cwd: &Utf8Path,
    text: &str,
    patterns: &[CompiledPattern],
    context: usize,
    limit: usize,
    hits: &mut Vec<SearchHit>,
) {
    let lines = source_lines(text);
    for pattern in patterns {
        for matched in pattern.regex.find_iter(text) {
            if hits.len() >= limit {
                return;
            }
            let line_index = line_index_for(&lines, matched.start());
            let Some(line) = lines.get(line_index) else {
                continue;
            };
            let column = text[line.start..matched.start()].chars().count() + 1;
            let path = relative_path(file, cwd);
            let classification =
                ast::classify_hit(&path, text, matched.start(), matched.end(), line.text);
            hits.push(SearchHit {
                pattern: pattern.spec.name.clone(),
                path: path.clone(),
                line: line.number,
                column,
                byte_range: ByteRange {
                    start: matched.start(),
                    end: matched.end(),
                },
                kind: classification.kind,
                classification_source: classification.source,
                language: classification.language,
                node_kind: classification.node_kind,
                enclosing_symbol: classification.enclosing_symbol,
                ast_path: classification.ast_path,
                matched_text: matched.as_str().to_owned(),
                snippet: snippet(&lines, line_index, context),
            });
        }
    }
}

fn source_lines(text: &str) -> Vec<SourceLine<'_>> {
    let mut lines = Vec::new();
    let mut start = 0_usize;
    for (index, line) in text.split_inclusive('\n').enumerate() {
        let trimmed = line.strip_suffix('\n').unwrap_or(line);
        let trimmed = trimmed.strip_suffix('\r').unwrap_or(trimmed);
        lines.push(SourceLine {
            number: index + 1,
            start,
            text: trimmed,
        });
        start += line.len();
    }
    if text.is_empty() {
        lines.push(SourceLine {
            number: 1,
            start: 0,
            text: "",
        });
    } else if !text.ends_with('\n') && lines.is_empty() {
        lines.push(SourceLine {
            number: 1,
            start: 0,
            text,
        });
    }
    lines
}

fn line_index_for(lines: &[SourceLine<'_>], byte: usize) -> usize {
    match lines.binary_search_by(|line| line.start.cmp(&byte)) {
        Ok(index) => index,
        Err(0) => 0,
        Err(index) => index - 1,
    }
}

fn snippet(lines: &[SourceLine<'_>], line_index: usize, context: usize) -> String {
    let start = line_index.saturating_sub(context);
    let end = (line_index + context + 1).min(lines.len());
    lines[start..end]
        .iter()
        .map(|line| format!("{}:{}", line.number, line.text))
        .collect::<Vec<_>>()
        .join("\n")
}

fn absolutize(path: &Utf8Path, cwd: &Utf8Path) -> Utf8PathBuf {
    if path.is_absolute() {
        path.to_owned()
    } else {
        cwd.join(path)
    }
}

pub fn relative_path(path: &Utf8Path, cwd: &Utf8Path) -> Utf8PathBuf {
    path.strip_prefix(cwd).map_or_else(|_| path.to_owned(), |path| {
        Utf8PathBuf::from(path.as_str().replace('\\', "/"))
    })
}

fn pattern_query(patterns: &[SearchPattern], name: &str) -> String {
    patterns
        .iter()
        .find(|pattern| pattern.name == name)
        .map_or_else(String::new, |pattern| pattern.query.clone())
}

fn next_suggestions(
    hits: &[SearchHit],
    patterns: &[SearchPattern],
    context: usize,
) -> Vec<String> {
    hits.first()
        .map(|hit| {
            vec![format!(
                "axt-ctxpack {} --pattern {} --context {} --agent",
                shell_quote(hit.path.as_str()),
                shell_quote(&format!("{}={}", hit.pattern, pattern_query(patterns, &hit.pattern))),
                context + 2
            )]
        })
        .unwrap_or_default()
}

fn shell_quote(value: &str) -> String {
    if value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'.' | b'_' | b'-'))
    {
        value.to_owned()
    } else {
        format!("'{}'", value.replace('\'', "'\\''"))
    }
}
