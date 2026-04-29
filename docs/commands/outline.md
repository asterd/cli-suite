# axt-outline

`axt-outline` emits compact source outlines without function bodies. It returns
declarations, signatures, doc comments, symbol kinds, visibility, file paths,
source ranges, and parent symbols for supported local source files.

## Usage

```bash
axt-outline [OPTIONS] [PATH]...
axt-outline src/lib.rs
axt-outline crates/axt-test/src --agent
axt-outline . --public-only --json
axt-outline src --symbols-only --agent --limit 100 --max-bytes 32768
```

`PATH` defaults to `.` and may be repeated. Directories are traversed
recursively up to `--max-depth`.

## Options

| Option | Description |
|---|---|
| `PATH...` | Files or directories to outline. Default `.`. |
| `--lang go|java|javascript|php|python|rust|typescript` | Restrict input to one language. Other supported files become warnings. |
| `--public-only` | Emit only public/exported symbols where visibility can be determined. |
| `--symbols-only` | In agent mode, emit compact symbol records with name, kind, and line only. |
| `--max-depth <N>` | Directory traversal depth. Default `16`. |
| `--sort path|name|kind|source` | Output ordering. Default `path`; `source` preserves collection/source order. |
| `--json` | Emit the `axt.outline.v1` JSON envelope. |
| `--agent` | Emit minified summary-first JSONL records. |
| `--print-schema [human|json|agent]` | Print the selected output contract and exit. |
| `--list-errors` | Print the standard error catalog as JSONL and exit. |
| `--limit <N>` | Maximum agent records. Default `200`. |
| `--max-bytes <BYTES>` | Maximum agent output bytes. Default `65536`. |
| `--strict` | Exit with `output_truncated_strict` when truncation is required. |

## Language Support

Embedded tree-sitter grammars are used for:

- Rust: `*.rs`
- TypeScript: `*.ts`, `*.tsx`, `*.mts`, `*.cts`
- JavaScript: `*.js`, `*.jsx`, `*.mjs`, `*.cjs`
- Python: `*.py`
- Go: `*.go`
- Java: `*.java`
- PHP: `*.php`

Unsupported extensions in mixed input produce `unsupported_language` warnings.
If no supported source file is found, the command exits with
`feature_unsupported`.

## Output

Human mode prints a summary and one compact line per symbol:

```text
files=1 symbols=3 warnings=0 source_bytes=8192 signature_bytes=240
src/lib.rs:42 pub fn parse_config(input: &str) -> Result<Config, Error>
```

JSON mode emits `axt.outline.v1`:

```json
{
  "schema": "axt.outline.v1",
  "ok": true,
  "data": {
    "root": ".",
    "summary": {
      "files": 1,
      "symbols": 3,
      "warnings": 0,
      "source_bytes": 8192,
      "signature_bytes": 240,
      "truncated": false
    },
    "symbols": [
      {
        "path": "src/lib.rs",
        "language": "rust",
        "kind": "fn",
        "visibility": "pub",
        "name": "parse_config",
        "signature": "pub fn parse_config(input: &str) -> Result<Config, Error>",
        "docs": "Parse the configuration text.",
        "range": {"start_line": 42, "end_line": 57},
        "parent": null
      }
    ],
    "warnings": [],
    "next": ["axt-slice src/lib.rs --symbol parse_config --agent"]
  },
  "warnings": [],
  "errors": []
}
```

Agent mode emits summary-first JSONL:

```jsonl
{"schema":"axt.outline.summary.v1","type":"summary","ok":true,"root":".","files":1,"symbols":3,"warnings":0,"source_bytes":8192,"signature_bytes":240,"truncated":false,"next":["axt-slice src/lib.rs --symbol parse_config --agent"]}
{"schema":"axt.outline.symbol.v1","type":"symbol","p":"src/lib.rs","l":"rust","k":"fn","vis":"pub","n":"parse_config","sig":"pub fn parse_config(input: &str) -> Result<Config, Error>","docs":"Parse the configuration text.","range":{"start_line":42,"end_line":57},"parent":null}
```

Agent record schemas:

- `axt.outline.summary.v1`
- `axt.outline.symbol.v1`
- `axt.outline.warn.v1`

## Notes

`axt-outline` is declaration-oriented. It does not emit function bodies, resolve
cross-file references, start LSP servers, or compute semantic importance. Use
`axt-ctxpack` to search code context and future `axt-slice` work to extract a
selected symbol body.

## Cross-Platform Notes

Directory traversal and embedded parsing are supported on Linux, macOS, and
Windows. Symlinks are not followed. Non-UTF-8 paths are treated as IO warnings
or errors depending on where they are encountered.

## Error Codes

Standard axt error codes are available through `--list-errors`. Common
`axt-outline` failures map to:

- `path_not_found`: an input path does not exist.
- `feature_unsupported`: no supported source files were found.
- `permission_denied`: a file or directory cannot be read.
- `io_error`: walking, reading, or output serialization failed.
- `output_truncated_strict`: output was truncated under `--strict`.
