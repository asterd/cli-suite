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

Run options:

- `--save <NAME>` names the saved run; the default is a timestamp slug.
- `--no-save` disables `.axt/runs` artifacts.
- `--cwd <DIR>` runs the command from a different working directory.
- `--env KEY=VALUE` and `--env-file <FILE>` add child environment values.
- `--timeout <DURATION>` terminates the command on timeout.
- `--capture always|never|auto` controls stream capture. `always` always
  pipes stdout/stderr through `axt-run` for summaries and saved logs;
  `never` inherits the parent's stdio so the child can use the terminal
  directly (no tail or persisted log); `auto` (default) inherits when
  `axt-run`'s own stdout is a TTY and captures otherwise.
- `--max-log-bytes <SIZE>` caps each persisted stream log.
- `--watch-files` / `--no-watch-files` controls cwd file snapshots.
- `--include <GLOB>` and `--exclude <GLOB>` filter file watching.
- `--hash` adds BLAKE3 hashes to changed-file detection.
- `--shell` runs through the platform shell and is opt-in.
- `--summary-only` keeps the primary summary-oriented output.
- `--tail-bytes <N>` controls captured stderr/stdout tail buffers.

Shared output flags are supported before the subcommand or run command:
`--json`, `--agent`, `--print-schema`,
`--list-errors`, `--limit`, `--max-bytes`, and `--strict`.

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

Error codes:

- `command_failed` exits 11 when the child command exits non-zero.
- `timeout` exits 5 when the child command exceeds `--timeout`.
- `usage_error` exits 2 for invalid CLI/env/glob input.
- `path_not_found` exits 3 for missing `--cwd` or saved run names.
- `permission_denied` exits 4 for permission failures.
- `io_error` exits 8 for filesystem or serialization failures.
- `runtime_error` exits 1 for process execution or rendering failures.

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

## Cross-platform matrix

| Feature | Linux | macOS | Windows | Notes |
|---|---:|---:|---:|---|
| Spawn, capture, exit code | yes | yes | yes | Uses `tokio::process::Command`. |
| Timeout | yes | yes | yes | Unix terminates the process group, then kills it. Windows assigns the child to a Job Object and terminates the job. |
| Shell mode | yes | yes | yes | Unix uses `$SHELL -lc`; Windows uses `cmd /C`. |
| File change snapshot | yes | yes | yes | Uses size + mtime + inode where available; `--hash` enables BLAKE3. |
| Saved runs/list/show/clean | yes | yes | yes | Stored in `.axt/runs`. |
