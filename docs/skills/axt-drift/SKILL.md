---
name: axt-drift
description: Use axt-drift to mark local filesystem state and report created, modified, and deleted files after builds, generators, or commands. Trigger when an agent needs compact filesystem drift detection.
license: MIT OR Apache-2.0
---

# axt-drift Skill

Use `axt-drift` to compare filesystem state before and after a task.

## Rules

- Use `mark` before a risky or noisy operation.
- Use `diff --since <name>` after the operation.
- Use `run -- <command>` when the command and drift should be captured together.
- Add `--hash` when metadata-only comparison is not strong enough.
- Marks are local state under `.axt/drift`; use `reset` only when old marks are no longer needed.

## Examples

```bash
axt-drift mark --name before
axt-drift diff --since before --agent
axt-drift run --agent -- cargo build
axt-drift list --json
axt-drift reset
```

Inspect contracts with `axt-drift --print-schema agent` or `axt-drift --print-schema json`.
