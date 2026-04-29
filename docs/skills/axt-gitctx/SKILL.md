# axt-gitctx

Use `axt-gitctx` before summarizing local changes, preparing a commit message,
or deciding which changed files need deeper inspection.

## Rules

- Prefer `axt-gitctx --agent` for agent workflows and `--json` for scripts.
- Keep inline diffs bounded with `--inline-diff-max-bytes`.
- Use `--changed-only` when commit history is not needed.
- If a file is listed with `diff_inline: false`, inspect it with `axt-slice`,
  `axt-peek --changed`, or a direct file read.
- Do not use this skill for remote pull request metadata or hosting-provider
  state; `axt-gitctx` is local-only.

## Examples

```bash
axt-gitctx --agent
axt-gitctx --changed-only --inline-diff-max-bytes 4000 --agent
axt-gitctx --json
```
