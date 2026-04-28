# Agent Skills

`docs/skills/axt-suite/SKILL.md` is an installable agent skill for Codex, Claude Code, and similar tools.

The skill keeps the command surface compact and points agents toward the lowest-token output modes:

- `--agent` for ACF.
- `--json` for stable envelopes.
- `--jsonl` for streaming records.

Copy `docs/skills/axt-suite/` into the target agent's skill directory when packaging the suite for agent environments.
