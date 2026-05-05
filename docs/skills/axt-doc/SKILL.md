---
name: axt-doc
description: Use axt-doc to diagnose local PATH, command resolution, environment, duplicate path entries, missing paths, and secret-redacted environment state. Trigger when an agent needs offline toolchain or shell environment diagnostics.
license: MIT OR Apache-2.0
---

# axt-doc Skill

Use `axt-doc` for local environment and toolchain diagnostics without network calls.

## Rules

- Prefer `axt-doc --agent all <command>` for compact command and environment context.
- Use `which` to resolve one executable.
- Use `path` to inspect missing or duplicate PATH entries.
- Use `env` to inspect redacted environment variables.
- Do not use `--show-secrets` unless the user explicitly asks for local secret debugging.

## Examples

```bash
axt-doc --agent which cargo
axt-doc --json path
axt-doc --agent env
axt-doc --json all rustc
```

Inspect contracts with `axt-doc --print-schema agent` or `axt-doc --print-schema json`.
