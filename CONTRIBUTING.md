# Contributing

Thanks for helping improve `axt`. This project values small, reviewable changes
that keep the command suite fast, portable, predictable, and safe to run on a
local machine.

## Product Principles

- Keep every command focused. Each tool should remain a single-purpose binary
  with a clear local workflow.
- Preserve cross-platform behavior across Linux, macOS, and Windows. If a
  feature cannot work on one platform, document it and return a clear
  unsupported error instead of silently degrading.
- Keep commands ultra-fast for common repositories. Prefer bounded traversal,
  compact output, and predictable resource use.
- Keep defaults super-safe. Commands should avoid surprising writes, process
  termination, secret exposure, or destructive behavior.
- Do not add telemetry, analytics, background reporting, postinstall fetches, or
  network calls from binaries.
- Use canonical `axt-*` command names in scripts, CI, docs, and examples. Short
  aliases are optional convenience names only.

## Before You Start

1. Check existing issues or discussions to avoid duplicate work.
2. Keep the change scoped to one behavior, command, or documentation topic.
3. Prefer the existing crate structure, CLI patterns, output modes, and error
   conventions.
4. Update user-facing docs when behavior or flags change.

Internal agent instructions and implementation specs are not contribution
guidelines. Contributors should use this file, the README, command docs, and the
existing code as the public development contract.

## Code Guidelines

- Use typed errors in library code.
- Avoid panics in normal command paths.
- Keep diagnostics on stderr and data on stdout.
- Support the shared output modes where applicable: human, `--plain`, `--json`,
  `--json-data`, `--jsonl`, and `--agent`.
- Keep output schemas stable and versioned.
- Add focused tests for behavior changes and regression fixes.
- Gate platform-specific tests with `#[cfg(...)]` instead of relying on manual
  skips.

## Documentation Guidelines

- Keep examples copy-pasteable.
- Prefer canonical `axt-*` names.
- Document safety implications for commands that run child commands, write local
  state, or signal processes.
- Keep command pages aligned with CLI help, schemas, and error output.

## Quality Checks

Run these before opening a pull request:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

When a change touches platform-specific behavior, also run the relevant command
manually on the affected platform if you have access to it.

## Commit Messages

Use conventional commits:

```text
<type>(<scope>): <subject>
```

Allowed types are `feat`, `fix`, `chore`, `docs`, `test`, `refactor`, `perf`,
`build`, and `ci`. Use the crate or document area as the scope, for example
`fix(axt-port): refuse to free the current process`.
