# axt-logdx

`axt-logdx` diagnoses local log files and command output. It reads bounded
offline input, deduplicates repeated failure messages, captures common stack
traces, builds a severity timeline, and returns representative snippets.

## Usage

```bash
axt-logdx app.log --severity error --top 20 --json
cat build.log | axt-logdx --stdin --agent
axt-logdx service.log --since 2026-04-28T10:00:00Z --until 2026-04-28T11:00:00Z
```

At least one `PATH` or `--stdin` is required.

## Options

| Option | Description |
|---|---|
| `PATH...` | Local log files to read. |
| `--stdin` | Read log data from stdin. |
| `--severity <LEVEL>` | Minimum severity: `trace`, `debug`, `info`, `warn`, `error`, `fatal`. Default `warn`. |
| `--since <RFC3339>` | Include records at or after a parseable timestamp. |
| `--until <RFC3339>` | Include records at or before a parseable timestamp. |
| `--top <N>` | Maximum retained failure groups before shared limits. Default `20`. |
| `--json` | Emit the `axt.logdx.v1` JSON envelope. |
| `--agent` | Emit minified summary-first JSONL records. |
| `--print-schema [human\|json\|agent]` | Print the selected output contract and exit. |
| `--list-errors` | Print the standard error catalog as JSONL and exit. |
| `--limit <N>` | Maximum agent records and retained groups. Default `200`. |
| `--max-bytes <BYTES>` | Maximum agent output bytes. Default `65536`. |
| `--strict` | Exit with `output_truncated_strict` when truncation is required. |

## Scope

The command supports local text logs and explicit stdin. It detects plain text
logs, JSONL logs, logfmt/key-value logs, Docker JSON logs, Kubernetes container
prefixes, syslog-like timestamps, common Nginx/Apache timestamps, ANSI-colored
logs, CRLF logs, RFC3339 and Unix epoch seconds/milliseconds, and common
JavaScript, Python, Rust, Go, and JVM stack traces through deterministic
heuristics. Invalid UTF-8 input is decoded lossily and reported with an
`invalid_utf8` warning.

Aggregation is streaming and memory-bounded. For high-cardinality logs,
`axt-logdx` retains the likely top groups within a fixed in-memory budget and
emits an `input_truncated` warning when low-frequency groups are evicted before
final ranking.

Time filters use RFC3339 arguments. Lines with unparseable timestamps remain
eligible unless a time filter is active. When a time filter excludes records
that matched severity but had no parseable timestamp, `axt-logdx` emits a
`time_unparseable` warning.

`axt-logdx` is not a live tailer, remote ingestion tool, dashboard, OpenTelemetry
trace graph, or query language.

## Output

JSON mode emits `axt.logdx.v1`:

```json
{
  "schema": "axt.logdx.v1",
  "ok": true,
  "data": {
    "sources": [{"path": "app.log", "lines": 10, "bytes": 900}],
    "summary": {"lines": 10, "groups": 1, "errors": 1, "warnings": 0, "bytes_scanned": 900, "truncated": false},
    "groups": [{"fingerprint": "blake3:...", "severity": "error", "count": 2, "first": {"source": "app.log", "line": 3, "timestamp": "2026-04-28T10:00:00Z"}, "last": {"source": "app.log", "line": 8, "timestamp": "2026-04-28T10:01:00Z"}, "message": "connection refused", "stack": [], "snippets": ["..."]}],
    "timeline": [{"bucket": "2026-04-28T10:00:00Z", "trace": 0, "debug": 0, "info": 0, "warn": 0, "error": 1, "fatal": 0}],
    "warnings": [],
    "next": ["axt-logdx app.log --severity error --top 20 --agent"]
  },
  "warnings": [],
  "errors": []
}
```

Agent mode emits summary-first JSONL:

```jsonl
{"schema":"axt.logdx.summary.v1","type":"summary","ok":true,"sources":1,"lines":10,"groups":1,"errors":1,"warnings":0,"bytes_scanned":900,"truncated":false,"next":["axt-logdx app.log --severity error --top 20 --agent"]}
{"schema":"axt.logdx.group.v1","type":"group","fp":"blake3:...","sev":"error","count":2,"first":{"p":"app.log","line":3,"ts":"2026-04-28T10:00:00Z"},"last":{"p":"app.log","line":8,"ts":"2026-04-28T10:01:00Z"},"msg":"connection refused","stack":[],"snip":["..."]}
```

Agent record schemas:

- `axt.logdx.summary.v1`
- `axt.logdx.group.v1`
- `axt.logdx.timeline.v1`
- `axt.logdx.warn.v1`

## Cross-Platform Notes

File and stdin input, CRLF handling, ANSI stripping, JSONL parsing, timestamp
heuristics, and stack-trace heuristics are supported on Linux, macOS, and
Windows.

## Benchmarking

Use `scripts/bench-logdx.py` for repeatable local performance checks. It builds
the release binary when needed, generates synthetic plain/JSONL/logfmt logs,
and reports elapsed time, throughput, output size, and best-effort child RSS:

```bash
python3 scripts/bench-logdx.py --lines 1000000 --format jsonl --cardinality 10000
```

## Error Codes

Standard axt error codes are available through `--list-errors`. Common
`axt-logdx` failures map to:

- `usage_error`: no input or invalid time filter.
- `path_not_found`: an input path does not exist.
- `permission_denied`: an input file cannot be read.
- `io_error`: filesystem, stdin, or output serialization failed.
- `output_truncated_strict`: output was truncated under `--strict`.
