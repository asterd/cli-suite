# axt-peek

`axt-peek` returns a compact directory snapshot: paths, entry kind, size,
language, git status, mtime, and a summary.

## Examples

```bash
axt-peek
axt-peek crates/axt-peek --depth 3
axt-peek fixtures/fs-small --json
axt-peek fixtures/fs-small --agent --summary-only
axt-peek . --changed
axt-peek . --kind file --type code --agent
```

## Output

Default output is human on TTY stdout and agent JSONL on non-TTY stdout.
Explicit modes are `--json` and `--agent`.

`--json` emits the `axt.peek.v1` envelope. `--agent` emits summary-first JSONL
with records:

- `axt.peek.summary.v1`
- `axt.peek.entry.v1`
- `axt.peek.warn.v1`

## Flags

- `PATHS...`: roots to scan. Default `.`.
- `--depth <N>`: maximum traversal depth. Default `2`.
- `--kind all|file|dir`: filter entry kind. Default `all`.
- `--include-hidden`: include dotfiles and hidden paths.
- `--no-ignore`: disable ignore-file handling.
- `--no-git`: skip git discovery and status lookup.
- `--changed`: include only entries with non-clean git status.
- `--changed-since <REF>`: include files changed between `<REF>` and `HEAD`.
- `--type <KIND>`: filter by `text`, `binary`, `image`, `archive`, `code`,
  `config`, or `data`.
- `--lang <LANG>`: include only entries whose language matches exactly.
- `--hash none|blake3`: compute no hash or BLAKE3 hashes. Default `none`.
- `--summary-only`: emit only the summary.
- `--sort name|size|mtime|git|type`: sort output entries. Default `name`.
- `--reverse`: reverse sort order.
- `--max-file-size <SIZE>`: skip larger regular files.
- `--follow-symlinks`: follow symlinks while walking.
- `--cross-fs`: allow traversal across filesystem boundaries.
- `--json`: emit the JSON envelope.
- `--agent`: emit summary-first JSONL.
- `--color auto|always|never`: color policy for human output.
- `--limit <N>`, `--max-bytes <BYTES>`, `--strict`: agent output limits.
- `--print-schema [human|json|agent]`: print schema reference.
- `--list-errors`: print the standard error catalog as JSONL.

## Cross-Platform Notes

Directory walking, metadata extraction, mtime output, and git status are
supported on Linux, macOS, and Windows. Use `--no-git` for very large
repositories when git status is not needed.
