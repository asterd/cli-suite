# axt-port

`axt-port` inspects local TCP/UDP sockets and can free ports held by local
processes. It is read-only except for the explicit `free` subcommand.

## Usage

```bash
axt-port [OPTIONS] list
axt-port [OPTIONS] who <PORT> [<PORT>...]
axt-port [OPTIONS] free <PORT> [<PORT>...] [FREE_OPTIONS]
axt-port [OPTIONS] watch <PORT> [--timeout 30s]
```

Shared flags are available before the subcommand: `--json`, `--agent`,
`--print-schema`, `--list-errors`, `--limit`, `--max-bytes`, and `--strict`.

## Shared Filters

| Option | Description |
|---|---|
| `--proto tcp|udp|both` | Protocol filter. Default `tcp`. |
| `--include-loopback true|false` | Include loopback-bound sockets. Default `true`. |
| `--listening-only true|false` | For TCP, keep only listening sockets. Default `true`. |
| `--host <ADDR>` | Match a local bind address such as `127.0.0.1`, `0.0.0.0`, or `::1`. |
| `--owner <USER>` | Keep sockets whose process owner matches. Best-effort on platforms that expose owner metadata. |
| `--pid <PID>` | Inverse lookup: keep sockets owned by a specific PID. |

## Subcommands

| Subcommand | Description |
|---|---|
| `list` | List matching local sockets. |
| `who <PORT>...` | Show socket and holder metadata for one or more local ports. |
| `free <PORT>...` | Signal holders of one or more local ports. |
| `watch <PORT>` | Poll until a port is observed free or `--timeout` expires. |

`free` options:

| Option | Description |
|---|---|
| `--signal term|kill|int` | Signal to send first. Default `term`. |
| `--grace <DURATION>` | Wait between `term`/`int` and kill escalation. Default `3s`. Supports `ms`, `s`, and `m`. |
| `--dry-run` | Report what would be signaled without mutating processes. |
| `--confirm` | Ask for confirmation when running interactively. |
| `--tree` | Also signal recursive child processes of each holder. |
| `--force-self` | Allow signaling the parent process. The current process and PID 1 are still refused. |

`watch` options:

| Option | Description |
|---|---|
| `--timeout <DURATION>` | Maximum polling duration. Default `30s`. |

## Examples

```bash
axt-port who 3000
axt-port --json who 3000
axt-port --proto both list
axt-port --agent free 3000 --dry-run
axt-port free 3000 --signal term --grace 5s --tree
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

## Output

JSON mode emits `axt.port.v1`:

```json
{
  "schema": "axt.port.v1",
  "ok": true,
  "data": {
    "action": "who",
    "ports": [3000],
    "held": true,
    "freed": false,
    "timed_out": false,
    "sockets": [],
    "holders": [],
    "attempts": [],
    "duration_ms": 12,
    "truncated": false
  },
  "warnings": [],
  "errors": []
}
```

Agent mode emits summary-first JSONL:

```jsonl
{"schema":"axt.port.summary.v1","type":"summary","action":"who","ports":[3000],"sockets":1,"holders":1,"held":true,"freed":false,"timed_out":false,"duration_ms":12,"truncated":false,"next":[]}
{"schema":"axt.port.holder.v1","type":"holder","port":3000,"proto":"tcp","pid":47281,"name":"node","bound":["0.0.0.0:3000"],"cmd":"node server.js","cwd":"/Users/dario/projects/api","owner":"dario","mem":190840832,"started":"2026-04-27T08:14:22Z"}
```

Agent record schemas:

- `axt.port.summary.v1`
- `axt.port.socket.v1`
- `axt.port.holder.v1`
- `axt.port.action.v1`
- `axt.port.warn.v1`

`watch` exits successfully as soon as the port is observed free. If the port
remains held until `--timeout`, it exits with `timeout` and includes the last
observed holder records.

## Safety

`free` is the only mutating subcommand. `list`, `who`, and `watch` are read-only.

`axt-port` refuses to kill PID 1, its own process, and its parent process unless
`--force-self` is passed for the parent case. `--dry-run` is the recommended
first call in agent workflows. `--confirm` prompts only when stdout is attached
to a terminal; non-interactive callers must obtain consent before invoking
`free`.

On Unix, `term`, `kill`, and `int` map to `SIGTERM`, `SIGKILL`, and `SIGINT`. On
Windows, all signals are implemented through `TerminateProcess` with different
exit codes because existing processes cannot be retroactively attached to a new
Job Object. `--tree` recursively discovers child PIDs through process metadata
and signals each descendant.

## Cross-Platform Notes

Socket discovery uses `netstat2` over platform APIs and process enrichment uses
`sysinfo`. Process name, command, parent PID, memory, owner, cwd, and start time
are best-effort fields; inaccessible values are `null` in JSON and omitted or
`null` in agent records. OS permissions are never bypassed.

The scope is local sockets only. Inputs such as `example.com:443` are rejected
as usage errors.

## Error Codes

Standard axt error codes are available through `--list-errors`. Common
`axt-port` failures map to:

- `usage_error`: invalid local port, remote `host:port` syntax, missing
  subcommand, or refused self/parent/PID 1 operation.
- `permission_denied`: the OS denied process inspection or signaling.
- `timeout`: `watch` exceeded `--timeout`.
- `feature_unsupported`: socket inspection is unavailable on this platform.
- `command_failed`: one or more holders could not be freed.
- `output_truncated_strict`: output was truncated under `--strict`.
