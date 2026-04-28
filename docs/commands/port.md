# axt-port

`axt-port` inspects local TCP/UDP sockets and can free a port held by a local process.

## Usage

```bash
axt-port [FLAGS] list
axt-port [FLAGS] who <PORT> [<PORT>...]
axt-port [FLAGS] free <PORT> [<PORT>...] [--dry-run] [--signal term|kill|int] [--grace 3s]
axt-port [FLAGS] watch <PORT> [--timeout 30s]
```

Shared flags are available before the subcommand: `--json`, `--json-data`, `--jsonl`, `--agent`, `--plain`, `--print-schema`, `--list-errors`, `--limit`, `--max-bytes`, and `--strict`.

Port filters are also shared: `--proto tcp|udp|both`, `--include-loopback true|false`, `--listening-only true|false`, `--host <ADDR>`, `--owner <USER>`, and `--pid <PID>`.

## Examples

```bash
axt-port who 3000
axt-port --json who 3000
axt-port --agent free 3000 --dry-run
axt-port watch 3000 --timeout 5s
```

Human `who` output:

```text
Port 3000 (tcp, listening)
  PID 47281    node    node server.js
  Cwd:         /Users/dario/projects/api
  Bound:       0.0.0.0:3000
  Owner:       dario
```

Agent output starts with an ACF summary:

```text
schema=axt.port.agent.v1 ok=true mode=records action=who port=3000 held=true holders=1 freed=false timed_out=false ms=12 truncated=false
H port=3000 proto=tcp pid=47281 name=node bound=0.0.0.0:3000 cmd="node server.js" cwd=/Users/dario/projects/api owner=dario
```

JSON uses the stable `axt.port.v1` envelope. JSONL emits `axt.port.summary.v1` first, followed by socket, holder, and action records.

`watch` exits successfully as soon as the port is observed free. If the port remains held until `--timeout`, it exits with `timeout` and includes the last observed holder records.

## Safety

`free` is the only mutating subcommand. `list`, `who`, and `watch` are read-only.

`--dry-run` reports the same holders and action records but does not signal a process. `--confirm` prompts only when stdout is attached to a terminal; non-interactive callers are expected to obtain consent before invoking `free`.

`axt-port` refuses to kill PID 1, its own process, and its parent process unless `--force-self` is passed for the parent case. `--tree` also targets child processes of the holder. On Unix, `term`, `kill`, and `int` map to `SIGTERM`, `SIGKILL`, and `SIGINT`; on Windows, `--tree` uses `taskkill /T` and `kill` uses forced `taskkill /F`.

## Cross-platform Notes

Linux reads `/proc/net/*` and maps socket inodes through `/proc/<pid>/fd`. macOS uses local `lsof` output. Windows uses local `netstat -ano`, PowerShell `Win32_Process` lookup for process metadata, and `taskkill` for signaling. Process `cwd` is best effort and may be `null` when the OS denies access.

The scope is local sockets only. Inputs such as `example.com:443` are rejected as usage errors.

## Error Codes

Standard axt error codes are available through `--list-errors`. Common `axt-port` failures map to:

- `usage_error`: invalid local port or missing subcommand.
- `permission_denied`: the OS denied process inspection or signaling.
- `timeout`: `watch` exceeded `--timeout`.
- `feature_unsupported`: socket inspection is unavailable on this platform.
- `command_failed`: a holder could not be freed.
