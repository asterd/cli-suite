---
name: axt-peek
description: Use axt-peek for compact, schema-versioned directory, file metadata, language, and Git status snapshots in local workspaces. Trigger when an agent needs low-token replacement output for ls, find, du, tree, or git status style inspection.
license: MIT OR Apache-2.0
---

# axt-peek Skill

Use `axt-peek` when you need a local repository or directory snapshot with stable output.

## Rules

- Prefer `axt-peek . --agent` for compact agent context.
- Use `--json` when code must parse a stable envelope.
- Use `--agent` for record streams.
- Add `--changed` when only dirty or untracked files matter.
- Add `--no-git` when scanning huge trees where Git state is not needed.
- Use `--hash blake3` only when content identity matters; metadata mode is faster.

## Examples

```bash
axt-peek . --agent
axt-peek . --changed --json
axt-peek crates/axt-peek --depth 3 --lang rust --agent
axt-peek . --kind file --type code --agent
```

Inspect contracts with `axt-peek --print-schema agent` or `axt-peek --print-schema json`.
