# axt-bundle

`axt-bundle` emits a session warmup bundle for coding agents: shallow file
inventory, local manifests, git state, and dynamic `next` hints in one call.

```bash
axt-bundle .
axt-bundle . --agent
axt-bundle . --json
```

## Output

Default output is human when stdout is a TTY and agent JSONL when stdout is not a
TTY. Explicit modes are `--agent` and `--json`.

Agent records:

- `axt.bundle.summary.v1`
- `axt.bundle.manifest.v1`
- `axt.bundle.git.v1`
- `axt.bundle.file.v1`
- `axt.bundle.warn.v1`

The summary record is always first and includes `next` hints such as `axt-peek`,
`axt-outline`, and `axt-test`.

## Flags

- `ROOT`: root path, default `.`.
- `--depth <N>`: file inventory depth, default `2`.
- `--max-files <N>`: maximum file records, default `40`.
- `--include-hidden`: include hidden paths.
- `--no-ignore`: ignore `.gitignore` and related ignore files.
- `--json`: canonical JSON envelope.
- `--agent`: summary-first JSONL.
- `--print-schema [json|agent|human]`: print schema reference.
- `--list-errors`: print standard error catalog as JSONL.
- `--limit`, `--max-bytes`, `--strict`: agent output limits.

## Cross-Platform

Filesystem and manifest collection are supported on Linux, macOS, and Windows.
Git state is included only when a readable repository is available.
