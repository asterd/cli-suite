# axt-drift

`axt-drift` marks filesystem state and later reports what changed. It is useful
after builds, generators, migrations, and test runs.

## Usage

```bash
axt-drift [OPTIONS] mark [--name <NAME>] [--hash]
axt-drift [OPTIONS] diff [--since <NAME>] [--hash]
axt-drift [OPTIONS] run [--name <NAME>] [--hash] -- <CMD> [ARGS]...
axt-drift [OPTIONS] list
axt-drift [OPTIONS] reset
```

When no name is provided, `axt-drift` uses `default`. Snapshots are stored as
JSONL under `.axt/drift/<NAME>.jsonl`.

## Options

| Option | Description |
|---|---|
| `mark` | Capture the current filesystem state. |
| `diff` | Compare current state to a named mark. |
| `run -- <CMD>` | Mark, run a local command, then report changed files. |
| `list` | List stored marks. |
| `reset` | Remove stored marks. |
| `--name <NAME>` | Mark name for `mark` and `run`. Default `default`. |
| `--since <NAME>` | Mark name for `diff`. Default `default`. |
| `--hash` | Include BLAKE3 hashes in snapshots. Slower but detects content changes beyond metadata. |
| `--json` | Emit the `axt.drift.v1` JSON envelope. |
| `--agent` | Emit minified summary-first JSONL records. |
| `--print-schema [human|json|agent]` | Print the selected output contract and exit. |
| `--list-errors` | Print the standard error catalog as JSONL and exit. |
| `--limit <N>` | Maximum agent records. Default `200`. |
| `--max-bytes <BYTES>` | Maximum agent output bytes. Default `65536`. |
| `--strict` | Exit with `output_truncated_strict` when truncation is required. |

## Output

Human mode prints compact counts:

```text
created=2 modified=1 deleted=0 since=default
```

JSON mode emits `axt.drift.v1`. Agent mode emits summary-first JSONL:

```jsonl
{"schema":"axt.drift.summary.v1","type":"summary","operation":"diff","name":"default","files":12,"changed":1,"marks":0,"removed":0,"truncated":false,"next":["axt-peek . --changed --agent"]}
{"schema":"axt.drift.file.v1","type":"file","path":"dist/app.js","action":"created","size_delta":1204}
```

Agent record schemas:

- `axt.drift.summary.v1`
- `axt.drift.file.v1`
- `axt.drift.mark.v1`
- `axt.drift.warn.v1`

## Storage

Snapshots contain relative path, size, mtime, and optional hash records.
`.axt/drift` is excluded from captured snapshots so marks do not report
themselves as changes.

## Cross-Platform Notes

Metadata and hash snapshots are supported on Linux, macOS, and Windows. Hash
mode is portable but slower. Filesystem timestamp resolution can differ by
platform; use `--hash` when exact content drift matters.

## Error Codes

Standard axt error codes are available through `--list-errors`. Common
`axt-drift` failures map to:

- `path_not_found`: a named mark is missing.
- `permission_denied`: a file or directory cannot be read.
- `command_failed`: `run` wrapped command exited non-zero.
- `io_error`: snapshot, command IO, or output serialization failed.
- `output_truncated_strict`: output was truncated under `--strict`.
