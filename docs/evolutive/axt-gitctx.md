# axt-gitctx Evolution Brief

Status: proposed. Requires spec approval before implementation.

## Purpose

`axt-gitctx` returns the local git state an agent usually needs in one bounded
call: branch, upstream, ahead/behind, changed files, diff stats, recent commits,
and small inline diffs.

## Market Position

Existing tools:

- Git exposes all raw data, but usually through several separate commands.
- GitHub CLI is excellent for GitHub hosting state, not provider-neutral local
  worktree context.
- Delta is a strong diff pager for humans, but not a compact schema source for
  agents.
- Onefetch summarizes repositories for humans, not active change review.

Market validity: medium-high.

Coverage and impact: high. It replaces repeated `git status`, `git branch`,
`git log`, `git diff --stat`, and selective `git diff` reads.

Build decision: YES.

## Naming

- Binary: `axt-gitctx`
- Optional alias: `gitctx`
- Crate: `crates/axt-gitctx`
- Schema prefix: `axt.gitctx.v1`

Verify package-name availability again before publish.

## MVP Scope

- Detect current repository.
- Return branch, upstream, ahead, behind, dirty state.
- Return changed files with status, additions, deletions, hunk count, and size.
- Include recent commits with hash, subject, author, and relative/absolute time.
- Include inline diffs only for files under `--inline-diff-max-bytes`.
- Never invoke network commands.

## Deferred Scope

- Pull request metadata.
- Remote hosting APIs.
- Interactive diff viewing.
- Commit creation.

## CLI Sketch

```bash
axt-gitctx --json
axt-gitctx --agent --commits 5 --inline-diff-max-bytes 12000
axt-gitctx --changed-only --jsonl
```

## Output Requirements

```json
{
  "repo": ".",
  "branch": {"name": "main", "upstream": "origin/main", "ahead": 1, "behind": 0},
  "summary": {"changed": 3, "added": 10, "deleted": 4, "truncated": false},
  "files": [
    {"path": "src/lib.rs", "status": "modified", "additions": 10, "deletions": 4, "diff_inline": true}
  ],
  "commits": [{"hash": "abc1234", "subject": "fix parser"}],
  "next": ["axt-gitctx --file src/lib.rs --agent"]
}
```

## Cross-Platform Matrix

| Feature | Linux | macOS | Windows |
|---|---:|---:|---:|
| Git discovery | yes | yes | yes, if git available |
| Status porcelain parsing | yes | yes | yes |
| Diff stat parsing | yes | yes | yes |
| Symlink mode details | yes | yes | partial; document Windows behavior |

## Tests

- Temporary git repositories for clean, dirty, staged, untracked, renamed, and
  deleted files.
- Ahead/behind fixture using local bare remotes.
- Inline diff threshold tests.
- Non-git directory error test.
- Snapshot tests for all output modes.

## Skill Requirements

Create `docs/skills/axt-gitctx/SKILL.md` with rules:

- Use before summarizing local changes or preparing commits.
- Use inline diffs only within byte budget.
- Use `axt-slice` or direct file reads for large changed files listed without
  inline diffs.

Update the skill installer after spec approval.
