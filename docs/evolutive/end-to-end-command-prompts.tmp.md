# End-to-End Command Implementation Prompts

This is a copy/paste prompt bank for future evolutive work. Each prompt is
scoped to one command or one existing-command evolution and uses the current
implemented suite contract:

- Primary modes: human, `--json`, and `--agent`.
- `--agent` is minified summary-first JSONL.
- Shared flags: `--print-schema`, `--list-errors`, `--limit`, `--max-bytes`,
  and `--strict`.
- Retired public flags: `--plain`, `--json-data`, and `--jsonl`.
- No network calls, telemetry, `unwrap()`, or `expect()` in non-test code.
- Diagnostics go to stderr; data goes to stdout.

Use one prompt per fresh implementation session. If the command is still only
an evolutive proposal, the prompt explicitly requires a spec/addendum contract
before code.

## Prompt 1: `axt-slice`

```text
Implement `axt-slice` end to end for the `axt` Foundation CLI Suite.

This session is scoped only to `axt-slice`. Do not implement any other new
command. If `axt-slice` is not yet approved in `docs/spec.md` or
`docs/spec-addendum.md`, first add an `axt-slice` contract to the addendum and
keep that spec change limited to this command.

Read first:
- `/Users/ddurzo/.codex/RTK.md`
- `AGENTS.md`
- `docs/agent-mode.md`
- `docs/error-catalog.md`
- `docs/evolutive/market-analysis.md`
- `docs/evolutive/axt-slice.md`
- Existing command docs under `docs/commands/`
- `docs/commands/outline.md`
- Existing crates `axt-outline`, `axt-ctxpack`, `axt-core`, and `axt-output`
- Existing skill files under `docs/skills/`

Goal:
Build `axt-slice`, a local single-binary command that extracts source by symbol
or enclosing line range, avoiding fragile manual line-range reads.

Required deliverables:
- Add the approved command contract to `docs/spec-addendum.md` if missing.
- Add `crates/axt-slice`.
- Add workspace membership and package metadata.
- Add binary `axt-slice`.
- Add optional alias `slice` behind the `aliases` feature.
- Support human, `--json`, and `--agent`.
- Support `--print-schema`, `--list-errors`, `--limit`, `--max-bytes`, and
  `--strict`.
- Use schema prefix `axt.slice.v1`.
- Implement extraction by `--symbol <NAME>` from one file.
- Implement `--line <N>` fallback that expands to the enclosing symbol.
- Include docs and attributes by default.
- Implement optional `--include-imports`, `--include-tests`,
  `--before-symbol`, and `--after-symbol` when feasible for the implemented
  languages.
- Detect ambiguous symbols and return candidate records instead of guessing.
- Add docs in `docs/commands/slice.md`.
- Add man page `docs/man/axt-slice.1`.
- Add skill `docs/skills/axt-slice/SKILL.md`.
- Update `scripts/agent/install-skills.py`.
- Add fixtures and snapshot tests for human, JSON, and agent output.
- Add focused tests for exact extraction, ambiguous symbols, line fallback,
  CRLF input, truncation, and binary/non-UTF-8 refusal.

Implementation constraints:
- Prefix shell commands with `rtk`.
- No `unwrap()` or `expect()` in non-test code.
- No network calls in binaries.
- Diagnostics go to stderr; data goes to stdout.
- Prefer existing tree-sitter and output patterns from `axt-outline` and
  `axt-ctxpack`.
- Use typed errors with `thiserror`; `anyhow` only at the binary edge.
- Keep output deterministic and compact.
- Document cross-platform behavior.

Before editing, reply with:
- Deliverables for this milestone.
- Files/crates you expect to modify.
- Output schemas and agent records.
- Tests to add.
- Ambiguities or risks.

Wait for approval before editing files.

After approval, implement and run:
- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`

Final response must use the status report template in `docs/agent-prompts.md`.
```

## Prompt 2: `axt-gitctx`

```text
Implement `axt-gitctx` end to end for the `axt` Foundation CLI Suite.

This session is scoped only to `axt-gitctx`. Do not implement any other new
command. If `axt-gitctx` is not yet approved in `docs/spec.md` or
`docs/spec-addendum.md`, first add an `axt-gitctx` contract to the addendum and
keep that spec change limited to this command.

Read first:
- `/Users/ddurzo/.codex/RTK.md`
- `AGENTS.md`
- `docs/agent-mode.md`
- `docs/error-catalog.md`
- `docs/evolutive/market-analysis.md`
- `docs/evolutive/axt-gitctx.md`
- Existing command docs under `docs/commands/`
- Existing crates `axt-git`, `axt-peek`, `axt-drift`, `axt-core`, and
  `axt-output`
- Existing command tests that create temporary Git repositories

Goal:
Build `axt-gitctx`, a local Git context command that returns branch, upstream,
ahead/behind, dirty state, changed files, diff stats, recent commits, and small
inline diffs in one bounded call.

Required deliverables:
- Add the approved command contract to `docs/spec-addendum.md` if missing.
- Add `crates/axt-gitctx`.
- Add workspace membership and package metadata.
- Add binary `axt-gitctx`.
- Add optional alias `gitctx` behind the `aliases` feature.
- Support human, `--json`, and `--agent`.
- Support `--print-schema`, `--list-errors`, `--limit`, `--max-bytes`, and
  `--strict`.
- Use schema prefix `axt.gitctx.v1`.
- Detect the current repository and return branch, upstream, ahead, behind, and
  dirty state.
- Return changed files with status, additions, deletions, hunk count, and size.
- Include recent commits with hash, subject, author, and timestamp/relative age
  where available.
- Include inline diffs only within `--inline-diff-max-bytes`.
- Never invoke network commands.
- Add docs in `docs/commands/gitctx.md`.
- Add man page `docs/man/axt-gitctx.1`.
- Add skill `docs/skills/axt-gitctx/SKILL.md`.
- Update `scripts/agent/install-skills.py`.
- Add fixtures and snapshot tests for human, JSON, and agent output.
- Add focused tests for clean, dirty, staged, untracked, renamed, deleted,
  no-git, ahead/behind with local bare remotes, inline diff thresholds, and
  truncation.

Implementation constraints:
- Prefix shell commands with `rtk`.
- No `unwrap()` or `expect()` in non-test code.
- No network calls in binaries.
- Diagnostics go to stderr; data goes to stdout.
- Prefer `axt-git` and stable local Git data over shelling out broadly.
- Use typed errors with `thiserror`; `anyhow` only at the binary edge.
- Keep output deterministic and compact.
- Document Windows symlink/executable-bit limitations honestly.

Before editing, reply with:
- Deliverables for this milestone.
- Files/crates you expect to modify.
- Output schemas and agent records.
- Tests to add.
- Ambiguities or risks.

Wait for approval before editing files.

After approval, implement and run:
- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`

Final response must use the status report template in `docs/agent-prompts.md`.
```

## Prompt 3: `axt-plan`

```text
Implement the first end-to-end milestone for `axt-plan`.

This session is scoped only to `axt-plan` dry-run planning. Do not implement
apply mode unless the approved spec already requires it for this milestone. If
`axt-plan` is not yet approved in `docs/spec.md` or `docs/spec-addendum.md`,
first add an `axt-plan` contract to the addendum and keep that spec change
limited to this command.

Read first:
- `/Users/ddurzo/.codex/RTK.md`
- `AGENTS.md`
- `docs/agent-mode.md`
- `docs/error-catalog.md`
- `docs/evolutive/market-analysis.md`
- `docs/evolutive/axt-plan.md`
- Existing command docs under `docs/commands/`
- Existing crates `axt-ctxpack`, `axt-fs`, `axt-core`, and `axt-output`
- Existing tests for truncation and snapshot output

Goal:
Build `axt-plan`, an auditable edit-plan command. The first milestone produces
dry-run plans for literal and regex replacements without writing files.

Required deliverables:
- Add the approved command contract to `docs/spec-addendum.md` if missing.
- Add `crates/axt-plan`.
- Add workspace membership and package metadata.
- Add binary `axt-plan`.
- Add optional alias `plan-edit` behind the `aliases` feature if approved.
- Support human, `--json`, and `--agent`.
- Support `--print-schema`, `--list-errors`, `--limit`, `--max-bytes`, and
  `--strict`.
- Use schema prefix `axt.plan.v1`.
- Implement dry-run literal replacement.
- Implement dry-run regex replacement.
- Read local UTF-8 text files only; refuse binary and non-UTF-8 files.
- Emit per-file match counts, hunks/diffs, pre-change hashes, plan checksum,
  and apply preconditions.
- Return no-match plans as successful, explicit empty plans.
- Add docs in `docs/commands/plan.md`.
- Add man page `docs/man/axt-plan.1`.
- Add skill `docs/skills/axt-plan/SKILL.md`.
- Update `scripts/agent/install-skills.py`.
- Add fixtures and snapshot tests for human, JSON, and agent output.
- Add focused tests for literal replacement, regex replacement, no-match,
  binary refusal, non-UTF-8 refusal, plan checksum, diff rendering, and
  truncation.

Implementation constraints:
- Prefix shell commands with `rtk`.
- No `unwrap()` or `expect()` in non-test code.
- No network calls in binaries.
- Diagnostics go to stderr; data goes to stdout.
- Do not write target files in this milestone.
- Prefer deterministic diff output over clever patch generation.
- Use typed errors with `thiserror`; `anyhow` only at the binary edge.
- Document apply mode as deferred unless implemented.

Before editing, reply with:
- Deliverables for this milestone.
- Files/crates you expect to modify.
- Output schemas and agent records.
- Tests to add.
- Ambiguities or risks.

Wait for approval before editing files.

After approval, implement and run:
- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`

Final response must use the status report template in `docs/agent-prompts.md`.
```

## Prompt 4: `axt-logsift`

```text
Implement `axt-logsift` end to end for the `axt` Foundation CLI Suite.

This session is scoped only to `axt-logsift`. Do not implement any other new
command. If `axt-logsift` is not yet approved in `docs/spec.md` or
`docs/spec-addendum.md`, first add an `axt-logsift` contract to the addendum and
keep that spec change limited to this command.

Read first:
- `/Users/ddurzo/.codex/RTK.md`
- `AGENTS.md`
- `docs/agent-mode.md`
- `docs/error-catalog.md`
- `docs/evolutive/market-analysis.md`
- `docs/evolutive/axt-logsift.md`
- Existing command docs under `docs/commands/`
- Existing crates `axt-run`, `axt-core`, and `axt-output`

Goal:
Build `axt-logsift`, a bounded local log triage command that reads files or
stdin and returns deduplicated error groups, stack traces, severity timelines,
and representative snippets.

Required deliverables:
- Add the approved command contract to `docs/spec-addendum.md` if missing.
- Add `crates/axt-logsift`.
- Add workspace membership and package metadata.
- Add binary `axt-logsift`.
- Add optional alias `logsift` behind the `aliases` feature.
- Support human, `--json`, and `--agent`.
- Support `--print-schema`, `--list-errors`, `--limit`, `--max-bytes`, and
  `--strict`.
- Use schema prefix `axt.logsift.v1`.
- Read one or more local files and stdin through `--stdin`.
- Detect plain text logs, JSONL logs, syslog-like timestamps, ANSI-colored
  logs, CRLF logs, and common JavaScript, Python, Rust, Go, and JVM stack
  traces.
- Filter by severity and parseable time range.
- Deduplicate repeated messages with counts and first/last occurrence.
- Emit top N groups and representative snippets.
- Add docs in `docs/commands/logsift.md`.
- Add man page `docs/man/axt-logsift.1`.
- Add skill `docs/skills/axt-logsift/SKILL.md`.
- Update `scripts/agent/install-skills.py`.
- Add fixtures and snapshot tests for human, JSON, and agent output.
- Add focused tests for plain logs, JSONL logs, syslog timestamps, ANSI
  stripping, CRLF logs, stack traces, dedup fingerprints, severity filters,
  time filters, large-file streaming, and truncation.

Implementation constraints:
- Prefix shell commands with `rtk`.
- No `unwrap()` or `expect()` in non-test code.
- No network calls in binaries.
- Diagnostics go to stderr; data goes to stdout.
- Do not implement live tailing or remote ingestion in this milestone.
- Use streaming reads for large files.
- Use typed errors with `thiserror`; `anyhow` only at the binary edge.
- Keep output deterministic and compact.

Before editing, reply with:
- Deliverables for this milestone.
- Files/crates you expect to modify.
- Output schemas and agent records.
- Tests to add.
- Ambiguities or risks.

Wait for approval before editing files.

After approval, implement and run:
- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`

Final response must use the status report template in `docs/agent-prompts.md`.
```

## Prompt 5: `axt-manifest`

```text
Implement `axt-manifest` end to end for the `axt` Foundation CLI Suite.

This session is scoped only to `axt-manifest`. Do not implement any other new
command. If `axt-manifest` is not yet approved in `docs/spec.md` or
`docs/spec-addendum.md`, first add an `axt-manifest` contract to the addendum
and keep that spec change limited to this command.

Read first:
- `/Users/ddurzo/.codex/RTK.md`
- `AGENTS.md`
- `docs/agent-mode.md`
- `docs/error-catalog.md`
- `docs/evolutive/market-analysis.md`
- `docs/evolutive/axt-manifest.md`
- Existing command docs under `docs/commands/`
- Existing crates `axt-bundle`, `axt-test`, `axt-fs`, `axt-core`, and
  `axt-output`

Goal:
Build `axt-manifest`, a local manifest normalization command for project
configuration files.

Required deliverables:
- Add the approved command contract to `docs/spec-addendum.md` if missing.
- Add `crates/axt-manifest`.
- Add workspace membership and package metadata.
- Add binary `axt-manifest`.
- Add optional alias `manifest` behind the `aliases` feature.
- Support human, `--json`, and `--agent`.
- Support `--print-schema`, `--list-errors`, `--limit`, `--max-bytes`, and
  `--strict`.
- Use schema prefix `axt.manifest.v1`.
- Detect and parse `Cargo.toml`, `package.json`, `tsconfig.json`,
  `pyproject.toml`, `go.mod`, `Dockerfile`, and `.github/workflows/*.yml`.
- Emit dependencies, dev dependencies, scripts/tasks, runtime versions, package
  names, workspace members, and CI job names where available.
- Preserve unknown sections as counts or summaries instead of silently dropping
  them.
- Add `--root <DIR>`, `--ecosystem <NAME>`, and `--include-ci` if approved in
  the command contract.
- Add docs in `docs/commands/manifest.md`.
- Add man page `docs/man/axt-manifest.1`.
- Add skill `docs/skills/axt-manifest/SKILL.md`.
- Update `scripts/agent/install-skills.py`.
- Add fixtures and snapshot tests for human, JSON, and agent output.
- Add focused tests for each ecosystem, malformed manifests, multi-workspaces,
  unknown section preservation, CI workflow parsing, and truncation.

Implementation constraints:
- Prefix shell commands with `rtk`.
- No `unwrap()` or `expect()` in non-test code.
- No network calls in binaries.
- Diagnostics go to stderr; data goes to stdout.
- Do not resolve lockfiles or fetch package metadata.
- Prefer structured parsers for JSON/TOML/YAML where dependencies already exist
  or are approved.
- Use typed errors with `thiserror`; `anyhow` only at the binary edge.
- Keep output deterministic and compact.

Before editing, reply with:
- Deliverables for this milestone.
- Files/crates you expect to modify.
- Output schemas and agent records.
- Tests to add.
- Ambiguities or risks.

Wait for approval before editing files.

After approval, implement and run:
- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`

Final response must use the status report template in `docs/agent-prompts.md`.
```

## Prompt 6: `axt-repomap`

```text
Implement `axt-repomap` end to end for the `axt` Foundation CLI Suite.

This session is scoped only to `axt-repomap`. Do not implement any other new
command. If `axt-repomap` is not yet approved in `docs/spec.md` or
`docs/spec-addendum.md`, first add an `axt-repomap` contract to the addendum and
keep that spec change limited to this command.

Read first:
- `/Users/ddurzo/.codex/RTK.md`
- `AGENTS.md`
- `docs/agent-mode.md`
- `docs/error-catalog.md`
- `docs/evolutive/market-analysis.md`
- `docs/evolutive/axt-repomap.md`
- Existing command docs under `docs/commands/`
- Existing crates `axt-bundle`, `axt-peek`, `axt-outline`, `axt-test`,
  `axt-git`, `axt-fs`, `axt-core`, and `axt-output`

Goal:
Build `axt-repomap`, a compact repository topology command that summarizes
layout, languages, build systems, entry points, tests, manifests, and recent
local Git context.

Required deliverables:
- Add the approved command contract to `docs/spec-addendum.md` if missing.
- Add `crates/axt-repomap`.
- Add workspace membership and package metadata.
- Add binary `axt-repomap`.
- Add optional alias `repomap` behind the `aliases` feature.
- Support human, `--json`, and `--agent`.
- Support `--print-schema`, `--list-errors`, `--limit`, `--max-bytes`, and
  `--strict`.
- Use schema prefix `axt.repomap.v1`.
- Detect repository root and workspace layout.
- Summarize directories by role: `source`, `tests`, `examples`, `docs`,
  `scripts`, `config`, `generated`, and `vendor`.
- Include detected languages, build systems, test frameworks, manifests, entry
  points, and recent local commits when Git is available.
- Include `next` hints for `axt-outline`, `axt-test`, and `axt-gitctx`.
- Add docs in `docs/commands/repomap.md`.
- Add man page `docs/man/axt-repomap.1`.
- Add skill `docs/skills/axt-repomap/SKILL.md`.
- Update `scripts/agent/install-skills.py`.
- Add fixtures and snapshot tests for human, JSON, and agent output.
- Add focused tests for multi-language repos, monorepos/workspaces, clean and
  dirty Git fixtures, no-Git directories, role classification, executable-bit
  platform behavior, and truncation.

Implementation constraints:
- Prefix shell commands with `rtk`.
- No `unwrap()` or `expect()` in non-test code.
- No network calls in binaries.
- Diagnostics go to stderr; data goes to stdout.
- Do not package full source contents or implement semantic ranking.
- Prefer reusing existing internal data shapes over inventing incompatible
  summaries.
- Use typed errors with `thiserror`; `anyhow` only at the binary edge.
- Keep output deterministic and compact.

Before editing, reply with:
- Deliverables for this milestone.
- Files/crates you expect to modify.
- Output schemas and agent records.
- Tests to add.
- Ambiguities or risks.

Wait for approval before editing files.

After approval, implement and run:
- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`

Final response must use the status report template in `docs/agent-prompts.md`.
```

## Prompt 7: `axt-impact`

```text
Implement the research-track first milestone for `axt-impact`.

This session is scoped only to `axt-impact`. Do not implement any other new
command. If `axt-impact` is not yet approved in `docs/spec.md` or
`docs/spec-addendum.md`, first add an `axt-impact` contract to the addendum and
keep that spec change limited to this command.

Read first:
- `/Users/ddurzo/.codex/RTK.md`
- `AGENTS.md`
- `docs/agent-mode.md`
- `docs/error-catalog.md`
- `docs/evolutive/market-analysis.md`
- `docs/evolutive/axt-impact.md`
- Existing command docs under `docs/commands/`
- Existing crates `axt-outline`, `axt-ctxpack`, `axt-test`, `axt-git`,
  `axt-core`, and `axt-output`

Goal:
Build a Rust-first `axt-impact` milestone that estimates the blast radius of a
symbol change by returning call sites, references, nearby tests, suggested
review files, engine confidence, and next hints.

Required deliverables:
- Add the approved command contract to `docs/spec-addendum.md` if missing.
- Add `crates/axt-impact`.
- Add workspace membership and package metadata.
- Add binary `axt-impact`.
- Add optional alias `impact` behind the `aliases` feature.
- Support human, `--json`, and `--agent`.
- Support `--print-schema`, `--list-errors`, `--limit`, `--max-bytes`, and
  `--strict`.
- Use schema prefix `axt.impact.v1`.
- Implement target selection by `--file <PATH> --line <N>` and
  `--symbol <NAME>` where practical.
- Use `rust-analyzer` only when available and local project configuration is
  valid; never install it or fetch anything.
- Implement deterministic local text/tree-sitter fallback with lower confidence.
- Return call sites with file, line, kind, confidence, and snippet.
- Suggest test files based on references, naming conventions, and local Git
  history where available.
- Add docs in `docs/commands/impact.md`.
- Add man page `docs/man/axt-impact.1`.
- Add skill `docs/skills/axt-impact/SKILL.md`.
- Update `scripts/agent/install-skills.py`.
- Add fixtures and snapshot tests for human, JSON, and agent output.
- Add focused tests for Rust fixture references, LSP-unavailable fallback,
  ambiguous symbols, confidence scoring, timeouts, process failures, and
  truncation.

Implementation constraints:
- Prefix shell commands with `rtk`.
- No `unwrap()` or `expect()` in non-test code.
- No network calls in binaries.
- Diagnostics go to stderr; data goes to stdout.
- Treat fallback results as incomplete and expose confidence clearly.
- Keep LSP process lifecycle bounded by timeout and robust cleanup.
- Use typed errors with `thiserror`; `anyhow` only at the binary edge.
- Keep output deterministic and compact.

Before editing, reply with:
- Deliverables for this milestone.
- Files/crates you expect to modify.
- Output schemas and agent records.
- Tests to add.
- Ambiguities or risks.

Wait for approval before editing files.

After approval, implement and run:
- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`

Final response must use the status report template in `docs/agent-prompts.md`.
```

## Prompt 8: `axt-test` Failure Digest Evolution

```text
Implement the `axt-test` failure digest evolution.

This session modifies only the existing `axt-test` command. Do not create a new
`testdigest` binary. If the digest behavior is not yet approved in
`docs/spec.md` or `docs/spec-addendum.md`, first add an `axt-test` evolution
contract to the addendum and keep that spec change limited to this behavior.

Read first:
- `/Users/ddurzo/.codex/RTK.md`
- `AGENTS.md`
- `docs/agent-mode.md`
- `docs/error-catalog.md`
- `docs/commands/test.md`
- `docs/evolutive/axt-test-evolution.md`
- Existing crate `crates/axt-test`
- Existing `axt-test` fixtures and snapshots

Goal:
Improve `axt-test` failure digest behavior so agents get stable failure IDs,
focused rerun hints, and compact failure-first output without learning
framework-specific result formats.

Required deliverables:
- Update the approved `axt-test` contract in `docs/spec-addendum.md` if needed.
- Keep all existing `axt.test.v1` compatibility unless a schema bump is
  explicitly approved.
- Add stable failure IDs, for example
  `<framework>:<path-or-suite>:<test-name>`.
- Add `--rerun-id <ID>` or an approved equivalent if feasible across supported
  frameworks.
- Add `next` hints in JSON and agent JSONL outputs:
  `axt-test --rerun-id <ID> --include-output --agent`.
- Improve parser tests for Cargo panic locations, Jest stack frames, Pytest
  assertion introspection, and Go JSON events.
- Ensure `command_failed` still represents failing tests, not parser failure.
- Update `docs/commands/test.md`.
- Update `docs/skills/axt-test/SKILL.md`.
- Add snapshots for failure-only human, JSON, and agent modes.
- Add truncation tests for large stderr/stdout blocks.

Implementation constraints:
- Prefix shell commands with `rtk`.
- No `unwrap()` or `expect()` in non-test code.
- No network calls in binaries.
- Diagnostics go to stderr; data goes to stdout.
- Do not add a standalone `testdigest` binary or alias unless explicitly
  approved.
- Preserve streaming `--agent` behavior: initial summary first, failure records
  as available, final authoritative summary last.
- Use typed errors with `thiserror`; `anyhow` only at the binary edge.

Before editing, reply with:
- Deliverables for this evolution.
- Files you expect to modify.
- Output/schema changes and compatibility impact.
- Tests to add.
- Ambiguities or risks.

Wait for approval before editing files.

After approval, implement and run:
- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`

Final response must use the status report template in `docs/agent-prompts.md`.
```

## Recommended Order

1. `axt-slice`
2. `axt-gitctx`
3. `axt-test` failure digest evolution
4. `axt-plan`
5. `axt-logsift`
6. `axt-manifest`
7. `axt-repomap`
8. `axt-impact`
