# axt-test

`axt-test` detects local test frameworks, runs their native command, and emits a
normalized result model. It does not replace framework runners; it wraps them so
humans, scripts, and agents can read one stable schema.

## Usage

```bash
axt-test [OPTIONS] [-- <FRAMEWORK_FLAGS>...]
axt-test --framework cargo
axt-test --framework jest --filter "checkout flow"
axt-test --files tests/checkout.test.ts
axt-test --changed
axt-test --changed-since main
axt-test --bail --workers 4
axt-test --top-failures 10 --include-output --agent
axt-test list-frameworks
```

Shared flags are available before the subcommand or run options: `--json`,
`--agent`, `--print-schema`, `--list-errors`, `--limit`, `--max-bytes`, and
`--strict`. TTY stdout defaults to human mode; non-TTY stdout defaults to
compact text.

## Options

| Option | Description |
|---|---|
| `--framework <NAME>` | Force `jest`, `vitest`, `pytest`, `cargo`, `go`, `bun`, or `deno`. |
| `--filter <PATTERN>` | Map a test-name filter to the underlying framework. |
| `--files <PATH>` | Run only selected test files. Repeatable. |
| `--changed` | Use Git status and run changed paths that look like tests. |
| `--changed-since <REF>` | Run changed test paths between `<REF>` and `HEAD`. |
| `--single` | Fail if detection finds more than one framework/project. |
| `--bail` | Ask the framework to stop at the first failure where supported. |
| `--workers <N>` | Map worker count to frameworks that support it. |
| `--top-failures <N>` | Maximum failed case records in compact agent output. Default `5`. |
| `--failures-only` | Suppress passing/skipped case records. Agent mode enables this by default. |
| `--rerun-failed` | Failure-focused shortcut; currently relies on framework-level filtering rather than persisted failure IDs. |
| `--include-output` | Include captured per-case stdout/stderr when the parser receives it. |
| `--no-include-output` | Do not include per-case stdout/stderr. |
| `--pass-through -- <FLAGS>` | Append raw framework flags after a `--` separator. |
| `list-frameworks` | Print supported frameworks and detection markers. |

## Examples

Auto-detect and run local tests with agent output:

```bash
axt-test --agent
```

Force Cargo and return the stable JSON envelope:

```bash
axt-test --framework cargo --json
```

Run only tests whose names match a framework filter:

```bash
axt-test --framework pytest --filter checkout --include-output
```

Pass framework-specific flags after the separator:

```bash
axt-test --framework cargo --pass-through -- --test integration_smoke
```

List supported frameworks and detection markers:

```bash
axt-test list-frameworks
```

## Detection

Detection order:

1. Explicit `--framework`.
2. `axt-test.toml`.
3. `[tool.axt-test]` in `pyproject.toml`.
4. `package.json#axt-test.framework`.
5. Local marker files in the current directory and one directory level below.

Detected markers:

| Framework | Marker |
|---|---|
| `deno` | `deno.json` |
| `go` | `go.mod` |
| `cargo` | `Cargo.toml` |
| `pytest` | `pyproject.toml` mentioning `pytest` |
| `vitest` | `package.json` mentioning `vitest` |
| `jest` | `package.json` mentioning `jest` |
| `bun` | `package.json` mentioning `bun` |

When multiple projects are detected, `axt-test` runs them in deterministic order
and merges the normalized results. `--single` turns that into a usage error.

## Framework Commands

| Framework | Command shape | Filter mapping | File mapping | Parser support |
|---|---|---|---|---|
| `jest` | `npm test --` | positional pattern | appended paths | Jest-style JSON documents and normalized JSON line records. |
| `vitest` | `npm test --` | positional pattern | appended paths | Vitest/Jest-like JSON documents and normalized JSON line records. |
| `pytest` | `python -m pytest -q` | `-k <PATTERN>` | appended paths | pytest-json-report-like documents and normalized JSON line records. |
| `cargo` | `cargo test -- --nocapture` | positional pattern | appended paths | Stable text fallback and normalized JSON line records. |
| `go` | `go test -json ./...` | `-run <PATTERN>` | appended paths | Native Go JSON event stream. |
| `bun` | `bun test` | `--test-name-pattern <PATTERN>` | appended paths | Bun-like JSON documents and normalized JSON line records. |
| `deno` | `deno test --reporter=json` | `--filter <PATTERN>` | appended paths | Deno JSON documents and normalized JSON line records. |

The current implementation is stable for the seven frameworks above. It does
not bundle custom reporters for Mocha, RSpec, AVA, JUnit, Gradle, or .NET yet.
For frameworks with unstable or unavailable native machine output, `axt-test`
uses deterministic fallback parsers covered by fixtures.

## Output

TTY stdout defaults to human mode. Non-TTY stdout defaults to compact text.
`--json` and `--agent` are explicit structured modes.

Human mode prints totals and expands failed tests:

```text
frameworks=jest total=3 passed=1 failed=1 skipped=1 todo=0 ms=120
FAILED checkout.test.ts fails
  expected 200, got 500
```

Compact mode is the default for non-TTY capture:

```text
test frameworks=jest total=3 passed=1 failed=1 skipped=1 todo=0 ms=120
fail framework=jest file=checkout.test.ts line=20 name=fails
```

JSON mode emits `axt.test.v1`:

```json
{
  "schema": "axt.test.v1",
  "ok": false,
  "data": {
    "frameworks": ["jest"],
    "total": 3,
    "passed": 1,
    "failed": 1,
    "skipped": 1,
    "todo": 0,
    "duration_ms": 120,
    "cases": []
  },
  "warnings": [],
  "errors": []
}
```

Agent mode is streaming JSONL. To preserve live failure reporting, it emits an
initial zero-count summary first, then case records as they are parsed, suite
records, and a final summary with the real totals. Consumers should treat the
last `axt.test.summary.v1` record as authoritative.

```jsonl
{"schema":"axt.test.summary.v1","type":"summary","frameworks":[],"total":0,"passed":0,"failed":0,"skipped":0,"todo":0,"duration_ms":0,"started":"2026-04-27T10:12:00Z","truncated":false,"next":[]}
{"schema":"axt.test.case.v1","type":"case","framework":"jest","status":"failed","name":"fails","suite":"checkout flow","file":"checkout.test.ts","line":20,"duration_ms":12,"failure":{"message":"expected 200, got 500","stack":null,"actual":"500","expected":"200","diff":null},"stdout":null,"stderr":null}
{"schema":"axt.test.suite.v1","type":"suite","framework":"jest","name":"checkout flow","file":"checkout.test.ts","passed":1,"failed":1,"skipped":1,"todo":0,"duration_ms":23}
{"schema":"axt.test.summary.v1","type":"summary","frameworks":["jest"],"total":3,"passed":1,"failed":1,"skipped":1,"todo":0,"duration_ms":120,"started":"2026-04-27T10:12:00Z","truncated":false,"next":["axt-test --rerun-failed --include-output --agent","axt-test --top-failures 5 --include-output --json"]}
```

Agent record schemas:

- `axt.test.summary.v1`
- `axt.test.case.v1`
- `axt.test.suite.v1`
- `axt.test.framework.v1`
- `axt.test.warn.v1`

## Changed Files

`--changed` uses the local Git worktree status and runs changed paths that look
like test files. `--changed-since <REF>` uses a tree diff from `<REF>` to
`HEAD`. Both require a readable Git worktree and return `git_unavailable` when
Git context is absent.

## Stability Notes

`axt-test` is production-stable for normalized execution of the seven supported
frameworks when their local toolchains are installed. Residual limitations are
documented behavior:

- Cargo uses stable text parsing instead of nightly-only libtest JSON.
- Jest and Vitest use the project `npm test` script to avoid `npx` network
  behavior.
- Missing framework executables return `feature_unsupported`.
- Framework-specific failure IDs are not persisted, so `--rerun-failed` is a
  compact output shortcut rather than a cross-framework exact rerun database.

## Cross-Platform Notes

Detection and output normalization are platform-neutral. Cargo and Go work
where their toolchains work. Node, Python, Bun, and Deno frameworks require
their local commands to be installed and available on `PATH`. No framework
command is downloaded or installed by `axt-test`.

## Error Codes

Standard axt error codes are available through `--list-errors`. Common
`axt-test` failures map to:

- `usage_error`: no framework detected, invalid arguments, or `--single`
  refused multiple detections.
- `feature_unsupported`: required framework command is unavailable.
- `git_unavailable`: changed-file filtering was requested outside a readable
  Git worktree.
- `command_failed`: one or more tests failed or a framework command exited
  non-zero.
- `io_error`: process IO, output, or serialization failed.
- `output_truncated_strict`: output was truncated under `--strict`.
