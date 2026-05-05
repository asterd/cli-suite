# axt-gitctx

`axt-gitctx` emits bounded local Git context for coding agents: repository,
branch, upstream, ahead/behind counts, dirty state, changed files, diff stats,
recent commits, and small inline diffs.

## Usage

```bash
axt-gitctx --json
axt-gitctx . --agent
axt-gitctx --commits 5 --inline-diff-max-bytes 12000
axt-gitctx --changed-only --agent
```

`ROOT` defaults to `.` and may point anywhere inside a Git worktree.

## Options

| Option | Description |
|---|---|
| `ROOT` | Repository path. Default `.`. |
| `--commits <N>` | Recent commits to include. Default `5`. |
| `--inline-diff-max-bytes <BYTES>` | Per-file inline diff cap. Default `12000`. |
| `--changed-only` | Omit recent commits. |
| `--json` | Emit the `axt.gitctx.v1` JSON envelope. |
| `--agent` | Emit minified summary-first JSONL records. |
| `--print-schema [human\|compact\|json\|agent]` | Print the selected output contract and exit. |
| `--list-errors` | Print the standard error catalog as JSONL and exit. |
| `--limit <N>` | Maximum agent records. Default `200`. |
| `--max-bytes <BYTES>` | Maximum agent output bytes. Default `65536`. |
| `--strict` | Exit with `output_truncated_strict` when truncation is required. |

## Examples

Summarize the current repository for an agent:

```bash
axt-gitctx . --agent
```

Emit JSON including recent commits and inline diffs up to the default cap:

```bash
axt-gitctx --json
```

Focus only on changed files when preparing a commit message:

```bash
axt-gitctx --changed-only --agent
```

Raise the inline diff cap and limit commit history:

```bash
axt-gitctx --commits 3 --inline-diff-max-bytes 24000 --json
```

## Scope

The command is read-only and local-only. It uses local repository data and never
runs `fetch`, `pull`, `push`, `ls-remote`, or hosting-provider API calls.

Changed files include status, index/worktree status, additions, deletions, hunk
count, current byte size, and previous path for renames when Git reports one.
Inline diffs are included only when the generated per-file diff is at or below
`--inline-diff-max-bytes`.

## Output

TTY stdout defaults to human mode. Non-TTY stdout defaults to compact text.
`--json` and `--agent` are explicit structured modes.

Human mode prints a repository summary with aligned change and commit sections:

```text
Repository .
Branch     main upstream=origin/main ahead=1 behind=0
Summary    changed=1 staged=0 unstaged=1 untracked=0 +2 -1 dirty=true truncated=false

Changes
  modified   src/lib.rs                       +2 -1 hunks=1 bytes=420
```

Compact mode is the default for non-TTY capture:

```text
gitctx repo=. branch=main upstream=origin/main ahead=1 behind=0 changed=1 staged=0 unstaged=1 untracked=0 dirty=true truncated=false
file status=modified path=src/lib.rs add=2 del=1 hunks=1 bytes=420 diff_inline=true diff_truncated=false
commit hash=abc1234 author=axt tests subject=initial
```

JSON mode emits `axt.gitctx.v1`:

```json
{
  "schema": "axt.gitctx.v1",
  "ok": true,
  "data": {
    "repo": ".",
    "root": "/work/repo",
    "branch": {"name": "main", "upstream": "origin/main", "ahead": 1, "behind": 0},
    "summary": {"changed": 1, "staged": 0, "unstaged": 1, "untracked": 0, "added": 2, "deleted": 1, "dirty": true, "truncated": false},
    "files": [{"path": "src/lib.rs", "previous_path": null, "status": "modified", "index_status": null, "worktree_status": "modified", "additions": 2, "deletions": 1, "hunks": 1, "bytes": 420, "diff_inline": true, "diff_truncated": false, "diff": "..."}],
    "commits": [{"hash": "abc1234", "subject": "initial", "author": "axt tests", "timestamp": "2026-04-27T10:12:00Z", "age": "2d"}],
    "next": ["axt-slice src/lib.rs --agent"]
  },
  "warnings": [],
  "errors": []
}
```

Agent mode emits:

```jsonl
{"schema":"axt.gitctx.summary.v1","type":"summary","ok":true,"repo":".","branch":"main","upstream":"origin/main","ahead":1,"behind":0,"changed":1,"staged":0,"unstaged":1,"untracked":0,"dirty":true,"truncated":false,"next":["axt-slice src/lib.rs --agent"]}
{"schema":"axt.gitctx.file.v1","type":"file","p":"src/lib.rs","prev":null,"g":"modified","idx":null,"wt":"modified","add":2,"del":1,"hunks":1,"b":420,"diff_inline":true,"diff_truncated":false,"diff":"..."}
{"schema":"axt.gitctx.commit.v1","type":"commit","hash":"abc1234","subject":"initial","author":"axt tests","ts":"2026-04-27T10:12:00Z","age":"2d"}
```

Agent record schemas:

- `axt.gitctx.summary.v1`
- `axt.gitctx.file.v1`
- `axt.gitctx.commit.v1`
- `axt.gitctx.warn.v1`

## Cross-Platform Notes

Repository discovery, status, diff stats, logs, and ahead/behind use local Git
and are supported on Linux, macOS, and Windows when Git is installed.

Windows symlink and executable-bit details depend on filesystem and Git
configuration. The command reports what local Git exposes and does not
fabricate POSIX mode details.

## Error Codes

Standard axt error codes are available through `--list-errors`. Common
`axt-gitctx` failures map to:

- `path_not_found`: the input root does not exist.
- `git_unavailable`: root is not in a Git repository, Git is unavailable, or
  local Git data cannot be read.
- `permission_denied`: a changed file cannot be read.
- `io_error`: filesystem or output serialization failed.
- `output_truncated_strict`: output was truncated under `--strict`.
