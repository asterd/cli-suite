---
name: axt-outline
description: Use axt-outline to inspect source declarations, signatures, doc comments, visibility, and source ranges without reading full function bodies. Trigger before opening large supported source files when symbol-level context is enough.
license: MIT OR Apache-2.0
---

# axt-outline Skill

Use `axt-outline` when an agent needs compact source structure before reading full files.

## Rules

- Prefer `axt-outline --agent <PATH>` before reading large supported source files.
- Use `--public-only` when API surface is enough.
- Use `--lang <LANG>` when a directory contains multiple supported languages and only one matters.
- Use when a downstream tool needs only the payload.
- Use `axt-slice` for full bodies after selecting a symbol from the outline.
- Treat LSP-backed ranking and full grammar coverage as deferred scope.

## Examples

```bash
axt-outline crates/axt-outline/src --agent
axt-outline src/lib.rs --public-only --json
axt-outline src --lang typescript --agent
axt-outline . --agent --limit 100
```
