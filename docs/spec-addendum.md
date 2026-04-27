# `ax` Foundation CLI Suite — Spec Addendum: Two New Commands

**Status**: Addendum to `ax-spec-v2.md`. Apply on top of the v2 spec.
**Adds**: `ax-port` (Phase 5), `ax-test` (Phase 6).
**Modifies**: Section 0 (TL;DR), section 2.2 (binary names), section 4 (cross-platform matrix), section 14 (implementation plan).

---

## Why these two and not others

After surveying community pain points (Reddit, HN, GitHub issues, blog measurements like "How We Cut Our Claude Code Token Usage 2.8x", and the documented ~33% agent-token cost of parsing CLI output), two distinct gaps emerged that meet all of these criteria simultaneously:

1. The pain is **universal** (every developer hits it, on every project).
2. There is **no agent-friendly tool** in the space (existing tools are npm-only, OS-specific, or text-output).
3. The implementation is **bounded** (a clear, small surface).
4. The token-savings vs alternatives is **measurable** (one call vs many, structured vs free-text).

Other candidates were considered and rejected, with rationale recorded at the end of this document so future maintainers don't relitigate them.

---

## Updated TL;DR (replaces section 0 of v2 spec)

Build a small suite of single-binary CLI tools, written in Rust, designed to be agent-friendly and human-friendly. Each binary is independently installable.

**Phase 1 (ship first):**
- `ax-peek` — directory & repo snapshot.
- Shared library crates.
- Full release pipeline.

**Phase 2–4 (after Phase 1 is shipping):**
- `ax-run` — observable command execution.
- `ax-doc` — environment & toolchain doctor.
- `ax-drift` — filesystem diff from a marker.

**Phase 5–6 (added in this addendum):**
- `ax-port` — port-occupancy inspection and reclaim, cross-platform.
- `ax-test` — test runner normalizer for jest, pytest, cargo, go, vitest, etc.

**Total surface**: 6 binaries. After Phase 6 the suite is feature-complete for v1.0; no further commands are planned.

---

## Updated binary table (replaces section 2.2 of v2 spec)

| Binary | Phase | One-line purpose |
|---|---|---|
| `ax-peek` | 1 | Snapshot of a directory + repo + git + language metadata in one shot. |
| `ax-run` | 2 | Run a command and produce a structured envelope of what happened. |
| `ax-doc` | 3 | Diagnose the dev environment: PATH, version managers, env vars. |
| `ax-drift` | 4 | Mark filesystem state, then later report what changed since the mark. |
| `ax-port` | 5 | Find and (optionally) free processes that hold TCP/UDP ports. |
| `ax-test` | 6 | Run a project's test suite and emit normalized NDJSON, regardless of framework. |

---

## 11.3 — `ax-port` (Phase 5)

### 11.3.1 Purpose

Eliminate the cross-platform pain of "address already in use". Find which process holds a TCP or UDP port, return structured info about it, and optionally free the port. One binary, one schema, three operating systems.

### 11.3.2 Why this exists

Today, the workflow is:
- Linux: `lsof -i :3000` then `kill -9 PID`, or `ss -tulpn | grep 3000`, or `fuser -k 3000/tcp`.
- macOS: `lsof -i :3000` then `kill -9 PID`. Sometimes `lsof` returns nothing for root-owned processes and you need `sudo lsof`.
- Windows: `netstat -ano | findstr :3000` then `taskkill /PID nnn /F`. Or in PowerShell, `Get-NetTCPConnection -LocalPort 3000`.

Every developer has hit this. Tools that solve it (e.g., `kill-my-port`, `kill-port-process`) are npm-only, target one ecosystem, return text, and have no agent mode. There is no single static binary that does this consistently with structured output.

`ax-port` does. It is small, focused, and the agent value is high: one call replaces a 3-step OS-specific recipe.

### 11.3.3 CLI surface

```
ax-port list                          # all listening ports
ax-port who <PORT> [<PORT>...]        # who holds these ports
ax-port free <PORT> [<PORT>...]       # send termination signal to holders
ax-port watch <PORT>                  # poll until the port is free or held

  --proto tcp|udp|both            # default: tcp
  --signal term|kill|int          # for `free`; default: term, escalate to kill after --grace
  --grace <DURATION>              # how long to wait between term and kill; default: 3s
  --include-loopback              # default: true
  --listening-only                # default: true (vs all states for `list`)
  --host <ADDR>                   # filter by bind address (e.g., 0.0.0.0, 127.0.0.1, ::1)
  --owner <USER>                  # filter by process owner (Unix)
  --pid <PID>                     # show ports held by this PID (inverse lookup)
  --dry-run                       # for `free`: print what would be killed, do nothing
  --confirm                       # for `free`: require manual confirmation if interactive

  + standard --json/--agent/--plain/--limit/--max-bytes/--strict
  + --print-schema, --list-errors
```

### 11.3.4 Output: human mode

```
$ ax-port who 3000
Port 3000 (tcp, listening)
  PID 47281    node    /Users/dario/projects/api    "node server.js"
  Bound:       0.0.0.0:3000  ::1:3000
  Started:     2026-04-27T08:14:22Z (12m ago)
  Owner:       dario
  Memory:      182.4 MB
```

```
$ ax-port list
Port    Proto  PID    Process       Bound          State
3000    tcp    47281  node          0.0.0.0:3000   LISTEN
5432    tcp    1284   postgres      127.0.0.1:5432 LISTEN
8080    tcp    52144  python        ::1:8080       LISTEN
```

```
$ ax-port free 3000
Port 3000 held by PID 47281 (node)
Sent SIGTERM. Waiting up to 3s...
Port 3000 freed.
```

### 11.3.5 Output: agent mode (NDJSON)

```
{"s":"ax.port.summary.v1","t":"summary","ok":true,"action":"who","port":3000,"proto":"tcp","held":true,"holders":1}
{"s":"ax.port.holder.v1","t":"holder","port":3000,"proto":"tcp","pid":47281,"name":"node","cmd":"node server.js","cwd":"/Users/dario/projects/api","bound":["0.0.0.0:3000"],"owner":"dario","mem":190840832,"started":"2026-04-27T08:14:22Z"}
```

For `free`:

```
{"s":"ax.port.summary.v1","t":"summary","ok":true,"action":"free","port":3000,"freed":true,"signal_sent":"term","escalated":false,"ms":1240}
{"s":"ax.port.action.v1","t":"action","port":3000,"pid":47281,"name":"node","signal":"term","result":"freed","ms":1240}
```

For an unfreeable port (process won't die or insufficient permissions):

```
{"s":"ax.port.summary.v1","t":"summary","ok":false,"action":"free","port":3000,"freed":false}
{"s":"ax.port.err.v1","t":"err","code":"permission_denied","port":3000,"pid":47281,"context":{"name":"system_daemon","owner":"root"},"hint":"sudo ax-port free 3000"}
```

### 11.3.6 Cross-platform implementation

| Capability | Linux | macOS | Windows | How |
|---|---|---|---|---|
| List listening ports | ✅ | ✅ | ✅ | `sysinfo` crate + `procfs` (Linux) / `libproc` (macOS) / `iphlpapi` (Windows) |
| Map port → PID | ✅ | ✅ | ✅ | Same |
| PID → command line | ✅ | ✅ | ✅ | `sysinfo` |
| PID → cwd | ✅ via `/proc/<pid>/cwd` | ⚠️ best-effort via `libproc` | ⚠️ requires elevated; degrade gracefully | |
| Send TERM | ✅ via `kill(2)` | ✅ | ✅ via `GenerateConsoleCtrlEvent` or `TerminateProcess` | |
| Send KILL | ✅ via `kill(2)` SIGKILL | ✅ | ✅ via `TerminateProcess` | |
| UDP socket enumeration | ✅ | ✅ | ✅ | `sysinfo` + native APIs |
| IPv6 visibility | ✅ | ✅ | ✅ | |

If `cwd` cannot be obtained, the field is `null` in JSON and omitted in agent mode rather than fabricated.

Crates to use:
- `sysinfo` — already in workspace deps for `ax-doc`/`ax-drift`. Provides cross-platform process enumeration with command line, owner, memory.
- `netstat2` or hand-rolled bindings to platform APIs for socket→PID mapping. `sysinfo` does not currently expose this on all platforms; this is the only platform-specific code in the binary.
- `nix` for Unix signals; `windows` crate for `TerminateProcess`.

### 11.3.7 Safety considerations

This is the only command in the suite that **mutates external state by default**. We treat that with respect:

- `ax-port free` is the only mutating subcommand. `list`, `who`, `watch` are read-only.
- `--dry-run` is supported on `free` and produces the same NDJSON schema with `freed: false` and an `action: simulated` flag.
- `--confirm` requires interactive y/n if stdout is a TTY. Non-interactive (agent) calls bypass this — the agent is responsible for explicit consent in its own loop.
- We refuse to kill PID 1 always. We refuse to kill the current process. We refuse to kill our own parent unless `--force-self` is passed (which prints a stderr warning).
- We respect process trees: `--tree` propagates the signal to children (process group on Unix, Job Object on Windows).
- The signal escalation (`term` → `kill` after `--grace`) is documented and configurable. Default 3s grace because dev servers usually shut down cleanly within that window.

### 11.3.8 Definition of done for v0.5

1. `list` returns all listening sockets on all three OSes, structured.
2. `who <port>` returns full holder info with PID, command, owner, bind addresses.
3. `free <port>` actually frees the port on all three OSes; `--dry-run` works.
4. `watch <port>` polls until the port is held or freed, with a `--timeout` option.
5. NDJSON output validates against `ax.port.v1` schema.
6. Snapshot tests on a fixture that spawns a known-port-listener process.
7. Cross-platform CI runs the full suite. Where a feature degrades (cwd on Windows), the test asserts graceful degradation, not failure.
8. `docs/commands/port.md` written, with safety section explicit.

### 11.3.9 What `ax-port` is not

- Not a network sniffer. We do not capture packets.
- Not a firewall manager. We do not modify rules.
- Not a port scanner against remote hosts. The scope is **local sockets only**. (`ax-port who example.com:443` returns a usage error.)
- Not a docker-port-mapper. Containers have their own port namespace; we report what the host sees.

---

## 11.4 — `ax-test` (Phase 6)

### 11.4.1 Purpose

Run a project's test suite and emit a single normalized NDJSON stream, regardless of which framework is being used. The agent calls `ax-test`, gets back a known schema, and never has to learn the JSON shapes of jest, pytest, cargo test, go test, vitest, mocha, junit, rspec, deno test, bun test, etc.

### 11.4.2 Why this exists

The pain is concrete:

- A monorepo can have a Rust crate, a TypeScript app, a Python ML script. An agent fixing a bug across them runs three test commands, parses three different `--json` schemas, and merges three different concepts of "failure", "duration", "skipped".
- Frameworks update their JSON shapes between major versions. jest 28 vs 30 differ. pytest output through `pytest-json-report` differs from `pytest --json-report`.
- Some frameworks have no machine output at all by default (e.g., bare `mocha` requires a custom reporter; `go test` needs `-json`).
- Agents waste tokens parsing partial output, retrying when JSON is invalid, or asking the user "what test runner is this?".

`ax-test` solves all of these by detecting the framework, invoking it correctly, parsing whatever native machine output exists, and re-emitting in a stable schema.

### 11.4.3 CLI surface

```
ax-test                                   # auto-detect and run
ax-test --framework jest                  # force a framework
ax-test --filter <PATTERN>                # pass-through to the framework's name filter
ax-test --files <PATH>...                 # run only specified files
ax-test --changed                         # only test files that changed in git
ax-test --changed-since <REF>             # files changed since a ref
ax-test --bail                            # stop at first failure
ax-test --workers <N>                     # set parallelism (per-framework mapping)
ax-test --top-failures <N>                # only emit the first N failure records (default 5)
ax-test --include-output / --no-include-output   # include stdout/stderr per failed test (default: only failed)
ax-test --pass-through -- <FRAMEWORK_FLAGS>  # raw flags to the underlying runner
ax-test list-frameworks                   # what we support and how we detect

  + standard --json/--agent/--plain/--limit/--max-bytes/--strict
```

### 11.4.4 Framework auto-detection

Order of detection:

1. Explicit `--framework <name>`.
2. `ax-test.toml` or `[tool.ax-test]` in `pyproject.toml` / `package.json#ax-test`.
3. Package files inspected:
   - `package.json#scripts.test` and `package.json#devDependencies` for jest, vitest, mocha, ava, jasmine, bun.
   - `Cargo.toml` for `cargo test` (workspaces detected).
   - `go.mod` for `go test ./...`.
   - `pyproject.toml` for pytest / unittest.
   - `Gemfile` for rspec / minitest.
   - `deno.json` for `deno test`.
4. If multiple frameworks detected (monorepo), `ax-test` runs each in turn and merges output, prefixing path with subproject. `--single` to refuse.

### 11.4.5 Normalized output schema

Agent mode (NDJSON):

```
{"s":"ax.test.summary.v1","t":"summary","ok":false,"frameworks":["jest"],"total":124,"passed":118,"failed":3,"skipped":3,"todo":0,"ms":12405,"started":"2026-04-27T10:12:00Z"}
{"s":"ax.test.suite.v1","t":"suite","name":"checkout flow","file":"tests/checkout.test.ts","passed":12,"failed":2,"skipped":0,"ms":3402}
{"s":"ax.test.case.v1","t":"case","status":"failed","name":"creates an order with a discount code","suite":"checkout flow","file":"tests/checkout.test.ts","line":47,"ms":234,"failure":{"message":"expected 200, got 500","actual":500,"expected":200,"diff":null}}
{"s":"ax.test.case.v1","t":"case","status":"failed","name":"applies tax for EU customers","suite":"checkout flow","file":"tests/checkout.test.ts","line":89,"ms":118,"failure":{"message":"Internal server error: undefined is not a function","stack":"at applyTax (src/tax.ts:42:11)\n  at processOrder (src/checkout.ts:88:7)"}}
{"s":"ax.test.hint.v1","t":"hint","run":"ax-test --filter 'checkout flow' --include-output"}
```

Human mode prints a compact table with only failures expanded; success cases are summarized. `--include-output` shows stdout/stderr for failed cases.

### 11.4.6 What "normalized" means precisely

| Field | Source per framework |
|---|---|
| `status` | jest `testResults[].status` → maps `passed/failed/skipped/todo`; pytest outcome → same; cargo test event "ok"/"failed"/"ignored" → same; go test "PASS"/"FAIL"/"SKIP" → same. |
| `name` | The most specific name the framework provides (jest title chain, pytest nodeid, cargo `tests::module::test_fn`, go `TestFoo/sub`). |
| `file`, `line` | Always normalized to repo-relative path; line is best-effort, may be `null` for go. |
| `ms` | Always milliseconds, integer. Frameworks reporting seconds get converted. |
| `failure.message` | First line of the failure / panic / assertion error. |
| `failure.stack` | Full stack if the framework provides it. May be `null`. |
| `failure.actual` / `expected` / `diff` | Filled when the framework reports them (jest, vitest, rspec); `null` otherwise. |

For frameworks without native JSON, we run the framework with our own reporter where supported (e.g., `mocha --reporter <ax-test-bundled-reporter>`), or parse text output as a fallback. The agent should not need to know which path was taken.

### 11.4.7 Implementation strategy

Each supported framework gets a `Frontend` trait implementation:

```rust
trait TestFrontend {
    fn name(&self) -> &'static str;
    fn detect(workspace: &Workspace) -> bool;
    fn build_command(&self, opts: &TestOptions) -> Command;
    fn parse_stream(&self, stdout: impl BufRead, stderr: impl BufRead) -> impl Iterator<Item=NormalizedEvent>;
}
```

`NormalizedEvent` is the union of summary/suite/case events. The streaming parser is critical: long test runs must produce records as they arrive, not after the run completes. (jest's stream-json reporter, cargo test's `--format json -Z unstable-options`, go test's `-json`, pytest with `--report-output-format=json` all support streaming.)

Crates to consider: `serde_json::Deserializer::into_iter` for streaming JSON; `regex` for fallback text parsers; `tokio::process` for async invocation (already in workspace for `ax-run`).

### 11.4.8 What we will and will not support

**Will support in v0.6**:
- jest (TypeScript/JavaScript)
- vitest
- pytest
- cargo test (workspace-aware)
- go test
- bun test
- deno test

**Will support in v0.7+** if requested:
- mocha (with our bundled reporter)
- rspec
- ava
- xunit (.NET)
- gradle test (Java)

**Will not support**:
- Hand-rolled test scripts (the user calls `npm run my-tests` which is a bash script). We require a framework.
- Property-test results that don't fit pass/fail/skip without losing information (we report them as pass/fail and put metadata in `failure.context`).
- Snapshot diffs as first-class. They show in `failure.diff` as a string when the framework gives them.

### 11.4.9 Definition of done for v0.6

1. Auto-detection works for the seven primary frameworks.
2. NDJSON schema validated for every supported framework against committed fixtures.
3. Streaming: failures appear in the output as they happen, not at the end.
4. `--changed` and `--changed-since` integrate with `ax-git` to filter affected files.
5. Cross-platform: jest and pytest work the same on Linux/macOS/Windows. cargo and go test work where their toolchain works.
6. `docs/commands/test.md` documents every framework's mapping in a table.

### 11.4.10 What `ax-test` is not

- Not a test runner itself. It does not discover or execute tests independently of an underlying framework.
- Not a benchmarking tool. `cargo bench`, `vitest bench`, etc., are out of scope.
- Not a coverage reporter. Coverage is per-framework and orthogonal.
- Not a flake-detector. We report results; flake analysis is for higher-level tools.

---

## Updated cross-platform matrix (additions to section 4)

| Capability | Linux | macOS | Windows | Notes |
|---|---|---|---|---|
| `ax-port`: list listening sockets | ✅ | ✅ | ✅ | |
| `ax-port`: who-has-port | ✅ | ✅ | ✅ | |
| `ax-port`: free port (TERM/KILL) | ✅ | ✅ | ✅ | |
| `ax-port`: PID → cwd | ✅ via `/proc` | ⚠️ best-effort via `libproc` | ⚠️ requires elevation; field is `null` otherwise | |
| `ax-port`: process tree kill | ✅ | ✅ | ✅ via Job Object | |
| `ax-test`: jest, vitest | ✅ | ✅ | ✅ | |
| `ax-test`: pytest | ✅ | ✅ | ✅ | |
| `ax-test`: cargo test | ✅ | ✅ | ✅ | |
| `ax-test`: go test | ✅ | ✅ | ✅ | |
| `ax-test`: bun, deno | ✅ | ✅ | ✅ | requires the toolchain installed |
| `ax-test`: streaming output | ✅ | ✅ | ✅ | |

---

## Updated implementation plan (additions to section 14)

### Milestone 5 — `ax-port` (target: 5–7 days)

After `ax-drift` is shipping in v0.4. Build, test, ship.

Steps:
1. New crate `crates/ax-port`.
2. Cross-platform socket→PID mapping (the only platform-specific code; abstract behind a trait).
3. CLI surface from 11.3.3.
4. Renderers: human, JSON, agent.
5. Process tree kill on Unix (process group) and Windows (Job Object).
6. Snapshot tests with a fixture that spawns a port-listener subprocess.
7. Documentation.

Done criteria: see 11.3.8.

### Milestone 6 — `ax-test` (target: 10–14 days)

This is the largest single binary in the suite. Plan accordingly.

Steps:
1. New crate `crates/ax-test`.
2. Define `TestFrontend` trait and `NormalizedEvent` union.
3. Implement frontends in priority order: jest, pytest, cargo test, go test, vitest, bun, deno.
4. Streaming infrastructure: line-buffered async reads, per-frontend parsers.
5. Auto-detection.
6. Monorepo / multi-framework mode.
7. CLI, renderers.
8. Snapshot tests using committed fixture project trees that include a tiny passing+failing test for each framework.
9. Documentation including the mapping table.

Done criteria: see 11.4.9.

---

## Decisions deferred (additions to section 15)

These commands were considered for the suite and explicitly rejected. Future maintainers should not relitigate without new evidence:

- **`ax-watch`** (file watcher with NDJSON events). Reason: `watchexec` is mature and widely installed; agents are session-based and rarely benefit from continuous watching; the unique value-add (NDJSON events) is small.
- **`ax-log`** (log analyzer with error extraction). Reason: `lnav` and `goaccess` cover this for humans; for agents, `ax-run --tail-bytes` already extracts errors from the run we just executed, which is the common case. Standalone log post-mortem is a niche.
- **`ax-net`** (network diagnostic structured). Reason: `curl --json` plus `jc dig` already give structured network output; nothing meaningful to add.
- **`ax-deps`** (cross-package-manager dependency analyzer). Reason: Each ecosystem has tools that already produce JSON (`npm ls --json`, `pnpm why`, `cargo metadata`, `pip show --format=json`); a unifying layer would either lose fidelity or be enormous. Better to let agents call the native tools.
- **`ax-git`** (git status/log/diff agent-friendly). Reason: `gix` is complex; `ax-peek --changed` and `--changed-since` cover the high-frequency cases; `git status --porcelain=v2` is stable and parsable. Insufficient new value to justify another binary.
- **`ax-bench`** (benchmark runner normalizer). Reason: too niche, agent value is low (benchmarks are rarely in the agent's loop).

If a clear, repeated pain point emerges in the v1.0 roadmap that one of these would solve, reopen the question with measurements, not opinions.

---

## Continuation prompt for Phase 5 (use after Phase 4 ships)

> Continue with Milestone 5 from the spec addendum. Implement `ax-port` per the spec in section 11.3. Preserve all existing public schemas. Add tests before or alongside code, including a fixture process that listens on a known port. Run `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`. Verify the safety guarantees in section 11.3.7 with explicit tests (refuse to kill PID 1, refuse to kill self, etc.). Stop when the milestone's Done criteria in 11.3.8 are met.

## Continuation prompt for Phase 6 (use after Phase 5 ships)

> Continue with Milestone 6 from the spec addendum. Implement `ax-test` per the spec in section 11.4. Define the `TestFrontend` trait first, then implement frontends one at a time in the priority order: jest, pytest, cargo test, go test, vitest, bun, deno. Each frontend gets a committed fixture project (one passing test, one failing test, one skipped test) and snapshot tests against the normalized NDJSON output. Streaming is required: events must appear as they happen, not at the end. Run all standard quality gates. Stop when the milestone's Done criteria in 11.4.9 are met.
