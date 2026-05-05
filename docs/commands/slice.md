# axt-slice

`axt-slice` extracts source by symbol or by the symbol enclosing a line. It is
intended to replace fragile manual line-range reads after a file has been edited.

## Usage

```bash
axt-slice [OPTIONS] <FILE> --symbol <NAME>
axt-slice [OPTIONS] <FILE> --line <N>
axt-slice src/lib.rs --symbol process_request
axt-slice src/lib.rs --symbol Handler::process_request --include-imports=matched
axt-slice src/lib.rs --line 150 --agent
```

## Options

| Option | Description |
|---|---|
| `FILE` | One local source file to slice. |
| `--symbol <NAME>` | Extract an exact symbol name, qualified name, or `kind::name`. |
| `--line <N>` | Extract the smallest supported symbol enclosing line `N`. |
| `--include-imports[=all\|matched]` | Prepend imports/package/use declarations before the selected symbol. `all` is the default when the flag is present without a value; `matched` uses local syntactic identifier matching. |
| `--include-tests` | Include syntactically detected test declarations from the same file. |
| `--before-symbol` | Include the immediately preceding symbol block. |
| `--after-symbol` | Include the immediately following symbol block. |
| `--json` | Emit the `axt.slice.v1` JSON envelope. |
| `--agent` | Emit minified summary-first JSONL records. |
| `--print-schema [human\|json\|agent]` | Print the selected output contract and exit. |
| `--list-errors` | Print the standard error catalog as JSONL and exit. |
| `--limit <N>` | Maximum agent records. Default `200`. |
| `--max-bytes <BYTES>` | Maximum agent output bytes. Default `65536`. |
| `--strict` | Exit with `output_truncated_strict` when truncation is required. |

Exactly one selector is required: `--symbol` or `--line`.

## Examples

Extract a function by name:

```bash
axt-slice src/lib.rs --symbol process_request
```

Extract the smallest symbol enclosing an edited line:

```bash
axt-slice src/lib.rs --line 150 --agent
```

Include syntactically matched imports with the selected symbol:

```bash
axt-slice src/lib.rs --symbol Handler::process_request --include-imports=matched
```

Disambiguate overloaded or repeated names with a kind-qualified selector:

```bash
axt-slice src/parser.rs --symbol fn::parse --json
```

## Language Support

Embedded tree-sitter grammars are used for:

- Rust: `*.rs`
- TypeScript: `*.ts`, `*.tsx`, `*.mts`, `*.cts`
- JavaScript: `*.js`, `*.jsx`, `*.mjs`, `*.cjs`
- Python: `*.py`
- Go: `*.go`
- Java: `*.java`
- PHP: `*.php`

Unsupported extensions, binary files, and non-UTF-8 files exit with
`feature_unsupported`.

## Output

Human mode prints a summary, the selected range, then the exact source text:

```text
path=src/lib.rs language=Rust status=Selected matches=1 candidates=0 source_bytes=82 truncated=false
src/lib.rs:10-13 fn pub process_request
/// Process one request.
#[inline]
pub fn process_request(req: Request) -> Response {
    Response::ok(req.id)
}
```

JSON mode emits `axt.slice.v1`:

```json
{
  "schema": "axt.slice.v1",
  "ok": true,
  "data": {
    "path": "src/lib.rs",
    "language": "rust",
    "selection": {"kind": "symbol", "query": "process_request"},
    "status": "selected",
    "summary": {"matches": 1, "candidates": 0, "source_bytes": 82, "truncated": false},
    "symbol": {"name": "process_request", "qualified_name": "process_request", "kind": "fn", "visibility": "pub", "range": {"start_line": 12, "end_line": 14}, "parent": null},
    "range": {"start_line": 10, "end_line": 14},
    "spans": [{"start_line": 10, "end_line": 14}],
    "source": "/// Process one request.\n#[inline]\npub fn process_request(req: Request) -> Response {\n    Response::ok(req.id)\n}\n",
    "candidates": [],
    "warnings": [],
    "next": []
  },
  "warnings": [],
  "errors": []
}
```

Agent mode emits summary-first JSONL:

```jsonl
{"schema":"axt.slice.summary.v1","type":"summary","ok":true,"p":"src/lib.rs","l":"rust","status":"selected","matches":1,"candidates":0,"source_bytes":82,"truncated":false,"next":[]}
{"schema":"axt.slice.source.v1","type":"source","p":"src/lib.rs","l":"rust","k":"fn","n":"process_request","qn":"process_request","range":{"start_line":10,"end_line":14},"spans":[{"start_line":10,"end_line":14}],"symbol_range":{"start_line":12,"end_line":14},"source":"/// Process one request.\n#[inline]\npub fn process_request(req: Request) -> Response {\n    Response::ok(req.id)\n}\n"}
```

Agent record schemas:

- `axt.slice.summary.v1`
- `axt.slice.source.v1`
- `axt.slice.candidate.v1`
- `axt.slice.warn.v1`

`range` is the primary selected symbol range, including contiguous docs and
attributes. `spans` lists every emitted source span, which matters when imports,
adjacent symbols, or tests make the returned source non-contiguous.

## Ambiguity

If `--symbol` matches multiple symbols, `axt-slice` does not guess. It returns
`status: "ambiguous"`, emits candidate records, and omits `source`.

Use a qualified query such as `Parser::parse`, a kind-qualified query such as
`fn::process_request`, or a kind-and-parent-qualified query such as
`method::Parser::parse` to disambiguate.

## Cross-Platform Notes

| Feature | Linux | macOS | Windows |
|---|---:|---:|---:|
| UTF-8 source extraction | yes | yes | yes |
| CRLF preservation inside extracted blocks | yes | yes | yes |
| Symbol extraction | parser-dependent | parser-dependent | parser-dependent |
| Import inclusion | language-dependent | language-dependent | language-dependent |

No external parser binaries, LSP servers, network calls, telemetry, or analytics
are used.

## Error Codes

Standard axt error codes are available through `--list-errors`. Common
`axt-slice` failures map to:

- `path_not_found`: the input file does not exist.
- `feature_unsupported`: unsupported extension, binary file, or non-UTF-8 file.
- `usage_error`: invalid selector such as `--line 0`.
- `permission_denied`: the file cannot be read.
- `io_error`: parsing, reading, or output serialization failed.
- `output_truncated_strict`: output was truncated under `--strict`.
