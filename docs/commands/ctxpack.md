# axt-ctxpack

`axt-ctxpack` searches local files for multiple named regex patterns and returns
bounded snippets plus hit classification. It is designed to replace repeated
`rg` plus line-range reads in agent workflows.

## Usage

```bash
axt-ctxpack --pattern todo=TODO src --json
axt-ctxpack --pattern todo=TODO --pattern panic='unwrap\(|expect\(' src --agent
axt-ctxpack --files 'crates/**/*.rs' --pattern public='pub fn' --context 2
axt-ctxpack --pattern test='#[test]' --include '**/*.rs' --limit 50 --agent
```

At least one `--pattern <NAME=REGEX>` is required. `ROOT` defaults to `.`.

## Options

| Option | Description |
|---|---|
| `ROOT...` | Files or directories to search. Default `.`. |
| `--pattern <NAME=REGEX>` | Named Rust regex. Repeatable and required. Names may contain ASCII letters, digits, `_`, and `-`. |
| `--files <GLOB>` | Include glob. Repeatable. |
| `--include <GLOB>` | Include glob. Repeatable; equivalent to `--files`. |
| `--context <N>` | Context lines around each match. Default `0`. |
| `--max-depth <N>` | Directory traversal depth. Default `16`. |
| `--hidden` | Include hidden files. |
| `--no-ignore` | Disable ignore, gitignore, global gitignore, and git exclude filters. |
| `--json` | Emit the `axt.ctxpack.v1` JSON envelope. |
| `--agent` | Emit minified summary-first JSONL records. |
| `--print-schema [human|compact|json|agent]` | Print the selected output contract and exit. |
| `--list-errors` | Print the standard error catalog as JSONL and exit. |
| `--limit <N>` | Maximum retained hits and maximum agent records. Default `200`. |
| `--max-bytes <BYTES>` | Maximum agent output bytes. Default `65536`. |
| `--strict` | Exit with `output_truncated_strict` when truncation is required. |

## Examples

Find TODO comments with compact JSON:

```bash
axt-ctxpack --pattern todo=TODO src --json
```

Search for panic-prone Rust calls and include nearby context:

```bash
axt-ctxpack crates --include '**/*.rs' --pattern panic='unwrap\(|expect\(' --context 2 --agent
```

Run two named searches in one bounded scan:

```bash
axt-ctxpack --pattern route='app\\.route' --pattern auth='Authorization' src --agent
```

Search hidden files while bypassing ignore rules:

```bash
axt-ctxpack . --pattern secret='API_KEY' --hidden --no-ignore --limit 25
```

## Scope

The command supports local UTF-8 text files. Directory traversal is
gitignore-aware by default through the shared filesystem walker. Binary files,
non-UTF-8 files, and non-UTF-8 paths are skipped with warnings where possible.

For Rust, TypeScript, JavaScript, Python, Go, Java, and PHP, hits are classified
through embedded tree-sitter grammars as `code`, `comment`, `string`, `test`, or
`unknown`. Unsupported languages and parse errors fall back to line heuristics
and set `classification_source` accordingly.

`axt-ctxpack` is not semantic search, embedding search, an edit tool, or an AST
query language.

## Output

TTY stdout defaults to human mode. Non-TTY stdout defaults to compact text.
`--json` and `--agent` are explicit structured modes.

Human mode groups hits by path/pattern and prints readable snippets:

```text
root=. patterns=1 files=10 hits=3 warnings=0 bytes_scanned=8192 truncated=false
src/lib.rs:12:5 todo comment "TODO"
  12:// TODO: tighten this
```

Compact mode is the default for non-TTY capture:

```text
ctxpack root=. patterns=1 files_scanned=10 files_matched=1 hits=3 warnings=0 bytes_scanned=8192 truncated=false
hit pat=todo src/lib.rs:12:5 kind=comment text="TODO"
```

JSON mode emits `axt.ctxpack.v1`:

```json
{
  "schema": "axt.ctxpack.v1",
  "ok": true,
  "data": {
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
  },
  "warnings": [],
  "errors": []
}
```

Agent mode emits summary-first JSONL. Hit records use short keys and omit
`ast_path` to keep token cost low:

```jsonl
{"schema":"axt.ctxpack.summary.v1","type":"summary","ok":true,"root":".","patterns":2,"files_scanned":10,"files_matched":1,"hits":3,"warnings":0,"bytes_scanned":8192,"truncated":false,"next":["axt-ctxpack src/lib.rs --pattern todo=TODO --context 2 --agent"]}
{"schema":"axt.ctxpack.hit.v1","type":"hit","pat":"todo","p":"src/lib.rs","line":12,"col":5,"range":{"start":240,"end":244},"k":"comment","src":"ast","l":"rust","node":"line_comment","sym":null,"text":"TODO","snippet":"12:// TODO: tighten this"}
```

Agent record schemas:

- `axt.ctxpack.summary.v1`
- `axt.ctxpack.hit.v1`
- `axt.ctxpack.warn.v1`

## Classification Fields

| Field | Meaning |
|---|---|
| `kind` / `k` | `code`, `comment`, `string`, `test`, or `unknown`. |
| `classification_source` / `src` | `ast`, `heuristic`, or `unknown`. |
| `language` / `l` | Detected language when known. |
| `node_kind` / `node` | Tree-sitter node kind when AST classification is available. |
| `enclosing_symbol` / `sym` | Closest function/class/module-like symbol when detected. |
| `ast_path` | JSON-only parser path for debugging classification. |

## Cross-Platform Notes

Regex search, gitignore-aware traversal, and embedded AST classification are
supported on Linux, macOS, and Windows. Windows non-UTF-8 paths can be skipped
with `path_not_utf8` warnings because public output uses UTF-8 paths.

## Performance

Each selected file is read once. The command stops retaining hits at `--limit`.
Use `--context 0` for location-only searches and raise context only when
snippets are needed.

## Error Codes

Standard axt error codes are available through `--list-errors`. Common
`axt-ctxpack` failures map to:

- `usage_error`: missing pattern, invalid `NAME=REGEX`, duplicate pattern name,
  invalid regex, or invalid glob.
- `path_not_found`: an input root does not exist.
- `permission_denied`: a file or directory cannot be read.
- `io_error`: filesystem or output serialization failed.
- `output_truncated_strict`: output was truncated under `--strict`.
