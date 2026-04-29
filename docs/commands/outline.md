# axt-outline

`axt-outline` emits compact source outlines: declarations, signatures, doc comments, symbol kinds, visibility, paths, and source ranges. It never emits function bodies.

## Scope

The command supports Rust, TypeScript, JavaScript, Python, Go, Java, and PHP source files through embedded tree-sitter grammars. It does not require parser tools, language servers, or network access at runtime.

Unsupported file extensions are reported as warnings in mixed input. If no supported source file is found, the command exits with `feature_unsupported` (exit 9).

## Examples

```bash
axt-outline src/lib.rs
axt-outline crates/axt-test/src --agent
axt-outline . --public-only --json
axt-outline src --jsonl --limit 100 --max-bytes 32768
```

## Flags

- `PATH...`: files or directories to outline. Default: `.`.
- `--lang rust|typescript|javascript|python|go|java|php`: select one supported language.
- `--public-only`: emit only public, crate-visible, and restricted symbols.
- `--private`: reserved compatibility flag; private symbols are included by default.
- `--tests`: reserved compatibility flag for future test-source filtering.
- `--max-depth <N>`: maximum directory traversal depth. Default: `16`.
- `--sort path|name|kind|source`: output ordering. Default: `path`.
- `--limit <N>`: maximum records for line-oriented output.
- `--max-bytes <BYTES>`: maximum payload bytes for line-oriented output.
- `--strict`: return exit 6 when truncation is required.
- `--plain`, `--json`, `--json-data`, `--jsonl`, `--agent`: standard output modes.
- `--print-schema [human|json|jsonl|agent]`: print the selected schema description.
- `--list-errors`: emit the standard error catalog as JSONL.

## JSON

`--json` emits an `axt.outline.v1` envelope. `--json-data` emits only:

```json
{
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
}
```

## JSONL

The first record is always `axt.outline.summary.v1`. Symbol records use `axt.outline.symbol.v1`. Warning records use `axt.outline.warn.v1`.

## Agent Mode

Agent mode uses `axt.outline.agent.v1`:

```text
schema=axt.outline.agent.v1 ok=true mode=records files=1 symbols=3 warnings=0 source_bytes=8192 signature_bytes=240 truncated=false
Y path=src/lib.rs lang=rust kind=fn visibility=pub name=parse_config line=42 end_line=57 parent=- signature="pub fn parse_config(input: &str) -> Result<Config, Error>" docs="Parse the configuration text."
S run="axt-slice src/lib.rs --symbol parse_config --agent"
```

Command-specific prefix:

- `Y`: symbol record.

Shared prefixes:

- `W`: warning.
- `S`: suggested next command.

## Error Codes

- `path_not_found` (3): an input path does not exist.
- `permission_denied` (4): a file or directory cannot be read.
- `output_truncated_strict` (6): truncation was required under `--strict`.
- `io_error` (8): filesystem or output error.
- `feature_unsupported` (9): no supported source files were found.
- `runtime_error` (1): parse errors are warnings in mixed input; unrecoverable runtime failures use this code.

## Cross-Platform Notes

| Feature | Linux | macOS | Windows | Notes |
|---|---:|---:|---:|---|
| File input | yes | yes | yes | UTF-8 paths are required. |
| Directory traversal | yes | yes | yes | Symlinks are not followed. |
| Rust outlines | yes | yes | yes | Embedded tree-sitter grammar. |
| TypeScript/JavaScript outlines | yes | yes | yes | Embedded tree-sitter grammars. |
| Python outlines | yes | yes | yes | Embedded tree-sitter grammar. |
| Go outlines | yes | yes | yes | Embedded tree-sitter grammar. |
| Java outlines | yes | yes | yes | Embedded tree-sitter grammar. |
| PHP outlines | yes | yes | yes | Embedded tree-sitter grammar. |
| LSP integration | no | no | no | Deferred scope; no external server dependency. |

## Performance

The command reads local files once and keeps output compact. Directory traversal is deterministic by filename and bounded by `--max-depth`. `source_bytes` and `signature_bytes` in the summary expose a simple compression signal for token-budget decisions.

## Deferred Scope

LSP-backed ranking and cross-file semantic ranking are deferred. The parser layer is tree-sitter based and intentionally extracts declarations only.
