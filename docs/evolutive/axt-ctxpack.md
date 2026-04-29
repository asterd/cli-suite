# axt-ctxpack Evolution Brief

Status: proposed. Requires spec approval before implementation.

## Purpose

`axt-ctxpack` performs multi-pattern, multi-file context search in one bounded
call. It is for agents that would otherwise run several `rg` commands followed
by line-range reads.

## Market Position

Existing tools:

- `ripgrep` is excellent for fast text search but does not correlate multiple
  named patterns into one agent-ready schema.
- `ast-grep` provides structural search and rewrite, but it is not a compact
  multi-pattern context packer.
- Probe combines fast search and AST-aware context, proving demand for this
  workflow, but it is a broader context engine with agent features.

Market validity: high.

Coverage and impact: high. Replaces repeated search/read loops, correlates
multiple named patterns, and reduces token waste from broad snippets by adding
tree-sitter AST context directly to each hit.

Build decision: YES.

## Naming

- Binary: `axt-ctxpack`
- Optional alias: `ctxpack`
- Crate: `crates/axt-ctxpack`
- Schema prefix: `axt.ctxpack.v1`

The `axt-ctxpack` name did not appear as an established public CLI in the
market scan. Verify crates.io and package registries again before publishing.

## MVP Scope

- Search one or more roots with gitignore-aware traversal.
- Accept multiple named patterns:
  `--pattern name=REGEX`.
- Support include globs through `--files <GLOB>` and repeated `--include`.
- Emit per-hit file, line, column, byte range, pattern name, matched text,
  snippet, language, tree-sitter node kind, enclosing symbol, and AST path.
- Classify hits with embedded tree-sitter parsers when the file language is
  supported: `code`, `comment`, `string`, `test`, or `unknown`.
- Emit `classification_source` as `ast`, `heuristic`, or `unknown`; fallback to
  heuristics only for unsupported languages or files that cannot be parsed.
- Enforce `--limit`, `--max-bytes`, and `--strict`.

## Deferred Scope

- Semantic search.
- Embeddings.
- Remote repository search.
- Rewrite or edit application.
- Full AST query language.
- LSP-backed semantic ranking or cross-file symbol graphs.

## CLI Sketch

```bash
axt-ctxpack --pattern todo=TODO --pattern panic='unwrap\(|expect\(' src --json
axt-ctxpack --files 'crates/**/*.rs' --pattern public='pub fn' --context 2 --agent
axt-ctxpack --print-schema json
```

## Output Requirements

- Human: compact grouped summary by pattern and file.
- `--json`: `axt.ctxpack.v1` envelope.
- `--json-data`: data payload only.
- `--jsonl`: summary record, then one hit record per match.
- `--agent`: ACF records with stable keys.

## JSON Data Shape

```json
{
  "root": ".",
  "patterns": [{"name": "todo", "query": "TODO", "kind": "regex"}],
  "summary": {"files_scanned": 10, "hits": 3, "truncated": false},
  "hits": [
    {
      "pattern": "todo",
      "path": "src/lib.rs",
      "line": 12,
      "column": 5,
      "kind": "comment",
      "classification_source": "ast",
      "language": "rust",
      "node_kind": "line_comment",
      "enclosing_symbol": null,
      "ast_path": ["line_comment", "source_file"],
      "snippet": "..."
    }
  ],
  "next": ["axt-ctxpack src/lib.rs --pattern todo=TODO --context 2 --agent"]
}
```

## Cross-Platform Matrix

| Feature | Linux | macOS | Windows |
|---|---:|---:|---:|
| Text regex search | yes | yes | yes |
| Gitignore traversal | yes | yes | yes |
| UTF-8 path output | yes | yes | partial; non-UTF-8 paths warn |
| AST classification | yes | yes | yes |
| Heuristic fallback | yes | yes | yes |

## Tests

- CLI mode snapshots for human, JSON, JSONL, and ACF.
- Regex parser tests for named patterns.
- Fixture tests for repeated hits, overlapping patterns, no hits, hidden files,
  gitignore behavior, binary file skipping, and truncation.
- Tree-sitter classification fixtures for Rust comments, strings, normal code,
  enclosing functions, and tests.
- Fallback classification fixtures for unsupported extensions and parse errors.
- Windows path separator snapshots gated by platform where necessary.

## Skill Requirements

Create `docs/skills/axt-ctxpack/SKILL.md` with rules:

- Use `axt-ctxpack --agent` when searching for several related patterns.
- Prefer named patterns over separate command calls.
- Use `--context 0` first when only locations are needed.
- Use `node_kind`, `enclosing_symbol`, and `ast_path` to decide which exact
  file region to inspect next.

Update `scripts/agent/install-skills.py` only after spec approval.
