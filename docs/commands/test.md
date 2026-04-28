# axt-test

`axt-test` runs one or more project test suites and normalizes their results.

## Usage

```bash
axt-test
axt-test --framework jest
axt-test --filter "checkout flow"
axt-test --files tests/checkout.test.ts
axt-test --changed
axt-test --changed-since main
axt-test --bail
axt-test --workers 4
axt-test --top-failures 10
axt-test --include-output
axt-test --pass-through -- --framework-specific-flag
axt-test list-frameworks
```

Shared flags are available before the subcommand or run options: `--json`, `--json-data`, `--jsonl`, `--agent`, `--plain`, `--print-schema`, `--list-errors`, `--limit`, `--max-bytes`, and `--strict`.

## Output

JSON uses the stable `axt.test.v1` envelope. JSONL streams `axt.test.case.v1` records as the underlying framework reports them, then emits `axt.test.suite.v1` and `axt.test.summary.v1` records at the end. Agent mode emits compact ACF records:

```text
schema=axt.test.agent.v1 ok=false mode=records frameworks=jest total=3 passed=1 failed=1 skipped=1 todo=0 ms=120 started=2026-04-27T10:12:00Z truncated=false
U name="checkout flow" framework=jest file=tests/checkout.test.ts passed=1 failed=1 skipped=1 ms=23
C status=failed name=fails framework=jest file=tests/checkout.test.ts line=20 ms=12 suite="checkout flow" message="expected 200, got 500"
S run="axt-test"
```

Human mode prints a compact summary and expands only failed tests. `--include-output` includes captured stdout/stderr for failed cases when the framework provides it.

## Framework Mapping

| Framework | Detection | Command | Filter mapping | File mapping | Notes |
|---|---|---|---|---|---|
| jest | `package.json` script or dependency mentions `jest` | `npm test --` | positional pattern | appended paths | Parses Jest JSON documents and normalized line records. |
| vitest | `package.json` script or dependency mentions `vitest` | `npm test --` | positional pattern | appended paths | Uses the project test script to avoid `npx` network behavior. |
| pytest | `pyproject.toml` mentions `pytest` | `python -m pytest -q` | `-k <PATTERN>` | appended paths | Parses pytest-json-report style documents and normalized line records. |
| cargo test | `Cargo.toml` | `cargo test -- --nocapture` | positional pattern | appended paths | Stable text output is parsed as fallback because libtest JSON requires unstable flags. |
| go test | `go.mod` | `go test -json ./...` | `-run <PATTERN>` | appended paths | Parses native Go JSON test events. |
| bun test | `package.json` mentions `bun` | `bun test` | `--test-name-pattern <PATTERN>` | appended paths | Requires Bun installed. |
| deno test | `deno.json` | `deno test --reporter=json` | `--filter <PATTERN>` | appended paths | Requires Deno installed. |

Detection order is explicit `--framework`, then `axt-test.toml`, `[tool.axt-test]` in `pyproject.toml`, `package.json#axt-test.framework`, then project marker files.

When multiple project roots are detected below the current directory, `axt-test` runs each detected framework and merges the result records. `--single` refuses that case. `--framework <name>` forces one framework at the current directory.

## Changed Files

`--changed` uses `axt-git` repository status and runs only changed paths that look like test files. `--changed-since <REF>` uses `axt-git` tree diff from `<REF>` to `HEAD`. Both modes require a readable Git worktree and exit with `git_unavailable` if no repository is found.

## Cross-platform Notes

Detection and output normalization are platform-neutral. Jest, Vitest, and Pytest work the same on Linux, macOS, and Windows when their local toolchains are installed. Cargo, Go, Bun, and Deno work where those toolchains are installed. Missing framework commands exit with `feature_unsupported`.

## Error Codes

Standard axt error codes are available through `--list-errors`. Common `axt-test` failures map to:

- `usage_error`: no framework detected or `--single` refused multiple detections.
- `feature_unsupported`: the required framework command is unavailable.
- `git_unavailable`: changed-file filtering was requested outside a readable Git worktree.
- `command_failed`: one or more tests failed.
- `io_error`: output or process IO failed.
