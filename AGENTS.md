# `axt` — Project Rules for AI Agents

You are implementing the `axt` Foundation CLI Suite. Source of truth: `docs/spec.md` and `docs/spec-addendum.md`.

## Hard rules (never violate)

1. **Stop at milestone boundaries.** Each session has a single target milestone. Do not start the next one without explicit instruction.
2. **No `unwrap()` or `expect()` in non-test code.** Use typed errors via `thiserror` in libraries; `anyhow` is allowed only at the binary edge (`main.rs`).
3. **No deviation from the spec without updating the spec first.** If you find an ambiguity or a real reason to change behavior, edit the relevant spec section, explain why in the commit message, then implement. Never silent-drift.
4. **No new commands or binaries beyond the six in the spec** (`axt-peek`, `axt-run`, `axt-doc`, `axt-drift`, `axt-port`, `axt-test`).
5. **No network calls in the binaries.** Ever. The string `reqwest` and friends should not appear in `crates/axt-*/Cargo.toml`.
6. **No telemetry, no analytics, no postinstall scripts that fetch anything.**
7. **Diagnostics on stderr, data on stdout.** Always.
8. **Four primary output modes always, even for stub commands**: `--json`, `--jsonl`, `--agent`, human (default). `--plain`, `--json-data`, `--print-schema`, and `--list-errors` are also standard shared flags.
9. **Cross-platform parity is the default.** When a feature degrades on Windows or macOS, document it in the per-command cross-platform matrix (`docs/commands/<cmd>.md`) and exit with code 9 (`feature_unsupported`) rather than fail silently.
10. **Conventional commits.** Format: `<type>(<scope>): <subject>` where type ∈ {feat, fix, chore, docs, test, refactor, perf, build, ci} and scope is the crate name (e.g., `axt-peek`, `axt-core`).

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

- New crates under `crates/`, but only `axt-*` per the spec.
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
