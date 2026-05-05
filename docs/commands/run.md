# axt-run

`axt-run` runs a command and emits a structured execution envelope with exit
status, duration, stdout/stderr summaries, saved stream logs, and file changes
inside the working directory.

## Usage

```bash
axt-run [OPTIONS] -- <COMMAND> [ARGS]...
axt-run --rerun-last
axt-run show [<NAME>|last] [--stdout|--stderr]
axt-run list
axt-run clean [--older-than <DURATION>]
```

## Options

| Option | Description |
|---|---|
| `-- <COMMAND> [ARGS]...` | Command to run. The separator is required when command arguments could be parsed as `axt-run` flags. |
| `--rerun-last` | Re-run the most recent saved command. |
| `show [<NAME>\|last]` | Show a saved run. Defaults to `last`. |
| `show --stdout` | Print the saved stdout stream for a run. |
| `show --stderr` | Print the saved stderr stream for a run. |
| `list` | List saved runs under `.axt/runs`. |
| `clean [--older-than <DURATION>]` | Remove saved runs older than a duration. Defaults to `.axt/config.toml` `retention_days`, then 30 days. |
| `--save <NAME>` | Name the saved run. Default is a timestamp slug. |
| `--no-save` | Disable `.axt/runs` artifacts. |
| `--cwd <DIR>` | Run the command from a different working directory. |
| `--env KEY=VALUE` | Add or override one child environment variable. Repeatable. |
| `--env-file <FILE>` | Add child environment values from a local env file. |
| `--timeout <DURATION>` | Terminate the command on timeout. |
| `--capture always\|never\|auto` | Control stream capture. Default `auto`. |
| `--max-log-bytes <SIZE>` | Cap each persisted stream log. Default `5MiB`. |
| `--watch-files` | Explicitly enable cwd file snapshots. File watching is enabled unless `--no-watch-files` is passed. |
| `--no-watch-files` | Disable cwd file snapshots and changed-file output. |
| `--include <GLOB>` | Include glob for file watching. Repeatable. |
| `--exclude <GLOB>` | Exclude glob for file watching. Repeatable. |
| `--shell` | Run through the platform shell. Off by default. |
| `--summary-only` | Keep the primary summary-oriented output. |
| `--tail-bytes <N>` | Control captured stderr/stdout tail buffers. |
| `--hash` | Add BLAKE3 hashes to changed-file detection. |
| `--json` | Emit the `axt.run.v1` JSON envelope. |
| `--agent` | Emit minified summary-first JSONL records. |
| `--print-schema [human\|json\|agent]` | Print the selected output contract and exit. |
| `--list-errors` | Print the standard error catalog as JSONL and exit. |
| `--limit <N>` | Maximum agent records. Default `200`. |
| `--max-bytes <BYTES>` | Maximum agent output bytes. Default `65536`. |
| `--strict` | Exit with `output_truncated_strict` when truncation is required. |

`--capture always` pipes stdout/stderr through `axt-run` for summaries and saved
logs. `--capture never` inherits the parent's stdio directly, so no stream tail
or persisted log is available. `--capture auto` inherits when `axt-run` stdout is
a TTY and captures otherwise.

## Examples

Run a command and emit agent JSONL:

```bash
axt-run --agent -- cargo test
```

Run from a subdirectory with a timeout and no saved artifacts:

```bash
axt-run --cwd crates/axt-test --timeout 2m --no-save -- cargo test
```

Capture a named failed run with stream tails:

```bash
axt-run --save failing-test --capture always --tail-bytes 4096 -- npm test
```

Inspect saved stderr from the last run:

```bash
axt-run show last --stderr
```

Clean saved runs older than seven days:

```bash
axt-run clean --older-than 7d
```

## Output

`--json` emits an `axt.run.v1` envelope and validates against
`schemas/axt.run.v1.schema.json`.

`--agent` emits summary-first JSONL records. The first line includes the schema,
command, exit status, duration, stream line counts, changed-file count, saved run
name, truncation state, and dynamic next hints. Changed-file detail records use
short keys: `p` for path, `a` for action, and `b` for bytes.

Command-specific agent JSONL keys:

- `cmd`: command string.
- `exit`: command exit code, or `timeout`.
- `stdout_lines` / `stderr_lines`: captured line counts.
- `changed`: number of changed files detected.
- `saved`: saved run name, or `none`.
- `name`: saved run name.
- `stream`: `stdout` or `stderr`.
- `runs`: saved-run count.
- `removed`: cleaned saved-run count.

Agent record schemas:

- `axt.run.summary.v1`
- `axt.run.file.v1`
- `axt.run.stream.v1`
- `axt.run.list.v1`
- `axt.run.clean.v1`
- `axt.run.warn.v1`

## Storage

Saved runs are written under:

```text
.axt/runs/<name>/
├── meta.json
├── stdout.log
├── stderr.log
├── changed.json
└── summary.agent.jsonl
```

`.axt/` is never added to `.gitignore` automatically. When a saved run is
created and `.axt/` is not ignored, `axt-run` prints a suggestion on stderr.

`axt-run clean` removes saved runs older than 30 days by default. A project can
set `.axt/config.toml` with `retention_days = 7` to change that default.

## Cross-Platform Notes

| Feature | Linux | macOS | Windows | Notes |
|---|---:|---:|---:|---|
| Spawn, capture, exit code | yes | yes | yes | Uses `tokio::process::Command`. |
| Timeout | yes | yes | yes | Unix terminates the process group, then kills it. Windows assigns the child to a Job Object and terminates the job. |
| Shell mode | yes | yes | yes | Unix uses `$SHELL -lc`; Windows uses `cmd /C`. |
| File change snapshot | yes | yes | yes | Uses size + mtime + inode where available; `--hash` enables BLAKE3. |
| Saved runs/list/show/clean | yes | yes | yes | Stored in `.axt/runs`. |

## Error Codes

Standard axt error codes are available through `--list-errors`. Common
`axt-run` failures map to:

- `command_failed` exits 11 when the child command exits non-zero.
- `timeout` exits 5 when the child command exceeds `--timeout`.
- `usage_error` exits 2 for invalid CLI/env/glob input.
- `path_not_found` exits 3 for missing `--cwd` or saved run names.
- `permission_denied` exits 4 for permission failures.
- `io_error` exits 8 for filesystem or serialization failures.
- `runtime_error` exits 1 for process execution or rendering failures.
