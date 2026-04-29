# `axt` Foundation CLI Suite — Spec Addendum: Additional Commands

**Status**: Addendum to `axt-spec-v2.md`. Apply on top of the v2 spec.
**Adds**: `axt-port` (Phase 5), `axt-test` (Phase 6), `axt-outline` (Phase 7), `axt-ctxpack` (Phase 8).
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
- `axt-peek` — directory & repo snapshot.
- Shared library crates.
- Full release pipeline.

**Phase 2–4 (after Phase 1 is shipping):**
- `axt-run` — observable command execution.
- `axt-doc` — environment & toolchain doctor.
- `axt-drift` — filesystem diff from a marker.

**Phase 5–6 (added in this addendum):**
- `axt-port` — port-occupancy inspection and reclaim, cross-platform.
- `axt-test` — test runner normalizer for jest, pytest, cargo, go, vitest, etc.

**Phase 7 (evolutive command):**
- `axt-outline` — compact local source outlines for declarations, signatures, docs, visibility, paths, and ranges.

**Phase 8 (evolutive command):**
- `axt-ctxpack` — bounded multi-pattern, multi-file local context search for coding agents.

**Total surface**: 8 binaries after Phase 8.

---

## Updated binary table (replaces section 2.2 of v2 spec)

| Binary | Phase | One-line purpose |
|---|---|---|
| `axt-peek` | 1 | Snapshot of a directory + repo + git + language metadata in one shot. |
| `axt-run` | 2 | Run a command and produce a structured envelope of what happened. |
| `axt-doc` | 3 | Diagnose the dev environment: PATH, version managers, env vars. |
| `axt-drift` | 4 | Mark filesystem state, then later report what changed since the mark. |
| `axt-port` | 5 | Find and (optionally) free processes that hold TCP/UDP ports. |
| `axt-test` | 6 | Run a project's test suite and emit normalized JSON plus agent JSONL for agents, regardless of framework. |
| `axt-outline` | 7 | Emit compact local source outlines without function bodies. |
| `axt-ctxpack` | 8 | Search local files for multiple named regex patterns with compact snippets and tree-sitter hit classification in one bounded call. |

---

## 11.3 — `axt-port` (Phase 5)

### 11.3.1 Purpose

Eliminate the cross-platform pain of "address already in use". Find which process holds a TCP or UDP port, return structured info about it, and optionally free the port. One binary, one schema, three operating systems.

### 11.3.2 Why this exists

Today, the workflow is:
- Linux: `lsof -i :3000` then `kill -9 PID`, or `ss -tulpn | grep 3000`, or `fuser -k 3000/tcp`.
- macOS: `lsof -i :3000` then `kill -9 PID`. Sometimes `lsof` returns nothing for root-owned processes and you need `sudo lsof`.
- Windows: `netstat -ano | findstr :3000` then `taskkill /PID nnn /F`. Or in PowerShell, `Get-NetTCPConnection -LocalPort 3000`.

Every developer has hit this. Tools that solve it (e.g., `kill-my-port`, `kill-port-process`) are npm-only, target one ecosystem, return text, and have no agent mode. There is no single static binary that does this consistently with structured output.

`axt-port` does. It is small, focused, and the agent value is high: one call replaces a 3-step OS-specific recipe.

### 11.3.3 CLI surface

```
axt-port list                          # all listening ports
axt-port who <PORT> [<PORT>...]        # who holds these ports
axt-port free <PORT> [<PORT>...]       # send termination signal to holders
axt-port watch <PORT>                  # poll until the port is free or held

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

  + standard --json/--agent/--agent//--limit/--max-bytes/--strict
  + --print-schema, --list-errors
```

### 11.3.4 Output: human mode

```
$ axt-port who 3000
Port 3000 (tcp, listening)
  PID 47281    node    /Users/dario/projects/api    "node server.js"
  Bound:       0.0.0.0:3000  ::1:3000
  Started:     2026-04-27T08:14:22Z (12m ago)
  Owner:       dario
  Memory:      182.4 MB
```

```
$ axt-port list
Port    Proto  PID    Process       Bound          State
3000    tcp    47281  node          0.0.0.0:3000   LISTEN
5432    tcp    1284   postgres      127.0.0.1:5432 LISTEN
8080    tcp    52144  python        ::1:8080       LISTEN
```

```
$ axt-port free 3000
Port 3000 held by PID 47281 (node)
Sent SIGTERM. Waiting up to 3s...
Port 3000 freed.
```

### 11.3.5 Output: agent mode (agent JSONL)

```jsonl
{"schema":"axt.port.summary.v1","type":"summary","action":"who","port":3000,"proto":"tcp","held":true,"holders":1,"truncated":false,"next":[]}
{"schema":"axt.port.holder.v1","type":"holder","port":3000,"proto":"tcp","pid":47281,"name":"node","command":"node server.js","cwd":"/Users/dario/projects/api","bound":"0.0.0.0:3000","owner":"dario","memory_bytes":190840832,"started":"2026-04-27T08:14:22Z"}
```

For `free`:

```jsonl
{"schema":"axt.port.summary.v1","type":"summary","action":"free","port":3000,"freed":true,"signal_sent":"term","escalated":false,"duration_ms":1240,"truncated":false,"next":[]}
{"schema":"axt.port.action.v1","type":"action","port":3000,"pid":47281,"name":"node","signal":"term","result":"freed","duration_ms":1240}
```

For an unfreeable port (process won't die or insufficient permissions):

```jsonl
{"schema":"axt.port.summary.v1","type":"summary","action":"free","port":3000,"freed":false,"truncated":false,"next":["sudo axt-port free 3000"]}
{"schema":"axt.port.warn.v1","type":"warn","code":"permission_denied","port":3000,"pid":47281,"name":"system_daemon","owner":"root"}
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
- `sysinfo` — already in workspace deps for `axt-doc`/`axt-drift`. Provides cross-platform process enumeration with command line, owner, memory.
- `netstat2` or hand-rolled bindings to platform APIs for socket→PID mapping. `sysinfo` does not currently expose this on all platforms; this is the only platform-specific code in the binary.
- `nix` for Unix signals; `windows` crate for `TerminateProcess`.

### 11.3.7 Safety considerations

This is the only command in the suite that **mutates external state by default**. We treat that with respect:

- `axt-port free` is the only mutating subcommand. `list`, `who`, `watch` are read-only.
- `--dry-run` is supported on `free` and produces the same JSON/JSONL schema and agent JSONL keys with `freed: false` and an `action: simulated` flag.
- `--confirm` requires interactive y/n if stdout is a TTY. Non-interactive (agent) calls bypass this — the agent is responsible for explicit consent in its own loop.
- We refuse to kill PID 1 always. We refuse to kill the current process. We refuse to kill our own parent unless `--force-self` is passed (which prints a stderr warning).
- We respect process trees: `--tree` propagates the signal to children (process group on Unix, Job Object on Windows).
- The signal escalation (`term` → `kill` after `--grace`) is documented and configurable. Default 3s grace because dev servers usually shut down cleanly within that window.

### 11.3.8 Definition of done for v0.5

1. `list` returns all listening sockets on all three OSes, structured.
2. `who <port>` returns full holder info with PID, command, owner, bind addresses.
3. `free <port>` actually frees the port on all three OSes; `--dry-run` works.
4. `watch <port>` polls until the port is held or freed, with a `--timeout` option.
5. JSON output validates against `axt.port.v1`; JSONL records validate against their record schemas; agent output follows agent JSONL.
6. Snapshot tests on a fixture that spawns a known-port-listener process.
7. Cross-platform CI runs the full suite. Where a feature degrades (cwd on Windows), the test asserts graceful degradation, not failure.
8. `docs/commands/port.md` written, with safety section explicit.

### 11.3.9 What `axt-port` is not

- Not a network sniffer. We do not capture packets.
- Not a firewall manager. We do not modify rules.
- Not a port scanner against remote hosts. The scope is **local sockets only**. (`axt-port who example.com:443` returns a usage error.)
- Not a docker-port-mapper. Containers have their own port namespace; we report what the host sees.

---

## 11.4 — `axt-test` (Phase 6)

### 11.4.1 Purpose

Run a project's test suite and emit normalized JSON plus agent JSONL, regardless of which framework is being used. The agent calls `axt-test`, gets back a known schema, and never has to learn the JSON shapes of jest, pytest, cargo test, go test, vitest, mocha, junit, rspec, deno test, bun test, etc.

### 11.4.2 Why this exists

The pain is concrete:

- A monorepo can have a Rust crate, a TypeScript app, a Python ML script. An agent fixing a bug across them runs three test commands, parses three different `--json` schemas, and merges three different concepts of "failure", "duration", "skipped".
- Frameworks update their JSON shapes between major versions. jest 28 vs 30 differ. pytest output through `pytest-json-report` differs from `pytest --json-report`.
- Some frameworks have no machine output at all by default (e.g., bare `mocha` requires a custom reporter; `go test` needs `-json`).
- Agents waste tokens parsing partial output, retrying when JSON is invalid, or asking the user "what test runner is this?".

`axt-test` solves all of these by detecting the framework, invoking it correctly, parsing whatever native machine output exists, and re-emitting in a stable schema.

### 11.4.3 CLI surface

```
axt-test                                   # auto-detect and run
axt-test --framework jest                  # force a framework
axt-test --filter <PATTERN>                # pass-through to the framework's name filter
axt-test --files <PATH>...                 # run only specified files
axt-test --changed                         # only test files that changed in git
axt-test --changed-since <REF>             # files changed since a ref
axt-test --bail                            # stop at first failure
axt-test --workers <N>                     # set parallelism (per-framework mapping)
axt-test --top-failures <N>                # only emit the first N failure records (default 5)
axt-test --include-output / --no-include-output   # include stdout/stderr per failed test (default: only failed)
axt-test --pass-through -- <FRAMEWORK_FLAGS>  # raw flags to the underlying runner
axt-test list-frameworks                   # what we support and how we detect

  + standard --json/--agent/--agent//--limit/--max-bytes/--strict
```

### 11.4.4 Framework auto-detection

Order of detection:

1. Explicit `--framework <name>`.
2. `axt-test.toml` or `[tool.axt-test]` in `pyproject.toml` / `package.json#axt-test`.
3. Package files inspected:
   - `package.json#scripts.test` and `package.json#devDependencies` for jest, vitest, mocha, ava, jasmine, bun.
   - `Cargo.toml` for `cargo test` (workspaces detected).
   - `go.mod` for `go test ./...`.
   - `pyproject.toml` for pytest / unittest.
   - `Gemfile` for rspec / minitest.
   - `deno.json` for `deno test`.
4. If multiple frameworks detected (monorepo), `axt-test` runs each in turn and merges output, prefixing path with subproject. `--single` to refuse.

### 11.4.5 Normalized output schema

Agent mode (agent JSONL):

```jsonl
{"schema":"axt.test.summary.v1","type":"summary","frameworks":["jest"],"total":124,"passed":118,"failed":3,"skipped":3,"todo":0,"duration_ms":12405,"started":"2026-04-27T10:12:00Z","truncated":false,"next":["axt-test --rerun-failed --include-output --agent"]}
{"schema":"axt.test.case.v1","type":"case","framework":"jest","status":"failed","name":"creates an order with a discount code","suite":"checkout flow","file":"tests/checkout.test.ts","line":47,"duration_ms":234,"failure":{"message":"expected 200, got 500","stack":null,"actual":"500","expected":"200","diff":null},"stdout":null,"stderr":null}
{"schema":"axt.test.case.v1","type":"case","framework":"jest","status":"failed","name":"applies tax for EU customers","suite":"checkout flow","file":"tests/checkout.test.ts","line":89,"duration_ms":118,"failure":{"message":"Internal server error: undefined is not a function","stack":null,"actual":null,"expected":null,"diff":null},"stdout":null,"stderr":null}
```

Human mode prints a compact table with only failures expanded; success cases are summarized. Agent mode defaults to failure-only case records. `--include-output` shows stdout/stderr for failed cases.

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

For frameworks without native JSON, we run the framework with our own reporter where supported (e.g., `mocha --reporter <axt-test-bundled-reporter>`), or parse text output as a fallback. The agent should not need to know which path was taken.

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

Crates to consider: `serde_json::Deserializer::into_iter` for streaming JSON; `regex` for fallback text parsers; `tokio::process` for async invocation (already in workspace for `axt-run`).

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
2. Agent JSONL schemas validated for every supported framework against committed fixtures; snapshots cover compact agent output.
3. Streaming: an initial summary appears first, then failures appear in `--agent` as they happen before the final summary.
4. `--changed` and `--changed-since` integrate with `axt-git` to filter affected files.
5. Cross-platform: jest and pytest work the same on Linux/macOS/Windows. cargo and go test work where their toolchain works.
6. `docs/commands/test.md` documents every framework's mapping in a table.

### 11.4.10 What `axt-test` is not

- Not a test runner itself. It does not discover or execute tests independently of an underlying framework.
- Not a benchmarking tool. `cargo bench`, `vitest bench`, etc., are out of scope.
- Not a coverage reporter. Coverage is per-framework and orthogonal.
- Not a flake-detector. We report results; flake analysis is for higher-level tools.

---

## 11.5 — `axt-outline` (Phase 7)

### 11.5.1 Purpose

`axt-outline` emits compact source outlines: declarations, signatures, doc comments, symbol kinds, visibility, file paths, and source ranges without function bodies. It gives agents a low-token map of supported local source files or directories before they decide which full bodies to read.

### 11.5.2 CLI surface

```
axt-outline [PATHS]...
axt-outline crates/axt-test/src --agent
axt-outline src/lib.rs --public-only --json
axt-outline . --agent --limit 100 --max-bytes 32768

  --lang rust|typescript|javascript|python|go|java|php
  --public-only
  --symbols-only
  --max-depth <N>
  --sort path|name|kind|source
```

Standard shared flags apply: `--json`, `--agent`, `--print-schema`, `--list-errors`, `--limit`, `--max-bytes`, and `--strict`.

### 11.5.3 Scope

- Rust (`*.rs`), TypeScript (`*.ts`, `*.tsx`, `*.mts`, `*.cts`), JavaScript (`*.js`, `*.jsx`, `*.mjs`, `*.cjs`), Python (`*.py`), Go (`*.go`), Java (`*.java`), and PHP (`*.php`) files and directories containing those files.
- Supported languages use embedded tree-sitter grammars and do not require external parser binaries, LSP servers, or network access.
- Top-level declarations plus common nested class/interface/module/trait/impl/member declarations.
- Symbol fields: `path`, `language`, `kind`, `visibility`, `name`, `signature`, `docs`, `range`, and `parent`.
- Optional alias binary `outline` behind the `aliases` feature.
- Schema prefix `axt.outline.v1`.

Unsupported extensions in mixed input produce `unsupported_language` warnings. If no supported source files are present, the command exits with `feature_unsupported` (exit 9).

### 11.5.4 Output contract

JSON uses the `axt.outline.v1` envelope. JSON data includes:

```json
{
  "root": ".",
  "summary": {"files": 1, "symbols": 3, "warnings": 0, "source_bytes": 8192, "signature_bytes": 240, "truncated": false},
  "symbols": [],
  "warnings": [],
  "next": []
}
```

Agent JSONL records:

- `axt.outline.summary.v1`
- `axt.outline.symbol.v1`
- `axt.outline.warn.v1`

Agent mode:

```jsonl
{"schema":"axt.outline.summary.v1","type":"summary","ok":true,"root":".","files":1,"symbols":3,"warnings":0,"source_bytes":8192,"signature_bytes":240,"truncated":false,"next":["axt-slice src/lib.rs --symbol parse_config --agent"]}
{"schema":"axt.outline.symbol.v1","type":"symbol","p":"src/lib.rs","l":"rust","k":"fn","vis":"pub","n":"parse_config","sig":"pub fn parse_config(input: &str) -> Result<Config, Error>","docs":"Parse the configuration text.","range":{"start_line":42,"end_line":57},"parent":null}
```

`--symbols-only` keeps symbol records to `name`, `kind`, and `line`.

### 11.5.5 Definition of done for v0.7

1. New crate `crates/axt-outline` and binary `axt-outline`.
2. Optional alias `outline` behind the `aliases` feature.
3. Rust, TypeScript, JavaScript, Python, Go, Java, and PHP support files and directories.
4. Standard output modes, schemas, `--print-schema`, and `--list-errors`.
5. Truncation through `--limit`, `--max-bytes`, and `--strict`.
6. Unsupported non-Rust files handled gracefully.
7. `docs/commands/outline.md`, `docs/man/axt-outline.1`, and `docs/skills/axt-outline/SKILL.md`.
8. Fixture and snapshot tests for output modes plus focused tests for supported-language symbols, visibility, docs, ranges, parse errors, unsupported files, language filtering, and truncation.

### 11.5.6 Deferred scope

- LSP-backed symbol ranking or cross-file semantic ranking.
- Full repository graph computation.
- LSP-backed symbol resolution and cross-file graph ranking. The tree-sitter layer is intentionally declaration-focused.

## 11.6 — `axt-ctxpack` (Phase 8)

### 11.6.1 Purpose

`axt-ctxpack` performs multi-pattern, multi-file context search in one bounded local call. It is for agents that would otherwise run several `rg` commands and then read line ranges manually. It goes beyond `rg --json` by correlating named patterns and classifying hits through embedded tree-sitter parsers where possible.

### 11.6.2 CLI surface

```
axt-ctxpack --pattern todo=TODO --pattern panic='unwrap\(|expect\(' src --json
axt-ctxpack --files 'crates/**/*.rs' --pattern public='pub fn' --context 2 --agent
axt-ctxpack --print-schema json

  ROOT...
  --pattern <NAME=REGEX>       repeatable named regex
  --files <GLOB>               include glob, repeatable
  --include <GLOB>             include glob, repeatable
  --context <N>                context lines around each match; default 0
  --max-depth <N>              directory traversal depth; default 16
  --hidden                     include hidden files
  --no-ignore                  disable ignore and gitignore filters
```

Standard shared flags apply: `--json`, `--agent`, `--print-schema`, `--list-errors`, `--limit`, `--max-bytes`, and `--strict`.

### 11.6.3 Scope

- Local regex/text search only.
- Multiple repeated named patterns using `--pattern name=REGEX`.
- File and directory roots.
- Include globs via `--files` and `--include`.
- Gitignore-aware directory traversal by default.
- Deterministic ordering by path, pattern order, and source position.
- JSON per-hit fields: `pattern`, `path`, `line`, `column`, `byte_range`, `kind`, `classification_source`, `language`, `node_kind`, `enclosing_symbol`, `ast_path`, `matched_text`, and `snippet`.
- Agent per-hit fields use short keys and omit `ast_path`: `pat`, `p`, `line`, `col`, `range`, `k`, `src`, `l`, `node`, `sym`, `text`, and `snippet`.
- Tree-sitter hit classification for supported languages: `code`, `comment`, `string`, `test`, or `unknown`.
- Heuristic fallback only when the file language is unsupported or the source cannot be parsed.
- Optional alias binary `ctxpack` behind the `aliases` feature.
- Schema prefix `axt.ctxpack.v1`.

### 11.6.4 Output contract

JSON uses the `axt.ctxpack.v1` envelope. JSON data includes:

```json
{
  "root": ".",
  "patterns": [{"name": "todo", "query": "TODO", "kind": "regex"}],
  "summary": {"roots": 1, "files_scanned": 10, "files_matched": 1, "hits": 3, "warnings": 0, "bytes_scanned": 8192, "truncated": false},
  "hits": [
    {
      "pattern": "todo",
      "path": "src/lib.rs",
      "line": 12,
      "column": 5,
      "byte_range": {"start": 240, "end": 244},
      "kind": "comment",
      "classification_source": "ast",
      "language": "rust",
      "node_kind": "line_comment",
      "enclosing_symbol": null,
      "ast_path": ["line_comment", "source_file"],
      "matched_text": "TODO",
      "snippet": "12:// TODO: tighten this"
    }
  ],
  "warnings": [],
  "next": ["axt-ctxpack src/lib.rs --pattern todo=TODO --context 2 --agent"]
}
```

Agent JSONL records:

- `axt.ctxpack.summary.v1`
- `axt.ctxpack.hit.v1`
- `axt.ctxpack.warn.v1`

Agent mode:

```jsonl
{"schema":"axt.ctxpack.summary.v1","type":"summary","ok":true,"root":".","patterns":2,"files_scanned":10,"files_matched":1,"hits":3,"warnings":0,"bytes_scanned":8192,"truncated":false,"next":["axt-ctxpack src/lib.rs --pattern todo=TODO --context 2 --agent"]}
{"schema":"axt.ctxpack.hit.v1","type":"hit","pat":"todo","p":"src/lib.rs","line":12,"col":5,"range":{"start":240,"end":244},"k":"comment","src":"ast","l":"rust","node":"line_comment","sym":null,"text":"TODO","snippet":"12:// TODO: tighten this"}
```

`ast_path` is retained in the JSON envelope for exact parser debugging and omitted from agent records to save tokens.

### 11.6.5 Definition of done for v0.8

1. New crate `crates/axt-ctxpack` and binary `axt-ctxpack`.
2. Optional alias `ctxpack` behind the `aliases` feature.
3. Standard output modes, schemas, `--print-schema`, and `--list-errors`.
4. Named regex patterns, roots, include globs, gitignore-aware walking, context lines, and deterministic ordering.
5. Hit fields and tree-sitter kind classification with documented heuristic fallback.
6. Truncation through `--limit`, `--max-bytes`, and `--strict`.
7. `docs/commands/ctxpack.md`, `docs/man/axt-ctxpack.1`, and `docs/skills/axt-ctxpack/SKILL.md`.
8. Fixture and snapshot tests for output modes plus focused tests for named patterns, overlapping hits, no hits, hidden files, ignored files, binary skipping, snippets, and truncation.

### 11.6.6 Deferred scope

- Semantic search.
- Embeddings.
- Remote repository search.
- Rewrite or edit application.
- Full AST query language.
- LSP-backed semantic ranking or cross-file symbol graphs.

---

## Updated cross-platform matrix (additions to section 4)

| Capability | Linux | macOS | Windows | Notes |
|---|---|---|---|---|
| `axt-port`: list listening sockets | ✅ | ✅ | ✅ | |
| `axt-port`: who-has-port | ✅ | ✅ | ✅ | |
| `axt-port`: free port (TERM/KILL) | ✅ | ✅ | ✅ | |
| `axt-port`: PID → cwd | ✅ via `/proc` | ⚠️ best-effort via `libproc` | ⚠️ requires elevation; field is `null` otherwise | |
| `axt-port`: process tree kill | ✅ | ✅ | ✅ via Job Object | |
| `axt-test`: jest, vitest | ✅ | ✅ | ✅ | |
| `axt-test`: pytest | ✅ | ✅ | ✅ | |
| `axt-test`: cargo test | ✅ | ✅ | ✅ | |
| `axt-test`: go test | ✅ | ✅ | ✅ | |
| `axt-test`: bun, deno | ✅ | ✅ | ✅ | requires the toolchain installed |
| `axt-test`: streaming output | ✅ | ✅ | ✅ | |
| `axt-outline`: directory traversal | ✅ | ✅ | ✅ | symlinks are not followed |
| `axt-outline`: Rust outlines | ✅ | ✅ | ✅ | embedded tree-sitter grammar |
| `axt-outline`: TypeScript/JavaScript outlines | ✅ | ✅ | ✅ | embedded tree-sitter grammars |
| `axt-outline`: Python outlines | ✅ | ✅ | ✅ | embedded tree-sitter grammar |
| `axt-outline`: Go outlines | ✅ | ✅ | ✅ | embedded tree-sitter grammar |
| `axt-outline`: Java outlines | ✅ | ✅ | ✅ | embedded tree-sitter grammar |
| `axt-outline`: PHP outlines | ✅ | ✅ | ✅ | embedded tree-sitter grammar |
| `axt-outline`: LSP ranking | ❌ | ❌ | ❌ | deferred; no external server dependency |
| `axt-ctxpack`: text regex search | ✅ | ✅ | ✅ | Rust regex engine; no network or external tools |
| `axt-ctxpack`: gitignore traversal | ✅ | ✅ | ✅ | uses the shared ignore-aware filesystem walker |
| `axt-ctxpack`: UTF-8 path output | ✅ | ✅ | ⚠️ | non-UTF-8 paths are skipped with warnings |
| `axt-ctxpack`: AST classification | ✅ | ✅ | ✅ | embedded tree-sitter grammars for Rust, TypeScript, JavaScript, Python, Go, Java, and PHP |
| `axt-ctxpack`: heuristic fallback | ✅ | ✅ | ✅ | used only for unsupported languages or parse errors |

---

## Updated implementation plan (additions to section 14)

### Milestone 5 — `axt-port` (target: 5–7 days)

After `axt-drift` is shipping in v0.4. Build, test, ship.

Steps:
1. New crate `crates/axt-port`.
2. Cross-platform socket→PID mapping (the only platform-specific code; abstract behind a trait).
3. CLI surface from 11.3.3.
4. Renderers: human, JSON, agent.
5. Process tree kill on Unix (process group) and Windows (Job Object).
6. Snapshot tests with a fixture that spawns a port-listener subprocess.
7. Documentation.

Done criteria: see 11.3.8.

### Milestone 6 — `axt-test` (target: 10–14 days)

This is the largest single binary in the suite. Plan accordingly.

Steps:
1. New crate `crates/axt-test`.
2. Define `TestFrontend` trait and `NormalizedEvent` union.
3. Implement frontends in priority order: jest, pytest, cargo test, go test, vitest, bun, deno.
4. Streaming infrastructure: line-buffered async reads, per-frontend parsers.
5. Auto-detection.
6. Monorepo / multi-framework mode.
7. CLI, renderers.
8. Snapshot tests using committed fixture project trees that include a tiny passing+failing test for each framework.
9. Documentation including the mapping table.

Done criteria: see 11.4.9.

### Milestone 7 — `axt-outline` (target: 3–5 days)

Build `axt-outline`. Do not add other new commands. Do not add LSP or semantic ranking in this milestone.

Steps:
1. Add the command contract in this addendum.
2. New crate `crates/axt-outline`.
3. Implement Rust, TypeScript, JavaScript, Python, Go, Java, and PHP file and directory outlining.
4. Implement renderers for every standard output mode.
5. Add schema, docs, man page, and skill.
6. Add fixtures, snapshots, and focused tests.
7. Run all standard quality gates.

Done criteria: see 11.5.5.

### Milestone 8 — `axt-ctxpack` (target: 3–5 days)

Build `axt-ctxpack`. Do not add other new commands. Do not add semantic search, embeddings, edit application, or a full AST query language in this milestone.

Steps:
1. Add the command contract in this addendum.
2. New crate `crates/axt-ctxpack`.
3. Implement named regex patterns, root traversal, include globs, snippets, and tree-sitter hit classification.
4. Implement renderers for every standard output mode.
5. Add schema, docs, man page, and skill.
6. Add fixtures, snapshots, and focused tests.
7. Run all standard quality gates.

Done criteria: see 11.6.5.

---

## Decisions deferred (additions to section 15)

These commands were considered for the suite and explicitly rejected. Future maintainers should not relitigate without new evidence:

- **`axt-watch`** (file watcher with JSONL events). Reason: `watchexec` is mature and widely installed; agents are session-based and rarely benefit from continuous watching; the unique value-add (JSONL events) is small.
- **`axt-log`** (log analyzer with error extraction). Reason: `lnav` and `goaccess` cover this for humans; for agents, `axt-run --tail-bytes` already extracts errors from the run we just executed, which is the common case. Standalone log post-mortem is a niche.
- **`axt-net`** (network diagnostic structured). Reason: `curl --json` plus `jc dig` already give structured network output; nothing meaningful to add.
- **`axt-deps`** (cross-package-manager dependency analyzer). Reason: Each ecosystem has tools that already produce JSON (`npm ls --json`, `pnpm why`, `cargo metadata`, `pip show --format=json`); a unifying layer would either lose fidelity or be enormous. Better to let agents call the native tools.
- **`axt-git`** (git status/log/diff agent-friendly). Reason: `gix` is complex; `axt-peek --changed` and `--changed-since` cover the high-frequency cases; `git status --porcelain=v2` is stable and parsable. Insufficient new value to justify another binary.
- **`axt-bench`** (benchmark runner normalizer). Reason: too niche, agent value is low (benchmarks are rarely in the agent's loop).

If a clear, repeated pain point emerges in the v1.0 roadmap that one of these would solve, reopen the question with measurements, not opinions.

---

## Continuation prompt for Phase 5 (use after Phase 4 ships)

> Continue with Milestone 5 from the spec addendum. Implement `axt-port` per the spec in section 11.3. Preserve all existing public schemas. Add tests before or alongside code, including a fixture process that listens on a known port. Run `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`. Verify the safety guarantees in section 11.3.7 with explicit tests (refuse to kill PID 1, refuse to kill self, etc.). Stop when the milestone's Done criteria in 11.3.8 are met.

## Continuation prompt for Phase 6 (use after Phase 5 ships)

> Continue with Milestone 6 from the spec addendum. Implement `axt-test` per the spec in section 11.4. Define the `TestFrontend` trait first, then implement frontends one at a time in the priority order: jest, pytest, cargo test, go test, vitest, bun, deno. Each frontend gets a committed fixture project (one passing test, one failing test, one skipped test) and snapshot tests against normalized JSONL plus agent JSONL agent output. Streaming is required for `--agent`: events must appear as they happen, not at the end. Run all standard quality gates. Stop when the milestone's Done criteria in 11.4.9 are met.
