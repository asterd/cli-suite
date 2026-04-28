# axt-test Evolution Brief for Test Digest Behavior

Status: proposed extension to existing `axt-test`. This should not become a new
binary unless the spec is explicitly changed.

## Purpose

The original `testdigest` proposal is valid, but this repository already has
`axt-test`, which runs and normalizes test suites across Cargo, Go, Jest,
Vitest, Pytest, Bun, and Deno. The correct evolution is to improve `axt-test`
failure digest behavior.

## Market Position

Existing tools:

- Native runners provide framework-specific output.
- Cargo supports test execution and some JSON compiler diagnostics, while
  libtest JSON remains unstable in common workflows.
- `cargo-tes` exists specifically to compact Cargo test failures for agentic
  sessions, validating the need.
- `axt-test` already covers the multi-runner abstraction in this suite.

Market validity: medium as a separate command, high as `axt-test` improvement.

Coverage and impact: high. Compact test failures are one of the most common
agent needs.

Build decision: NO as a new binary. Extend `axt-test`.

## Recommended Scope

- Add `--failures-only` if current human/agent output is still too broad.
- Add `--rerun-id <ID>` or stable failure IDs for framework-specific reruns.
- Add `next` hints in JSON and ACF outputs:
  `axt-test --rerun-id <ID> --include-output --agent`.
- Improve parser tests for Cargo panic locations, Jest stack frames, Pytest
  assertion introspection, and Go JSON events.
- Ensure `command_failed` still represents failing tests, not command parser
  failure.

## Optional Alias

Avoid a standalone `testdigest` alias initially. If user testing proves the
term is valuable, add an opt-in alias that maps to:

```bash
axt-test --failures-only
```

Only add this after spec approval and installer updates.

## Output Additions

```json
{
  "failures": [
    {
      "id": "cargo:crates/axt-core:parse_config_rejects_empty",
      "framework": "cargo",
      "name": "parse_config_rejects_empty",
      "file": "crates/axt-core/src/lib.rs",
      "line": 42,
      "message": "assertion failed",
      "rerun": "axt-test --rerun-id cargo:crates/axt-core:parse_config_rejects_empty --agent"
    }
  ],
  "next": ["axt-test --rerun-id cargo:crates/axt-core:parse_config_rejects_empty --include-output --agent"]
}
```

## Tests

- Add fixture output for each supported runner.
- Add stable failure ID tests.
- Add rerun command rendering tests.
- Add snapshots for failure-only human, JSON, JSONL, and ACF modes.
- Add truncation tests for large stderr/stdout blocks.

## Skill Requirements

Update `docs/skills/axt-test/SKILL.md` after spec approval:

- Prefer `--failures-only --agent` when debugging a failing suite.
- Use `--rerun-id` instead of broad runner filters when provided.
- Use `--include-output` only for a focused failure.
