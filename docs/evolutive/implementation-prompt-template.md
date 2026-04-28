# Evolutive Command Implementation Prompt Template

Use this prompt after a command has been approved in `docs/spec.md` or
`docs/spec-addendum.md`. Do not use it to bypass the current project rule that
new binaries require explicit spec approval.

Replace placeholders before starting:

- `{COMMAND}`: full binary name, for example `axt-outline`
- `{ALIAS}`: optional unprefixed alias, for example `outline`
- `{CRATE}`: crate name, for example `axt-outline`
- `{BRIEF}`: evolutive brief path, for example `docs/evolutive/axt-outline.md`
- `{SPEC_SECTION}`: approved spec section for this command
- `{MILESTONE}`: command milestone label, for example `E1-outline-mvp`
- `{SCOPE}`: exact implementation scope for this session

## Implementation Prompt

```text
Continue the `axt` Foundation CLI Suite with evolutive milestone `{MILESTONE}`.

Target command: `{COMMAND}`.
Optional alias: `{ALIAS}`.
Crate: `crates/{CRATE}`.
Approved spec section: `{SPEC_SECTION}`.
Design brief: `{BRIEF}`.

You are implementing only `{SCOPE}` in this session. Stop at this milestone
boundary. Do not start later phases, deferred scope, or adjacent commands.

Project rules are binding:
- Read `AGENTS.md` / `CLAUDE.md` rules already loaded in context.
- Read `/Users/ddurzo/.codex/RTK.md` and prefix shell commands with `rtk`.
- Source of truth is the approved spec section plus the command brief.
- Do not modify `docs/spec.md` or `docs/spec-addendum.md` unless I explicitly
  ask for a spec-change session.
- No `unwrap()` or `expect()` in non-test code.
- No network calls in binaries.
- Diagnostics on stderr; data on stdout.
- Support all standard output modes: human, `--plain`, `--json`, `--json-data`,
  `--jsonl`, `--agent`, `--print-schema`, and `--list-errors`.
- Keep schemas versioned under `axt.<command>.v1`.
- Keep the optional unprefixed alias behind the `aliases` feature.
- Use typed errors with `thiserror` in libraries. `anyhow` is allowed only at
  the binary edge.
- Prefer existing suite patterns over new abstractions.
- Preserve cross-platform behavior. If a feature cannot work on Linux, macOS,
  or Windows, document it and return `feature_unsupported` rather than silently
  degrading.

Before writing code:
1. Read only the relevant approved spec section `{SPEC_SECTION}`.
2. Read `{BRIEF}`.
3. Read the closest existing command implementation and docs:
   - `docs/commands/test.md` or `docs/commands/peek.md`
   - one similar `crates/axt-*/src/cli.rs`
   - one similar `crates/axt-*/src/render.rs`
   - one similar `crates/axt-*/tests/modes.rs`
   - one existing skill under `docs/skills/`
4. Inspect workspace `Cargo.toml` and existing crate `Cargo.toml` patterns.
5. Reply with a concise 5-bullet implementation plan:
   - Deliverables for this milestone.
   - Files or crates you expect to create or modify.
   - Output schemas and agent-mode records you will add.
   - Tests you will add.
   - Any ambiguity or risk, with your proposed interpretation.

Wait for my confirmation before editing files.

After I confirm:
1. Implement only `{SCOPE}`.
2. Add or update:
   - `crates/{CRATE}/`
   - workspace membership and package metadata
   - `docs/commands/{COMMAND_WITHOUT_PREFIX}.md`
   - `docs/man/{COMMAND}.1`
   - `docs/skills/{COMMAND}/SKILL.md`
   - `scripts/agent/install-skills.py`
   - tests and fixtures needed for this scope
3. Keep generated output compact and deterministic.
4. Add snapshot tests for human, JSON, JSONL, and agent output.
5. Add unit tests for parsing, truncation, errors, and cross-platform behavior.
6. Run quality gates:
   - `cargo fmt --all --check`
   - `cargo clippy --workspace --all-targets -- -D warnings`
   - `cargo test --workspace`
7. If a gate fails, fix only issues caused by this milestone and rerun it.

When done, produce a status report using the template in
`docs/agent-prompts.md` section 6. Use `{MILESTONE}` as the status label and
include:
- What was implemented.
- What was intentionally deferred.
- Deviations from spec, if any.
- Exact quality-gate results.
- Open questions.
- Next recommended evolutive milestone.
```

## Spec-Approval Prompt

Use this before implementation when the command is still only an evolutive
proposal.

```text
Prepare a spec-change proposal for `{COMMAND}`.

Do not implement code. Do not create crates.

Read:
- `docs/evolutive/market-analysis.md`
- `{BRIEF}`
- `docs/spec.md` sections covering shared output modes, errors, workspace
  structure, release rules, and existing command contracts
- `docs/spec-addendum.md` sections covering current later commands

Output a proposed spec patch plan, not the patch itself:
- Where the command should be added.
- Binary and alias naming.
- MVP behavior.
- Explicit deferred scope.
- Output schemas.
- Error-code mapping.
- Cross-platform matrix.
- Tests and docs required.
- Any conflict with current hard rules.

Wait for my approval before editing any spec file.
```

## Review Prompt

Use this after an implementation session and before starting the next command
or next phase.

```text
Review evolutive milestone `{MILESTONE}` for `{COMMAND}`.

Do not write new features.

Tasks:
1. Read `{SPEC_SECTION}` and `{BRIEF}`.
2. Read the status report from the implementation session.
3. Run:
   - `cargo fmt --all --check`
   - `cargo clippy --workspace --all-targets -- -D warnings`
   - `cargo test --workspace`
4. Inspect:
   - `crates/{CRATE}/`
   - `docs/commands/{COMMAND_WITHOUT_PREFIX}.md`
   - `docs/man/{COMMAND}.1`
   - `docs/skills/{COMMAND}/SKILL.md`
   - `scripts/agent/install-skills.py`
5. For each done criterion in the approved spec and brief, mark `done`,
   `partial`, or `missing` with one-line evidence.

Output:
- Findings first, ordered by severity, with file and line references.
- Done-criteria checklist.
- Quality-gate results.
- Go / no-go recommendation for the next evolutive milestone.
```

## Recovery Prompt

Use this when tests, CI, packaging, or behavior fails after a command milestone.

```text
Recover `{COMMAND}` milestone `{MILESTONE}`.

Do not add new features. Do not refactor unrelated code.

Problem:
{FAILURE_SUMMARY}

Tasks:
1. Reproduce the failure with the smallest command.
2. Identify the smallest fix.
3. Reply before editing with:
   - Error in one sentence.
   - Likely cause with file and line if known.
   - Proposed fix.
   - Tests to rerun.

Wait for my approval before changing files.

After approval:
1. Apply the smallest fix.
2. Rerun the failing command.
3. Rerun full quality gates if the fix touches shared code or output schemas.
4. Produce a short recovery status report.
```

## Recommended Command Milestone Splits

Use small sessions. Do not implement a whole complex command in one pass.

| Command | First milestone | Later milestones |
|---|---|---|
| `axt-outline` | Rust-only file outline plus all output modes | Directories, TS/JS, Python, Go, Java |
| `axt-ctxpack` | Regex multi-pattern search with snippets | AST classification, language-specific kinds |
| `axt-slice` | Rust symbol extraction from one file | Imports, line fallback, more languages |
| `axt-gitctx` | Status, branch, recent commits, diff stats | Inline small diffs, rename details, submodules |
| `axt-plan` | Dry-run literal/regex plans | Structural matching, apply mode |
| `axt-logsift` | Plain text and JSONL dedup/top errors | Stack traces, time windows, format presets |
| `axt-manifest` | Cargo and package.json normalization | Pyproject, Go, Dockerfile, CI YAML |
| `axt-repomap` | Topology summary from existing primitives | Symbol integration, ranking, larger monorepos |
| `axt-impact` | Rust research prototype with fallback | LSP process management, more languages |

`axt-test` digest behavior should be handled as an `axt-test` evolution, not as
a new command milestone.
