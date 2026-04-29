# axt-outline

`axt-outline` emits compact source outlines: declarations, signatures, doc
comments, symbol kinds, visibility, paths, and source ranges. It never emits
function bodies.

Supported languages: Rust, TypeScript, JavaScript, Python, Go, Java, and PHP.

## Examples

```bash
axt-outline src/lib.rs
axt-outline crates/axt-test/src --agent
axt-outline . --public-only --json
axt-outline src --symbols-only --agent
axt-outline src --agent --limit 100 --max-bytes 32768
```

## Output

Default output is human on TTY stdout and agent JSONL on non-TTY stdout.
Explicit modes are `--json` and `--agent`.

`--json` emits the `axt.outline.v1` envelope. `--agent` emits summary-first JSONL
with records:

- `axt.outline.summary.v1`
- `axt.outline.symbol.v1`
- `axt.outline.warn.v1`

## Flags

- `PATH...`: files or directories to outline. Default `.`.
- `--lang rust|typescript|javascript|python|go|java|php`: select one language.
- `--public-only`: emit only public, crate-visible, and restricted symbols.
- `--symbols-only`: in agent mode, emit only `name`, `kind`, and `line` for each symbol.
- `--max-depth <N>`: maximum directory traversal depth. Default `16`.
- `--sort path|name|kind|source`: output ordering. Default `path`.
- `--json`: emit the JSON envelope.
- `--agent`: emit summary-first JSONL.
- `--limit <N>`, `--max-bytes <BYTES>`, `--strict`: agent output limits.
- `--print-schema [human|json|agent]`: print schema reference.
- `--list-errors`: print the standard error catalog as JSONL.

Unsupported extensions are warnings in mixed input. If no supported source file
is found, the command exits with `feature_unsupported` (9).
