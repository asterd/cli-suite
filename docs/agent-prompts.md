# `ax` Coding-Agent Prompt Playbook

How to drive Claude Code, Codex, Aider, or any coding agent through the implementation of `ax` without losing the plot.

This file is meant to live at `docs/agent-prompts.md` in the repo, alongside `spec.md` and `spec-addendum.md`. The prompts below are **copy-paste ready**.

---

## 1. The mental model

You are not writing one big prompt. You are running **a sequence of small, bounded sessions**, each scoped to one milestone. Between sessions you read what was produced, decide if it's good, and either move on or do recovery.

Three layers of instructions, in order of permanence:

| Layer | Lives in | Read when |
|---|---|---|
| Permanent rules | `CLAUDE.md` (or `AGENTS.md`) at repo root | Auto-loaded by the agent every session |
| Spec | `docs/spec.md` + `docs/spec-addendum.md` | Loaded by the agent when the prompt tells it to |
| Session prompts | This file | Pasted by you at the start of each session |

This separation matters. Permanent rules in `CLAUDE.md` cost zero tokens to repeat. The spec is read on demand. Session prompts are short because they reference, not duplicate.

---

## 2. The `CLAUDE.md` to commit at repo root

Create this file once, commit it. Claude Code auto-loads it. For Codex, rename to `AGENTS.md`. For other agents, paste its content into the system prompt.

```markdown
# `ax` — Project Rules for AI Agents

You are implementing the `ax` Foundation CLI Suite. Source of truth: `docs/spec.md` and `docs/spec-addendum.md`.

## Hard rules (never violate)

1. **Stop at milestone boundaries.** Each session has a single target milestone. Do not start the next one without explicit instruction.
2. **No `unwrap()` or `expect()` in non-test code.** Use typed errors via `thiserror` in libraries; `anyhow` is allowed only at the binary edge (`main.rs`).
3. **No deviation from the spec without updating the spec first.** If you find an ambiguity or a real reason to change behavior, edit the relevant spec section, explain why in the commit message, then implement. Never silent-drift.
4. **No new commands or binaries beyond the six in the spec** (`ax-peek`, `ax-run`, `ax-doc`, `ax-drift`, `ax-port`, `ax-test`).
5. **No network calls in the binaries.** Ever. The string `reqwest` and friends should not appear in `crates/ax-*/Cargo.toml`.
6. **No telemetry, no analytics, no postinstall scripts that fetch anything.**
7. **Diagnostics on stderr, data on stdout.** Always.
8. **Four primary output modes always, even for stub commands**: `--json`, `--jsonl`, `--agent`, human (default). `--plain`, `--json-data`, `--print-schema`, and `--list-errors` are also standard shared flags.
9. **Cross-platform parity is the default.** When a feature degrades on Windows or macOS, document it in the per-command cross-platform matrix (`docs/commands/<cmd>.md`) and exit with code 9 (`feature_unsupported`) rather than fail silently.
10. **Conventional commits.** Format: `<type>(<scope>): <subject>` where type ∈ {feat, fix, chore, docs, test, refactor, perf, build, ci} and scope is the crate name (e.g., `ax-peek`, `ax-core`).

## Quality gates (run before declaring a milestone done)

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

CI must pass on Linux, macOS, and Windows. If a test depends on platform-specific behavior, gate it with `#[cfg(target_os = "...")]` rather than skipping.

## Design defaults (when the spec is silent)

- Prefer simple over clever. Two functions are clearer than one generic.
- Prefer `camino::Utf8PathBuf` over `std::path::PathBuf` for path handling.
- Prefer `clap` derive macros over the builder API.
- Prefer typed enum errors (`#[derive(thiserror::Error)]`) over string errors.
- Prefer `serde` with `#[derive]` over hand-rolled JSON.
- Prefer snapshot tests (`insta`) for output assertions.
- Match ACF agent-mode key names and prefixes to the dictionary in `docs/agent-mode.md`. Add new keys only when no existing one fits, and document them.

## Files you may freely create

- New crates under `crates/`, but only `ax-*` per the spec.
- New tests anywhere appropriate.
- New docs under `docs/`.
- New fixtures under `fixtures/`.

## Files you may not modify without explicit permission

- `docs/spec.md` and `docs/spec-addendum.md` (these are the contract; edits go through human review).
- `CLAUDE.md` itself.
- `LICENSE-MIT` and `LICENSE-APACHE`.
- `.github/workflows/release.yml` once `cargo dist` has generated it (regenerate with `cargo dist generate-ci` if changes are needed).

## Communication

- Reply in the language the user uses (most often Italian for high-level discussion).
- Code, identifiers, comments, commit messages, and documentation are always in **English**.
- When you finish a milestone, your final message must follow the **status report template** in `docs/agent-prompts.md`.

## Token discipline

Long contexts hurt quality. To keep sessions tight:
- Read only the spec sections relevant to the current milestone. The spec's table of contents lets you target.
- When debugging, read individual files with `view` and a line range, not whole files.
- When tests pass, do not re-print the full output; summarize.
- When implementing a milestone with multiple sub-tasks, prefer subagents (Claude Code) or sequential focused sessions (other agents) over one mega-context.
```

Commit this as `CLAUDE.md` at the repo root. Done once.

---

## 3. The kickoff prompt (Milestone 0, run once)

Use this **exactly once**, at project start, after you've created the empty repo with `CLAUDE.md`, `docs/spec.md`, `docs/spec-addendum.md`, and nothing else.

```
You are starting the `ax` Foundation CLI Suite.

Your task this session is **Milestone 0 only**, defined in `docs/spec.md` section 14, "Milestone 0 — workspace scaffolding".

Before writing any code:
1. Read `CLAUDE.md` in full.
2. Read `docs/spec.md` sections 0–7 and section 14.
3. Read `docs/spec-addendum.md` sections "Updated TL;DR", "Updated binary table", and "Updated implementation plan".
4. Reply with a 5-bullet confirmation:
   - The 6 binaries in the suite, in implementation order.
   - The 4 internal library crates.
   - The output modes every binary must support.
   - The exit-code convention.
   - The hard rules from CLAUDE.md you find most important to remember.

Wait for me to confirm before proceeding.

When I confirm, implement Milestone 0:
- Create the repository structure from spec.md section 6.
- Workspace `Cargo.toml` from section 7.
- Empty crate stubs for `ax-core`, `ax-output`, `ax-fs`, `ax-git`, `ax-peek`, plus `xtask`.
- License files, README, CONTRIBUTING, SECURITY.
- A green CI workflow on Linux/macOS/Windows running fmt, clippy, test on stable Rust.
- `cargo dist init --ci=github --installer shell --installer powershell --installer homebrew`, configured for tier-1 targets only.

Do not implement anything for Milestone 1 yet. Do not write `ax-peek` logic. Stub crates should compile but not do real work.

When done, produce the status report described in `docs/agent-prompts.md` section 6.
```

The two-step structure (read-confirm-then-implement) is intentional. It catches misunderstandings before any code is written.

---

## 4. The continuation prompt template (Milestones 1–6)

Replace `{N}` and `{TARGET}` per the table below. Everything else stays the same.

```
Continue with **Milestone {N}** from the spec.

Target: {TARGET}.

Before writing any code:
1. Read the relevant spec section (see table below in this prompt).
2. Read the previous milestone's status report (in this conversation history or in `docs/status/M{N-1}.md` if checkpointed).
3. Reply with a 3-bullet plan:
   - The deliverables you'll produce.
   - The order you'll produce them in.
   - Any ambiguity you spotted in the spec, with your proposed interpretation.

Wait for me to confirm the plan before writing code.

When I confirm:
- Implement only this milestone. Stop at its boundary.
- Preserve all existing public schemas (`ax.*.v1`). Schema changes require a major version bump and explicit instruction.
- Add tests before or alongside production code.
- Run all three quality gates before declaring done.
- Produce the status report.

Spec sections to read for this milestone:
- {SPEC_SECTIONS}
```

### Per-milestone reference table

| N | Target | Spec sections to read | Approx duration |
|---|---|---|---|
| 1 | `ax-core` and `ax-output` foundations | `spec.md` §3, §5, §8.1, §8.2, §13, §14 (M1) | 3–5 days |
| 2 | `ax-fs` and `ax-git` shared crates | `spec.md` §8.3, §8.4, §14 (M2) | 5–7 days |
| 3 | `ax-peek` MVP, end-to-end | `spec.md` §9 (full), §14 (M3) | 5–7 days |
| 4 | Release pipeline shakedown | `spec.md` §12 (full), §14 (M4) | 1–3 days |
| 5 | `ax-run` MVP | `spec.md` §10 (full) | 5–10 days |
| 6 | `ax-doc` MVP | `spec.md` §11.1 | 5–7 days |
| 7 | `ax-drift` MVP | `spec.md` §11.2 | 3–5 days |
| 8 | `ax-port` MVP | `spec-addendum.md` §11.3 (full) | 5–7 days |
| 9 | `ax-test` MVP | `spec-addendum.md` §11.4 (full) | 10–14 days |

(Milestones 0 + 1–9 = 10 sessions total. Some milestones split into sub-sessions for the bigger ones; see section 7.)

---

## 5. The review prompt (between milestones)

Run this **between** milestones, in a fresh session, before launching the next implementation prompt. The fresh context catches things the implementing agent missed because it was too close to the code.

```
You are reviewing what was just shipped, not writing new code.

Your task:
1. Read the spec section for **Milestone {N}** (see the table in `docs/agent-prompts.md` section 4).
2. Read the most recent status report.
3. Run, in order:
   - `cargo fmt --all --check`
   - `cargo clippy --workspace --all-targets -- -D warnings`
   - `cargo test --workspace`
4. Inspect the new crate(s) and `docs/commands/<cmd>.md` if applicable.
5. For each item in the milestone's "Done criteria" in the spec, mark it `done`, `partial`, or `missing` with one-line evidence.

Output:
- A checklist with the verdict per criterion.
- A list of issues found, ordered by severity.
- A go / no-go recommendation for the next milestone.

Do not write code in this session. If issues need fixing, I will start a recovery session.
```

This is cheap (no implementation) and catches drift that the implementing context normalizes.

---

## 6. The status-report template

Every implementation session ends with this format. The agent produces it; you read it; you decide go / no-go for the next session.

```markdown
# Milestone {N} Status Report

## What was implemented
- (3–10 bullets, concrete: "added X to crate Y", not "improved error handling")

## What was not implemented and why
- (every item in the spec's Done criteria that is not yet done, with reason)

## Deviations from the spec
- (every behavioral choice that departed from the spec, with justification — these should be rare; if there are any, the spec should also have been updated)

## How to run tests
```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## How to run the release pipeline locally
- (commands that demonstrate the pipeline works, even if just a dry-run)

## Quality gates: pass / fail
- fmt: ✅ / ❌
- clippy: ✅ / ❌
- test: ✅ / ❌
- CI on Linux/macOS/Windows: ✅ / ❌

## Open questions for the maintainer
- (anything you encountered that needs human judgment before the next milestone)

## Next recommended milestone
- M{N+1}: {TARGET}
```

Commit this as `docs/status/M{N}.md` so it's discoverable in future sessions.

---

## 7. Splitting bigger milestones into sub-sessions

Milestones 1, 5, 6, and 9 are large. Don't run them as one session. Split:

### Milestone 1 — split into 1a, 1b
- **1a**: `ax-core` only. Errors, exit codes, `Clock`, `OutputMode`, common flags, `--print-schema`/`--list-errors` machinery. Tests for everything.
- **1b**: `ax-output` only. `JsonEnvelope`, `JsonlWriter`, `AgentCompactWriter`, color/TTY handling, truncation. Snapshot tests.

### Milestone 5 — split into 5a, 5b
- **5a**: `ax-run` core: spawn, capture, exit code, timeout. JSON envelope. No file-watching yet.
- **5b**: `ax-run` extras: `--watch-files`, artifact storage (`.ax/runs/`), `runx show/list/clean`, agent ACF plus JSONL.

### Milestone 6 — split into 6a, 6b, 6c
- **6a**: `ax-doc which <cmd>` only.
- **6b**: `ax-doc path` (PATH duplicates, missing dirs, broken symlinks).
- **6c**: `ax-doc env` (var listing, secret-like detection) + `ax-doc all`.

### Milestone 9 — split into 9a, 9b, 9c, 9d
- **9a**: `ax-test` framework, `TestFrontend` trait, jest frontend only.
- **9b**: pytest frontend.
- **9c**: cargo and go test frontends.
- **9d**: vitest, bun, deno frontends + monorepo multi-framework merging.

Each sub-session is its own continuation prompt. Same review pattern between them.

---

## 8. The recovery prompt (when something breaks)

When tests fail, CI is red, or a release candidate doesn't install cleanly:

```
Tests / CI / install are failing. Do not write new features. Do not refactor.

Your task:
1. Run the failing command and capture exact output.
2. Identify the smallest set of changes that would make it pass.
3. Reply with:
   - The error in one sentence.
   - The likely cause (file + line if known).
   - The fix you propose.
4. Wait for me to approve before changing anything.

When I approve:
- Apply the fix.
- Re-run the failing command to confirm green.
- Re-run the full quality gates.
- Commit with `fix(<scope>): <subject>`.

Do not expand scope. If you discover a related issue, note it but do not fix it in the same commit.
```

The discipline of "diagnose → propose → wait → fix" prevents agents from cascading changes that introduce new problems.

---

## 9. The "stuck" prompt (when an agent loops)

Agents sometimes loop on hard problems, burning tokens without progress. If you see retries with no change in approach, kill the session and start a new one with:

```
Previous session was stuck on: {DESCRIBE PROBLEM IN ONE LINE}.

Do not retry the previous approach. Instead:
1. Read the relevant spec section: {SECTION}.
2. Identify three possible approaches to the problem.
3. For each, list (a) one reason to use it, (b) one risk.
4. Recommend one.
5. Wait for me to confirm before implementing.
```

Forcing the agent to enumerate alternatives breaks the loop pattern.

---

## 10. The release prompt (end of a phase)

Run this at the end of Milestones 4, 5, 7, 8, and 9 (i.e., when you have something publishable).

```
Prepare release v{VERSION}.

Steps:
1. Confirm `cargo test --workspace` is green.
2. Update `CHANGELOG.md` in Keep-a-Changelog format. Group changes by binary.
3. Bump version in workspace `Cargo.toml` and per-crate `Cargo.toml` files. All binary crates use the same version. Internal library crates may bump independently.
4. Run `cargo dist plan` and confirm the build matrix matches the targets in `spec.md` §12.2.
5. Commit with message `chore(release): v{VERSION}`.
6. Tag: `git tag v{VERSION} -s -m "ax v{VERSION}"`.
7. Do not push the tag yet. Output the exact commands I should run, including the smoke-test plan from `spec.md` §12.4 step 7.

After I push and the workflow runs, I'll start a new session with the release-verification prompt.
```

Then verification:

```
Verify release v{VERSION}.

The release workflow has finished. Run:
1. The shell installer on a clean Linux fixture (use a Docker container or a fresh VM).
2. `brew install <org>/ax/ax-peek` on macOS (if local).
3. `scoop install ax-peek` on Windows (if local).
4. `cargo install ax-peek` from a fresh `cargo` install.

For each: confirm the binary runs, `--version` reports the expected version, and `--help` works.

Output a table: channel × OS × pass/fail.

If anything fails, do not yank yet. Diagnose first; recovery prompt next.
```

---

## 11. What the maintainer does between sessions

You are the loop closer. After every session:

1. **Read the status report.** If the milestone's done-criteria are met, move on. If not, decide whether to extend the same milestone or split.
2. **Skim the diff.** Look for: new dependencies added, scope creep, subtle deviations from the spec.
3. **Run the review prompt** (section 5) in a fresh session if you want a second opinion.
4. **Commit and push the work** if you haven't already, so CI gates it.
5. **Pick the next session's prompt** from this file. Paste, run.

Most agent sessions in a project like this are 30–90 minutes of human attention each, plus background CI time. Plan accordingly.

---

## 12. Anti-patterns — things not to do

- **Do not paste the entire spec into the prompt.** It's already in the repo. The prompt should reference, not duplicate.
- **Do not say "implement everything in the spec".** Always milestone-bounded. Always.
- **Do not skip the review step** between milestones. The marginal cost is low, the catch rate is high.
- **Do not mix new features with refactoring in one session.** Two sessions, two commits.
- **Do not let the agent edit `spec.md` or `spec-addendum.md` without you reading the diff first.** The spec is the contract.
- **Do not run a continuation prompt with stale context** from a previous milestone. New milestone, new session.
- **Do not approve plans that say "I'll figure it out as I go".** Make the agent commit to a concrete plan first.
- **Do not let "all green" mean "done".** Green tests means the existing tests pass; it doesn't mean the milestone's done-criteria are met. Check the criteria explicitly.

---

## 13. Quick reference card (print this)

```
START               → kickoff prompt           (§3)  — Milestone 0
EACH MILESTONE      → continuation prompt      (§4)  — read, plan, confirm, implement
BETWEEN MILESTONES  → review prompt            (§5)  — fresh session, no code
END OF IMPL         → status report            (§6)  — committed under docs/status/
WHEN BROKEN         → recovery prompt          (§8)  — diagnose, approve, fix, commit
WHEN LOOPING        → stuck prompt             (§9)  — list alternatives, pick one
END OF PHASE        → release prompt           (§10) — version, tag, smoke-test
```

That's the whole machine.
