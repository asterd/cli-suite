---
name: axt-suite
description: Use the axt Foundation CLI Suite commands for compact, schema-versioned local repository warmup, command execution, environment, filesystem drift, port, test, source-outline, source-slice, context-pack, Git-context, and log-diagnosis inspection. Trigger when an agent needs stable low-token CLI output from a local workspace.
license: MIT OR Apache-2.0
---

# axt Suite Skill

Use this skill when you need local, offline, structured command output for agent workflows. Prefer canonical `axt-*` names in scripts and CI. Short aliases may exist only when installed with the `aliases` feature. There are no `ax-*` aliases.

## Agent Integration Pattern

This suite follows an RTK-inspired initialization model: install a lightweight skill into the agent's skill directory, then restart the agent. Unlike RTK, `axt` does not install shell rewrite hooks. Agents should call `axt-*` commands explicitly so command execution remains transparent and auditable.

Install this skill from a local checkout:

```bash
python3 scripts/agent/install-skills.py --agent both --scope project --skill axt-suite
```

Install every command-specific skill:

```bash
python3 scripts/agent/install-skills.py --agent both --scope project --skill all
```

## Global Rules

- Use `--agent` for low-token agent context.
- Use `--json` when you need a stable envelope with `schema`, `ok`, `data`, `warnings`, and `errors`.
- Agent mode is minified JSONL with a summary record first. Non-TTY stdout defaults to agent mode.
- Keep diagnostics and logs separate: axt commands write data to stdout and diagnostics to stderr.
- Do not assume network access. The suite is designed for offline local inspection.
- Inspect supported failures with `--list-errors`.
- Inspect output contracts with `--print-schema json` or `--print-schema agent`.

## Commands

### `axt-bundle`

Use at the start of a repository task to warm up context in one call.

```bash
axt-bundle . --agent
axt-bundle . --json
```

It returns a shallow file inventory, manifest previews, git state, and dynamic
next-step hints.

### `axt-peek`

Use for repository and directory snapshots.

```bash
axt-peek . --agent
axt-peek . --changed --json
axt-peek crates/axt-peek --depth 3 --lang rust --agent
```

Prefer it over combinations of `find`, `ls`, `du`, and `git status` when an agent needs one compact view.

### `axt-run`

Use to run commands and capture structured results.

```bash
axt-run --agent -- cargo test
axt-run --json --timeout 30s -- npm test
axt-run show last --stderr
```

Use `--no-save` for disposable runs. Use `show last` when a previous run already captured the stream tail.

### `axt-doc`

Use to diagnose local toolchain and environment state.

```bash
axt-doc which cargo --agent
axt-doc path --json
axt-doc env --agent
axt-doc all rustc --json
```

Environment values that look secret are redacted by default. Do not use `--show-secrets` unless the user explicitly needs local secret debugging.

### `axt-drift`

Use to detect filesystem changes from a mark or a command.

```bash
axt-drift mark --name before
axt-drift diff --since before --agent
axt-drift run --agent -- cargo build
```

Use `--hash` when metadata-only detection is not strong enough.

### `axt-port`

Use to inspect local port holders and, with explicit intent, free ports.

```bash
axt-port who 3000 --agent
axt-port list --proto both --json
axt-port free 3000 --dry-run --agent
```

Treat `free` as mutating and prefer `--dry-run` first. Never use it for remote hosts; the scope is local sockets only.

### `axt-test`

Use to run tests through a normalized schema.

```bash
axt-test --agent
axt-test --framework cargo --json
axt-test --changed --agent
axt-test list-frameworks --json
```

Supported frameworks: Jest, Vitest, Pytest, Cargo test, Go test, Bun test, and Deno test.

### `axt-outline`

Use to inspect source declarations, signatures, doc comments, visibility, and ranges without reading full function bodies.

```bash
axt-outline src/lib.rs --agent
axt-outline crates/axt-outline/src --public-only --json
axt-outline app --lang typescript --agent
```

Use it before opening large supported source files when symbol-level context is enough.

### `axt-slice`

Use to extract exact source for a selected symbol or enclosing line.

```bash
axt-slice src/lib.rs --symbol process_request --agent
axt-slice src/lib.rs --line 150 --json
```

Use it after `axt-outline` when you need the selected implementation body.

### `axt-ctxpack`

Use to search local files for multiple named patterns with compact snippets.

```bash
axt-ctxpack --pattern todo=TODO --pattern panic='unwrap\(|expect\(' crates --agent
axt-ctxpack --pattern route='app\\.route' src --json
```

Use it when `rg` output would be too large or when named hit groups help the agent.

### `axt-gitctx`

Use to inspect local Git branch, status, recent commits, and bounded diffs.

```bash
axt-gitctx . --agent
axt-gitctx --changed-only --json
```

It is read-only and never runs fetch, pull, push, or remote API calls.

### `axt-logdx`

Use to diagnose large local logs and command outputs.

```bash
axt-logdx target/test.log --severity error --top 20 --agent
cat build.log | axt-logdx --stdin --agent
```

Use returned fingerprints, snippets, line numbers, and timeline buckets to narrow follow-up reads.

## Installation Reference

Install all commands from a local checkout:

```bash
python3 scripts/install-local.py --command all
```

Install one command:

```bash
cargo install --path crates/axt-peek --locked
```

Install optional aliases:

```bash
cargo install --path crates/axt-peek --locked --features aliases
```

Canonical names are `axt-peek`, `axt-run`, `axt-doc`, `axt-drift`, `axt-port`, `axt-test`, `axt-outline`, `axt-slice`, `axt-ctxpack`, `axt-bundle`, `axt-gitctx`, and `axt-logdx`.
