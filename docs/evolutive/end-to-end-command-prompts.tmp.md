# Temporary End-to-End Command Implementation Prompts

This is a temporary copy-paste file. Delete it after use.

Each prompt is intended for a fresh implementation session. Run only one prompt
at a time. Each session must implement exactly one command or one `axt-test`
evolution end to end.

Important: each prompt explicitly authorizes the agent to update
`docs/spec.md` or `docs/spec-addendum.md` for the named command only, because
the current project rules require spec approval before new binaries are added.

## Prompt 1: `axt-outline`

```text
Implement `axt-outline` end to end for the `axt` Foundation CLI Suite.

You are authorized in this session to update `docs/spec.md` or
`docs/spec-addendum.md` only for the `axt-outline` command contract. Do not
change unrelated spec sections. Do not implement any other new command.

Read first:
- `/Users/ddurzo/.codex/RTK.md`
- `docs/evolutive/market-analysis.md`
- `docs/evolutive/axt-outline.md`
- Existing command docs under `docs/commands/`
- Existing skill files under `docs/skills/`
- Existing command crates, especially `axt-peek` and `axt-test`
- `docs/agent-mode.md`
- `docs/error-catalog.md`
- `docs/agent-prompts.md` section 6

Goal:
Build `axt-outline`, a local Rust single-binary command that emits compact
source outlines: declarations, signatures, doc comments, symbol kinds,
visibility, file paths, and source ranges, without function bodies.

Required end-to-end deliverables:
- Add the approved command contract to the spec/addendum.
- Add `crates/axt-outline`.
- Add workspace membership and package metadata.
- Add binary `axt-outline`.
- Add optional alias `outline` behind the `aliases` feature.
- Support standard modes: human, `--plain`, `--json`, `--json-data`,
  `--jsonl`, `--agent`, `--print-schema`, and `--list-errors`.
- Use schema prefix `axt.outline.v1`.
- Implement MVP support for Rust source files and directories.
- Add graceful unsupported-language handling for non-Rust files.
- Add truncation through `--limit`, `--max-bytes`, and `--strict`.
- Add docs in `docs/commands/outline.md`.
- Add man page `docs/man/axt-outline.1`.
- Add skill `docs/skills/axt-outline/SKILL.md`.
- Update `scripts/agent/install-skills.py`.
- Add fixtures and snapshot tests for all output modes.
- Add focused tests for Rust symbols, visibility, doc comments, ranges,
  parse errors, unsupported files, and truncation.

Implementation constraints:
- Prefix shell commands with `rtk`.
- No `unwrap()` or `expect()` in non-test code.
- No network calls in binaries.
- Diagnostics go to stderr; data goes to stdout.
- Prefer existing suite patterns over new abstractions.
- Use typed errors with `thiserror`; `anyhow` only at the binary edge.
- Keep output deterministic and compact.
- Cross-platform behavior must be documented.

Scope discipline:
Implement Rust MVP fully. Do not add TypeScript, Python, Go, Java, LSP, or
semantic ranking in this session. Mention them as deferred scope in docs.

Quality gates:
Run and fix:
- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`

Final response:
Use the status report template from `docs/agent-prompts.md` section 6. Include
files changed, tests run, any spec deviations, and next recommended milestone.
```

## Prompt 2: `axt-ctxpack`

```text
Implement `axt-ctxpack` end to end for the `axt` Foundation CLI Suite.

You are authorized in this session to update `docs/spec.md` or
`docs/spec-addendum.md` only for the `axt-ctxpack` command contract. Do not
change unrelated spec sections. Do not implement any other new command.

Read first:
- `/Users/ddurzo/.codex/RTK.md`
- `docs/evolutive/market-analysis.md`
- `docs/evolutive/axt-ctxpack.md`
- Existing command docs under `docs/commands/`
- Existing crates `axt-peek`, `axt-test`, `axt-core`, `axt-output`, and
  `axt-fs`
- `docs/agent-mode.md`
- `docs/error-catalog.md`
- `docs/agent-prompts.md` section 6

Goal:
Build `axt-ctxpack`, a local Rust single-binary command that performs
multi-pattern, multi-file context search in one bounded call for coding agents.

Required end-to-end deliverables:
- Add the approved command contract to the spec/addendum.
- Add `crates/axt-ctxpack`.
- Add workspace membership and package metadata.
- Add binary `axt-ctxpack`.
- Add optional alias `ctxpack` behind the `aliases` feature.
- Support standard modes: human, `--plain`, `--json`, `--json-data`,
  `--jsonl`, `--agent`, `--print-schema`, and `--list-errors`.
- Use schema prefix `axt.ctxpack.v1`.
- Implement repeated `--pattern name=REGEX`.
- Implement roots, include globs, gitignore-aware walking where existing suite
  primitives allow it, context lines, and deterministic ordering.
- Emit file, line, column, byte range when available, pattern name, matched
  text, snippet, and basic kind: `code`, `comment`, `string`, `test`, or
  `unknown`.
- Add truncation through `--limit`, `--max-bytes`, and `--strict`.
- Add docs in `docs/commands/ctxpack.md`.
- Add man page `docs/man/axt-ctxpack.1`.
- Add skill `docs/skills/axt-ctxpack/SKILL.md`.
- Update `scripts/agent/install-skills.py`.
- Add fixtures and snapshot tests for all output modes.
- Add focused tests for named patterns, overlapping hits, no hits, hidden
  files, ignored files, binary skipping, snippets, and truncation.

Implementation constraints:
- Prefix shell commands with `rtk`.
- No `unwrap()` or `expect()` in non-test code.
- No network calls in binaries.
- Diagnostics go to stderr; data goes to stdout.
- Prefer existing suite patterns over new abstractions.
- Use typed errors with `thiserror`; `anyhow` only at the binary edge.
- Keep output deterministic and compact.
- Cross-platform behavior must be documented.

Scope discipline:
Implement regex/text search MVP fully. Do not implement semantic search,
embeddings, edit application, or a full AST query language in this session.

Quality gates:
Run and fix:
- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`

Final response:
Use the status report template from `docs/agent-prompts.md` section 6. Include
files changed, tests run, any spec deviations, and next recommended milestone.
```

## Prompt 3: `axt-slice`

```text
Implement `axt-slice` end to end for the `axt` Foundation CLI Suite.

You are authorized in this session to update `docs/spec.md` or
`docs/spec-addendum.md` only for the `axt-slice` command contract. Do not
change unrelated spec sections. Do not implement any other new command.

Read first:
- `/Users/ddurzo/.codex/RTK.md`
- `docs/evolutive/market-analysis.md`
- `docs/evolutive/axt-slice.md`
- Existing command docs under `docs/commands/`
- Existing crates `axt-peek`, `axt-test`, `axt-core`, and `axt-output`
- `docs/agent-mode.md`
- `docs/error-catalog.md`
- `docs/agent-prompts.md` section 6

Goal:
Build `axt-slice`, a local Rust single-binary command that extracts source by
symbol or enclosing line range, avoiding fragile manual `sed -n` reads.

Required end-to-end deliverables:
- Add the approved command contract to the spec/addendum.
- Add `crates/axt-slice`.
- Add workspace membership and package metadata.
- Add binary `axt-slice`.
- Add optional alias `slice` behind the `aliases` feature.
- Support standard modes: human, `--plain`, `--json`, `--json-data`,
  `--jsonl`, `--agent`, `--print-schema`, and `--list-errors`.
- Use schema prefix `axt.slice.v1`.
- Implement Rust source extraction by `--symbol <NAME>`.
- Implement `--line <N>` as fallback that expands to the enclosing Rust symbol.
- Include docs and attributes by default when they immediately belong to the
  selected symbol.
- Return ambiguity candidates instead of guessing when multiple symbols match.
- Add truncation through `--limit`, `--max-bytes`, and `--strict`.
- Add docs in `docs/commands/slice.md`.
- Add man page `docs/man/axt-slice.1`.
- Add skill `docs/skills/axt-slice/SKILL.md`.
- Update `scripts/agent/install-skills.py`.
- Add fixtures and snapshot tests for all output modes.
- Add focused tests for Rust functions, impl methods, structs, doc comments,
  attributes, ambiguity, line fallback, CRLF input, and truncation.

Implementation constraints:
- Prefix shell commands with `rtk`.
- No `unwrap()` or `expect()` in non-test code.
- No network calls in binaries.
- Diagnostics go to stderr; data goes to stdout.
- Prefer existing suite patterns over new abstractions.
- Use typed errors with `thiserror`; `anyhow` only at the binary edge.
- Keep output deterministic and compact.
- Cross-platform behavior must be documented.

Scope discipline:
Implement Rust-only symbol slicing fully. Do not implement workspace-wide
resolution, LSP integration, import analysis, or non-Rust languages in this
session.

Quality gates:
Run and fix:
- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`

Final response:
Use the status report template from `docs/agent-prompts.md` section 6. Include
files changed, tests run, any spec deviations, and next recommended milestone.
```

## Prompt 4: `axt-gitctx`

```text
Implement `axt-gitctx` end to end for the `axt` Foundation CLI Suite.

You are authorized in this session to update `docs/spec.md` or
`docs/spec-addendum.md` only for the `axt-gitctx` command contract. Do not
change unrelated spec sections. Do not implement any other new command.

Read first:
- `/Users/ddurzo/.codex/RTK.md`
- `docs/evolutive/market-analysis.md`
- `docs/evolutive/axt-gitctx.md`
- Existing command docs under `docs/commands/`
- Existing crates `axt-git`, `axt-peek`, `axt-core`, and `axt-output`
- `docs/agent-mode.md`
- `docs/error-catalog.md`
- `docs/agent-prompts.md` section 6

Goal:
Build `axt-gitctx`, a local Rust single-binary command that returns compact
worktree context for agents: branch, upstream, ahead/behind, changed files,
diff stats, and recent commits.

Required end-to-end deliverables:
- Add the approved command contract to the spec/addendum.
- Add `crates/axt-gitctx`.
- Add workspace membership and package metadata.
- Add binary `axt-gitctx`.
- Add optional alias `gitctx` behind the `aliases` feature.
- Support standard modes: human, `--plain`, `--json`, `--json-data`,
  `--jsonl`, `--agent`, `--print-schema`, and `--list-errors`.
- Use schema prefix `axt.gitctx.v1`.
- Return repository root, branch, upstream, ahead/behind, dirty state, changed
  files with status/additions/deletions/hunk count where available, and recent
  commits.
- Include inline diffs only when under a strict byte budget.
- Add truncation through `--limit`, `--max-bytes`, and `--strict`.
- Add docs in `docs/commands/gitctx.md`.
- Add man page `docs/man/axt-gitctx.1`.
- Add skill `docs/skills/axt-gitctx/SKILL.md`.
- Update `scripts/agent/install-skills.py`.
- Add fixture tests using temporary git repositories.
- Add snapshot tests for all output modes.

Implementation constraints:
- Prefix shell commands with `rtk`.
- No `unwrap()` or `expect()` in non-test code.
- No network calls in binaries.
- Do not call remote git operations.
- Diagnostics go to stderr; data goes to stdout.
- Prefer existing suite patterns and `axt-git` helpers.
- Use typed errors with `thiserror`; `anyhow` only at the binary edge.
- Keep output deterministic and compact.
- Cross-platform behavior must be documented.

Scope discipline:
Implement local worktree context fully. Do not implement pull request metadata,
hosting-provider APIs, commit creation, or interactive diff viewing.

Quality gates:
Run and fix:
- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`

Final response:
Use the status report template from `docs/agent-prompts.md` section 6. Include
files changed, tests run, any spec deviations, and next recommended milestone.
```

## Prompt 5: `axt-plan`

```text
Implement `axt-plan` end to end for the `axt` Foundation CLI Suite.

You are authorized in this session to update `docs/spec.md` or
`docs/spec-addendum.md` only for the `axt-plan` command contract. Do not
change unrelated spec sections. Do not implement any other new command.

Read first:
- `/Users/ddurzo/.codex/RTK.md`
- `docs/evolutive/market-analysis.md`
- `docs/evolutive/axt-plan.md`
- Existing command docs under `docs/commands/`
- Existing crates `axt-peek`, `axt-core`, `axt-output`, and `axt-fs`
- `docs/agent-mode.md`
- `docs/error-catalog.md`
- `docs/agent-prompts.md` section 6

Goal:
Build `axt-plan`, a local Rust single-binary command that creates auditable
dry-run edit plans for broad literal or regex replacements. It must preview
changes safely and never write files in the first implementation.

Required end-to-end deliverables:
- Add the approved command contract to the spec/addendum.
- Add `crates/axt-plan`.
- Add workspace membership and package metadata.
- Add binary `axt-plan`.
- Add optional alias `plan-edit` behind the `aliases` feature.
- Support standard modes: human, `--plain`, `--json`, `--json-data`,
  `--jsonl`, `--agent`, `--print-schema`, and `--list-errors`.
- Use schema prefix `axt.plan.v1`.
- Implement dry-run literal replacement.
- Implement dry-run regex replacement.
- Emit plan ID, summary, file entries, match counts, pre-change hashes, and
  unified diffs.
- Refuse binary files and ambiguous unsafe input.
- Add truncation through `--limit`, `--max-bytes`, and `--strict`.
- Add docs in `docs/commands/plan.md`.
- Add man page `docs/man/axt-plan.1`.
- Add skill `docs/skills/axt-plan/SKILL.md`.
- Update `scripts/agent/install-skills.py`.
- Add fixtures and snapshot tests for all output modes.
- Add focused tests for literal replacement, regex replacement, no matches,
  binary refusal, diff rendering, plan IDs, hashes, and truncation.

Implementation constraints:
- Prefix shell commands with `rtk`.
- No `unwrap()` or `expect()` in non-test code.
- No network calls in binaries.
- This first milestone must not write target files.
- Diagnostics go to stderr; data goes to stdout.
- Prefer existing suite patterns over new abstractions.
- Use typed errors with `thiserror`; `anyhow` only at the binary edge.
- Keep output deterministic and compact.
- Cross-platform behavior must be documented.

Scope discipline:
Implement dry-run planning fully. Do not implement `--apply`, ast-grep
structural matching, backups, or atomic writes in this session.

Quality gates:
Run and fix:
- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`

Final response:
Use the status report template from `docs/agent-prompts.md` section 6. Include
files changed, tests run, any spec deviations, and next recommended milestone.
```

## Prompt 6: `axt-logsift`

```text
Implement `axt-logsift` end to end for the `axt` Foundation CLI Suite.

You are authorized in this session to update `docs/spec.md` or
`docs/spec-addendum.md` only for the `axt-logsift` command contract. Do not
change unrelated spec sections. Do not implement any other new command.

Read first:
- `/Users/ddurzo/.codex/RTK.md`
- `docs/evolutive/market-analysis.md`
- `docs/evolutive/axt-logsift.md`
- Existing command docs under `docs/commands/`
- Existing crates `axt-peek`, `axt-test`, `axt-core`, and `axt-output`
- `docs/agent-mode.md`
- `docs/error-catalog.md`
- `docs/agent-prompts.md` section 6

Goal:
Build `axt-logsift`, a local Rust single-binary command that triages large log
files or stdin into compact deduplicated error groups, severity summaries, and
representative snippets.

Required end-to-end deliverables:
- Add the approved command contract to the spec/addendum.
- Add `crates/axt-logsift`.
- Add workspace membership and package metadata.
- Add binary `axt-logsift`.
- Add optional alias `logsift` behind the `aliases` feature.
- Support standard modes: human, `--plain`, `--json`, `--json-data`,
  `--jsonl`, `--agent`, `--print-schema`, and `--list-errors`.
- Use schema prefix `axt.logsift.v1`.
- Read from paths and stdin.
- Parse plain text and JSONL log lines.
- Strip ANSI sequences.
- Detect severity where possible.
- Deduplicate repeated messages into stable fingerprints.
- Emit top groups, counts, first/last line, representative sample, and summary.
- Add truncation through `--limit`, `--max-bytes`, and `--strict`.
- Add docs in `docs/commands/logsift.md`.
- Add man page `docs/man/axt-logsift.1`.
- Add skill `docs/skills/axt-logsift/SKILL.md`.
- Update `scripts/agent/install-skills.py`.
- Add fixtures and snapshot tests for all output modes.
- Add focused tests for plain logs, JSONL logs, ANSI stripping, CRLF logs,
  dedup fingerprints, severity filtering, stdin, and truncation.

Implementation constraints:
- Prefix shell commands with `rtk`.
- No `unwrap()` or `expect()` in non-test code.
- No network calls in binaries.
- Diagnostics go to stderr; data goes to stdout.
- Prefer streaming parsing; do not load huge logs unnecessarily.
- Use typed errors with `thiserror`; `anyhow` only at the binary edge.
- Keep output deterministic and compact.
- Cross-platform behavior must be documented.

Scope discipline:
Implement offline triage fully. Do not implement live tailing, remote
ingestion, OpenTelemetry trace reconstruction, or a full query language.

Quality gates:
Run and fix:
- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`

Final response:
Use the status report template from `docs/agent-prompts.md` section 6. Include
files changed, tests run, any spec deviations, and next recommended milestone.
```

## Prompt 7: `axt-manifest`

```text
Implement `axt-manifest` end to end for the `axt` Foundation CLI Suite.

You are authorized in this session to update `docs/spec.md` or
`docs/spec-addendum.md` only for the `axt-manifest` command contract. Do not
change unrelated spec sections. Do not implement any other new command.

Read first:
- `/Users/ddurzo/.codex/RTK.md`
- `docs/evolutive/market-analysis.md`
- `docs/evolutive/axt-manifest.md`
- Existing command docs under `docs/commands/`
- Existing crates `axt-peek`, `axt-test`, `axt-core`, and `axt-output`
- `docs/agent-mode.md`
- `docs/error-catalog.md`
- `docs/agent-prompts.md` section 6

Goal:
Build `axt-manifest`, a local Rust single-binary command that normalizes common
project manifest files into one compact agent-first schema.

Required end-to-end deliverables:
- Add the approved command contract to the spec/addendum.
- Add `crates/axt-manifest`.
- Add workspace membership and package metadata.
- Add binary `axt-manifest`.
- Add optional alias `manifest` behind the `aliases` feature.
- Support standard modes: human, `--plain`, `--json`, `--json-data`,
  `--jsonl`, `--agent`, `--print-schema`, and `--list-errors`.
- Use schema prefix `axt.manifest.v1`.
- Implement MVP parsing for `Cargo.toml` and `package.json`.
- Emit packages, dependencies, dev dependencies, scripts, workspace members,
  runtime/tool hints where available, source file path, and unknown-section
  summaries.
- Add `--ecosystem rust|node|all`.
- Add truncation through `--limit`, `--max-bytes`, and `--strict`.
- Add docs in `docs/commands/manifest.md`.
- Add man page `docs/man/axt-manifest.1`.
- Add skill `docs/skills/axt-manifest/SKILL.md`.
- Update `scripts/agent/install-skills.py`.
- Add fixtures and snapshot tests for all output modes.
- Add focused tests for Cargo workspaces, package.json scripts, dependency
  scopes, malformed manifests, missing manifests, and truncation.

Implementation constraints:
- Prefix shell commands with `rtk`.
- No `unwrap()` or `expect()` in non-test code.
- No network calls in binaries.
- Do not query package registries.
- Diagnostics go to stderr; data goes to stdout.
- Prefer structured parsers over ad hoc string parsing.
- Use typed errors with `thiserror`; `anyhow` only at the binary edge.
- Keep output deterministic and compact.
- Cross-platform behavior must be documented.

Scope discipline:
Implement Cargo and package.json normalization fully. Do not implement
pyproject, go.mod, Dockerfile, CI YAML, vulnerability checks, lockfile solving,
or network metadata in this session.

Quality gates:
Run and fix:
- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`

Final response:
Use the status report template from `docs/agent-prompts.md` section 6. Include
files changed, tests run, any spec deviations, and next recommended milestone.
```

## Prompt 8: `axt-repomap`

```text
Implement `axt-repomap` end to end for the `axt` Foundation CLI Suite.

You are authorized in this session to update `docs/spec.md` or
`docs/spec-addendum.md` only for the `axt-repomap` command contract. Do not
change unrelated spec sections. Do not implement any other new command.

Read first:
- `/Users/ddurzo/.codex/RTK.md`
- `docs/evolutive/market-analysis.md`
- `docs/evolutive/axt-repomap.md`
- Existing command docs under `docs/commands/`
- Existing crates `axt-peek`, `axt-test`, `axt-git`, `axt-core`, and
  `axt-output`
- `docs/agent-mode.md`
- `docs/error-catalog.md`
- `docs/agent-prompts.md` section 6

Goal:
Build `axt-repomap`, a local Rust single-binary command that emits compact
repository topology: layout, detected languages, build systems, entry points,
test frameworks, manifest hints, and local git summary.

Required end-to-end deliverables:
- Add the approved command contract to the spec/addendum.
- Add `crates/axt-repomap`.
- Add workspace membership and package metadata.
- Add binary `axt-repomap`.
- Add optional alias `repomap` behind the `aliases` feature.
- Support standard modes: human, `--plain`, `--json`, `--json-data`,
  `--jsonl`, `--agent`, `--print-schema`, and `--list-errors`.
- Use schema prefix `axt.repomap.v1`.
- Detect repository root and summarize directories by role: `source`, `tests`,
  `examples`, `docs`, `scripts`, `config`, `generated`, `vendor`, `unknown`.
- Detect Rust and Node build/test markers.
- Detect primary language by file counts and bytes.
- Include entry point hints and local git state where available.
- Add truncation through `--limit`, `--max-bytes`, and `--strict`.
- Add docs in `docs/commands/repomap.md`.
- Add man page `docs/man/axt-repomap.1`.
- Add skill `docs/skills/axt-repomap/SKILL.md`.
- Update `scripts/agent/install-skills.py`.
- Add fixtures and snapshot tests for all output modes.
- Add focused tests for repository root detection, role classification,
  language summary, monorepo layout, no-git directories, and truncation.

Implementation constraints:
- Prefix shell commands with `rtk`.
- No `unwrap()` or `expect()` in non-test code.
- No network calls in binaries.
- Diagnostics go to stderr; data goes to stdout.
- Prefer existing suite primitives and data shapes.
- Use typed errors with `thiserror`; `anyhow` only at the binary edge.
- Keep output deterministic and compact.
- Cross-platform behavior must be documented.

Scope discipline:
Implement topology summary fully. Do not implement full repository packing,
semantic ranking, embeddings, or remote repository support.

Quality gates:
Run and fix:
- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`

Final response:
Use the status report template from `docs/agent-prompts.md` section 6. Include
files changed, tests run, any spec deviations, and next recommended milestone.
```

## Prompt 9: `axt-impact`

```text
Implement `axt-impact` end to end for the `axt` Foundation CLI Suite as a
bounded Rust-first MVP.

You are authorized in this session to update `docs/spec.md` or
`docs/spec-addendum.md` only for the `axt-impact` command contract. Do not
change unrelated spec sections. Do not implement any other new command.

Read first:
- `/Users/ddurzo/.codex/RTK.md`
- `docs/evolutive/market-analysis.md`
- `docs/evolutive/axt-impact.md`
- Existing command docs under `docs/commands/`
- Existing crates `axt-peek`, `axt-test`, `axt-git`, `axt-core`, and
  `axt-output`
- `docs/agent-mode.md`
- `docs/error-catalog.md`
- `docs/agent-prompts.md` section 6

Goal:
Build `axt-impact`, a local Rust single-binary command that estimates the blast
radius of changing a Rust symbol using deterministic local analysis first.

Required end-to-end deliverables:
- Add the approved command contract to the spec/addendum.
- Add `crates/axt-impact`.
- Add workspace membership and package metadata.
- Add binary `axt-impact`.
- Add optional alias `impact` behind the `aliases` feature.
- Support standard modes: human, `--plain`, `--json`, `--json-data`,
  `--jsonl`, `--agent`, `--print-schema`, and `--list-errors`.
- Use schema prefix `axt.impact.v1`.
- Implement Rust-first text/tree-sitter-style fallback analysis without
  requiring LSP.
- Accept `--file <PATH> --symbol <NAME>` and `--file <PATH> --line <N>`.
- Return target, engine kind, confidence, call/reference sites, likely tests,
  suggested review files, and next-step hints.
- Add confidence labels: `high`, `medium`, `low`.
- Add truncation through `--limit`, `--max-bytes`, and `--strict`.
- Add docs in `docs/commands/impact.md`.
- Add man page `docs/man/axt-impact.1`.
- Add skill `docs/skills/axt-impact/SKILL.md`.
- Update `scripts/agent/install-skills.py`.
- Add fixtures and snapshot tests for all output modes.
- Add focused tests for symbol references, ambiguous symbols, line target
  resolution, likely test detection, no references, and truncation.

Implementation constraints:
- Prefix shell commands with `rtk`.
- No `unwrap()` or `expect()` in non-test code.
- No network calls in binaries.
- Do not require `rust-analyzer` in this first milestone.
- Diagnostics go to stderr; data goes to stdout.
- Prefer deterministic local analysis over fragile process orchestration.
- Use typed errors with `thiserror`; `anyhow` only at the binary edge.
- Keep output deterministic and compact.
- Cross-platform behavior must be documented.

Scope discipline:
Implement the Rust fallback MVP fully. Do not implement LSP process
management, multi-language support, semantic call graphs, or build-system
execution in this session. Document those as deferred.

Quality gates:
Run and fix:
- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`

Final response:
Use the status report template from `docs/agent-prompts.md` section 6. Include
files changed, tests run, any spec deviations, and next recommended milestone.
```

## Prompt 10: `axt-test` Digest Evolution

```text
Implement the `axt-test` digest evolution end to end.

Do not create a new `axt-testdigest` binary. This session evolves the existing
`axt-test` command only.

You are authorized in this session to update `docs/spec.md` or
`docs/spec-addendum.md` only for the `axt-test` digest/failure-summary
contract. Do not change unrelated spec sections. Do not implement any other new
command.

Read first:
- `/Users/ddurzo/.codex/RTK.md`
- `docs/evolutive/market-analysis.md`
- `docs/evolutive/axt-test-evolution.md`
- `docs/commands/test.md`
- `docs/skills/axt-test/SKILL.md`
- Existing crate `crates/axt-test`
- `docs/agent-mode.md`
- `docs/error-catalog.md`
- `docs/agent-prompts.md` section 6

Goal:
Improve `axt-test` so it behaves like a compact test digest for coding agents:
stable failure IDs, failure-only output, rerun hints, and sharper parser tests.

Required end-to-end deliverables:
- Add the approved digest contract to the spec/addendum or command docs as
  appropriate.
- Add `--failures-only`.
- Add stable failure IDs to JSON, JSONL, and agent output.
- Add rerun hints in JSON and ACF output.
- Add `--rerun-id <ID>` if it can be implemented cleanly for at least the
  currently supported framework mappings. If not, document the exact limitation
  and implement stable hints only.
- Update `docs/commands/test.md`.
- Update `docs/skills/axt-test/SKILL.md`.
- Add or update man page `docs/man/axt-test.1`.
- Add fixtures and snapshot tests for failure-only output in all modes.
- Add parser coverage for Cargo, Go, Jest, Vitest, Pytest, Bun, and Deno where
  existing fixtures make that practical.
- Add truncation tests for large failure output.

Implementation constraints:
- Prefix shell commands with `rtk`.
- No `unwrap()` or `expect()` in non-test code.
- No network calls in binaries.
- Diagnostics go to stderr; data goes to stdout.
- Preserve existing `axt.test.v1` compatibility unless the spec explicitly
  requires a new version.
- Prefer additive fields over breaking output changes.
- Use typed errors with `thiserror`; `anyhow` only at the binary edge.
- Keep output deterministic and compact.
- Cross-platform behavior must be documented.

Scope discipline:
Do not create `axt-testdigest`. Do not add new test frameworks. Do not rewrite
the whole runner architecture unless necessary for the digest behavior.

Quality gates:
Run and fix:
- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`

Final response:
Use the status report template from `docs/agent-prompts.md` section 6. Include
files changed, tests run, any spec deviations, and next recommended milestone.
```
