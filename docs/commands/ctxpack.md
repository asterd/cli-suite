# axt-ctxpack

`axt-ctxpack` searches local files for multiple named regex patterns and returns compact, bounded context snippets plus tree-sitter hit classification in one call.

## Scope

The command supports local UTF-8 text files. Directory traversal is gitignore-aware by default through the shared filesystem walker. Binary files and non-UTF-8 files are skipped with warnings.

For Rust, TypeScript, JavaScript, Python, Go, Java, and PHP, each hit is parsed with an embedded tree-sitter grammar and classified as `code`, `comment`, `string`, `test`, or `unknown`. Unsupported languages and parse errors fall back to documented line heuristics and set `classification_source` accordingly.

`axt-ctxpack` is not semantic search, embedding search, an edit tool, or an AST query language.

## Examples

```bash
axt-ctxpack --pattern todo=TODO --pattern panic='unwrap\(|expect\(' src --json
axt-ctxpack --files 'crates/**/*.rs' --pattern public='pub fn' --context 2 --agent
axt-ctxpack --pattern test='#[test]' --include '**/*.rs' --jsonl --limit 50
```

## Flags

- `ROOT...`: files or directories to search. Default: `.`.
- `--pattern <NAME=REGEX>`: named regex to search for. Repeatable and required.
- `--files <GLOB>`: include glob. Repeatable.
- `--include <GLOB>`: include glob. Repeatable.
- `--context <N>`: context lines around each match. Default: `0`.
- `--max-depth <N>`: maximum directory traversal depth. Default: `16`.
- `--hidden`: include hidden files.
- `--no-ignore`: disable ignore, gitignore, global gitignore, and git exclude filters.
- `--limit <N>`: maximum hit records retained and maximum line-oriented output records.
- `--max-bytes <BYTES>`: maximum payload bytes for line-oriented output.
- `--strict`: return exit 6 when line-oriented output truncation is required.
- `--plain`, `--json`, `--json-data`, `--jsonl`, `--agent`: standard output modes.
- `--print-schema [human|json|jsonl|agent]`: print the selected schema description.
- `--list-errors`: emit the standard error catalog as JSONL.

## JSON

`--json` emits an `axt.ctxpack.v1` envelope. `--json-data` emits only:

```json
{
  "root": ".",
  "patterns": [{"name": "todo", "query": "TODO", "kind": "regex"}],
  "summary": {
    "roots": 1,
    "files_scanned": 10,
    "files_matched": 1,
    "hits": 3,
    "warnings": 0,
    "bytes_scanned": 8192,
    "truncated": false
  },
  "hits": [
    {
      "pattern": "todo",
      "path": "src/lib.rs",
      "line": 12,
      "column": 5,
      "byte_range": {"start": 240, "end": 244},
      "kind": "comment",
      "classification_source": "ast",
      "language": "rust",
      "node_kind": "line_comment",
      "enclosing_symbol": null,
      "ast_path": ["line_comment", "source_file"],
      "matched_text": "TODO",
      "snippet": "12:// TODO: tighten this"
    }
  ],
  "warnings": [],
  "next": ["axt-ctxpack src/lib.rs --pattern todo=TODO --context 2 --agent"]
}
```

## JSONL

The first record is always `axt.ctxpack.summary.v1`. Hit records use `axt.ctxpack.hit.v1`. Warning records use `axt.ctxpack.warn.v1`.

## Agent Mode

Agent mode uses `axt.ctxpack.agent.v1`:

```text
schema=axt.ctxpack.agent.v1 ok=true mode=records patterns=2 files=10 matched=1 hits=3 warnings=0 bytes=8192 truncated=false
H pattern=todo path=src/lib.rs line=12 col=5 start=240 end=244 kind=comment src=ast lang=rust node=line_comment symbol=- text=TODO snippet="12:// TODO: tighten this"
S run="axt-ctxpack src/lib.rs --pattern todo=TODO --context 2 --agent"
```

Command-specific prefix:

- `H`: hit record.

Shared prefixes:

- `W`: warning.
- `S`: suggested next command.

## Error Codes

- `usage_error` (2): a pattern, regex, or glob is invalid.
- `path_not_found` (3): an input root does not exist.
- `permission_denied` (4): a file or directory cannot be read.
- `output_truncated_strict` (6): truncation was required under `--strict`.
- `io_error` (8): filesystem or output error.

## Cross-Platform Notes

| Feature | Linux | macOS | Windows | Notes |
|---|---:|---:|---:|---|
| Text regex search | yes | yes | yes | Uses Rust `regex`. |
| Gitignore traversal | yes | yes | yes | Uses shared ignore-aware walker. |
| UTF-8 path output | yes | yes | partial | Non-UTF-8 paths are skipped with warnings. |
| AST classification | yes | yes | yes | Embedded tree-sitter grammars for Rust, TypeScript, JavaScript, Python, Go, Java, and PHP. |
| Heuristic fallback | yes | yes | yes | Used only for unsupported languages or parse errors. |

## Performance

The command reads each selected file once and stops retaining hits at `--limit`. Use `--context 0` for location-only searches and raise context only when snippets are needed.
