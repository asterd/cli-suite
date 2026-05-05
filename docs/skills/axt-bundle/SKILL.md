---
name: axt-bundle
description: Use axt-bundle for session warmup when an agent needs a compact repository overview with files, manifests, git state, and next-step hints in one command.
license: MIT OR Apache-2.0
---

# axt-bundle

Prefer:

```bash
axt-bundle --agent
```

Use it at the start of a repo task before deciding whether to inspect files with
`axt-outline`, search context with `axt-ctxpack`, or run tests with `axt-test`.

Use `--json` when a script needs the canonical envelope.

## Rules

- Prefer `axt-bundle . --agent` as the first repository-inspection command.
- Use `--depth` and `--max-files` to expand context deliberately.
- Add `--include-hidden` only when hidden project configuration matters.
- Treat `next` hints as suggestions, not mandatory follow-up commands.

## Examples

```bash
axt-bundle . --agent
axt-bundle . --depth 3 --max-files 80 --json
axt-bundle . --include-hidden --no-ignore --agent
```
