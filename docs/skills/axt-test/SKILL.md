---
name: axt-test
description: Use axt-test to detect and run local test suites with normalized, schema-versioned output across Jest, Vitest, Pytest, Cargo, Go, Bun, and Deno. Trigger when an agent needs compact test results or changed-file test selection.
license: MIT OR Apache-2.0
---

# axt-test Skill

Use `axt-test` when test output should be compact and normalized across frameworks.

## Rules

- Prefer `axt-test --agent` for agent context.
- Use `--framework <NAME>` when detection would be ambiguous.
- Use `--changed` when only tests mapped from changed files should run.
- Use `--json list-frameworks` to inspect supported frontends.
- Missing local test tools should be treated as `feature_unsupported`, not as network-install prompts.

## Examples

```bash
axt-test --agent
axt-test --framework cargo --json
axt-test --changed --agent
axt-test --json list-frameworks
```

Supported frameworks: `jest`, `vitest`, `pytest`, `cargo`, `go`, `bun`, and `deno`.
