# Agent Skills

The `docs/skills/` tree contains installable Agent Skills for Codex, Claude Code,
and compatible tools.

| Skill | Scope |
|---|---|
| `axt-suite` | Whole command suite. |
| `axt-peek` | Repository and filesystem snapshots. |
| `axt-run` | Structured command execution. |
| `axt-doc` | PATH, command, and environment diagnostics. |
| `axt-drift` | Filesystem drift marks and diffs. |
| `axt-port` | Local port inspection and cleanup. |
| `axt-test` | Normalized test runner output. |
| `axt-outline` | Source symbol outlines. |
| `axt-ctxpack` | Search/read context packs. |
| `axt-bundle` | Session warmup bundle. |

## Install

Install the suite skill into the current project for both Codex and Claude Code:

```bash
python3 scripts/agent/install-skills.py --agent both --scope project --skill axt-suite
```

Install every skill globally:

```bash
python3 scripts/agent/install-skills.py --agent both --scope user --skill all
```

Install one command skill for Codex only:

```bash
python3 scripts/agent/install-skills.py --agent codex --scope project --skill axt-run
```

Use `--dry-run` to inspect the target paths and `--force` to replace an existing
copy.

## Target Paths

| Agent | Project scope | User scope |
|---|---|---|
| Codex | `.codex/skills/<skill>` | `~/.codex/skills/<skill>` |
| Claude Code | `.claude/skills/<skill>` | `~/.claude/skills/<skill>` |

Restart the agent after installation so the new skill metadata is loaded.
