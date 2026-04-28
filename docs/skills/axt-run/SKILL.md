---
name: axt-run
description: Use axt-run to execute local commands with structured exit status, duration, stdout/stderr tails, saved run logs, timeout handling, and file-change summaries. Trigger when an agent needs reliable low-token command execution output.
license: MIT OR Apache-2.0
---

# axt-run Skill

Use `axt-run` instead of raw shell execution when the result should be compact and machine-readable.

## Rules

- Prefer `axt-run --agent -- <command>` for agent context.
- Use `--json` when automation needs the full envelope.
- Use `--timeout <DURATION>` for potentially long commands.
- Use `--no-save` for disposable checks; omit it when later `show last` inspection is useful.
- Use `--no-watch-files` when file-change tracking is unnecessary.
- Remember that the child command may mutate local state.

## Examples

```bash
axt-run --agent -- cargo test
axt-run --json --timeout 30s -- npm test
axt-run --no-save --no-watch-files --agent -- cargo fmt --all --check
axt-run show last --stderr
axt-run clean --older-than 7d
```

Inspect contracts with `axt-run --print-schema agent` or `axt-run --print-schema json`.
