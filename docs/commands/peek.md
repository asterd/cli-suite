# axt-peek

`axt-peek` returns a compact directory snapshot: paths, entry kind, size,
content type, language, Git status, modified time, optional hash, warnings, and
summary counts.

## Usage

```bash
axt-peek [OPTIONS] [PATHS]...
axt-peek
axt-peek crates/axt-peek --depth 3
axt-peek fixtures/fs-small --json
axt-peek fixtures/fs-small --agent --summary-only
axt-peek . --changed --kind file --type code
```

`PATHS` defaults to `.` and may include files or directories.

## Options

| Option | Description |
|---|---|
| `PATHS...` | Roots to scan. Default `.`. |
| `--depth <N>` | Maximum traversal depth. Default `2`. |
| `--kind all|file|dir` | Filter entry kind. Default `all`. |
| `--include-hidden` | Include dotfiles and hidden paths. |
| `--no-ignore` | Disable ignore, gitignore, global gitignore, and git exclude filters. |
| `--no-git` | Skip Git discovery and status lookup. |
| `--changed` | Include only entries with non-clean Git status. |
| `--changed-since <REF>` | Include files changed between `<REF>` and `HEAD`. |
| `--type text|binary|image|archive|code|config|data` | Filter by content category. |
| `--lang <LANG>` | Include only entries whose language matches exactly. |
| `--hash none|blake3` | Compute no hash or BLAKE3 hashes. Default `none`. |
| `--summary-only` | Emit only the summary record/section. |
| `--sort name|size|mtime|git|type` | Sort output entries. Default `name`. |
| `--reverse` | Reverse sort order. |
| `--max-file-size <SIZE>` | Skip larger regular files. |
| `--follow-symlinks` | Follow symlinks while walking. |
| `--cross-fs` | Allow traversal across filesystem boundaries. |
| `--json` | Emit the `axt.peek.v1` JSON envelope. |
| `--agent` | Emit minified summary-first JSONL records. |
| `--color auto|always|never` | Color policy for human output. |
| `--print-schema [human|compact|json|agent]` | Print the selected output contract and exit. |
| `--list-errors` | Print the standard error catalog as JSONL and exit. |
| `--limit <N>` | Maximum agent records. Default `200`. |
| `--max-bytes <BYTES>` | Maximum agent output bytes. Default `65536`. |
| `--strict` | Exit with `output_truncated_strict` when truncation is required. |

## Examples

Get the default human summary for the current directory:

```bash
axt-peek
```

List only changed Rust files as agent JSONL for a follow-up agent step:

```bash
axt-peek . --changed --kind file --lang rust --agent
```

Collect a deeper JSON inventory while skipping Git status for speed:

```bash
axt-peek crates --depth 4 --no-git --json
```

Find large files by size without crossing filesystem boundaries:

```bash
axt-peek . --kind file --sort size --reverse --max-file-size 10485760
```

## Output

TTY stdout defaults to human mode. Non-TTY stdout defaults to compact text.
`--json` and `--agent` are explicit structured modes.

Human mode prints a formatted table:

```text
fixtures/fs-small/
  README.md                            56 B  markdown   clean
  dist/                                 0 B             clean
  src/main.rs                          45 B  rust       clean

Summary
  files     4        modified   0
  dirs      2        untracked  0
  bytes     718 B    ignored    1
  git       clean    truncated  no
```

Compact mode is the default for non-TTY capture:

```text
peek root=. files=42 dirs=8 bytes=381204 git=dirty modified=5 untracked=2 ignored=138 truncated=false
file Cargo.toml b=2102 lang=toml git=clean
dir src b=0 lang=- git=clean
```

JSON mode emits `axt.peek.v1`. Agent mode emits summary-first JSONL:

```jsonl
{"schema":"axt.peek.summary.v1","type":"summary","ok":true,"root":".","files":42,"dirs":8,"bytes":381204,"git":"dirty","modified":5,"untracked":2,"truncated":false,"next":["axt-outline src --agent"]}
{"schema":"axt.peek.entry.v1","type":"file","p":"Cargo.toml","b":2102,"l":"toml","g":"clean"}
{"schema":"axt.peek.warn.v1","type":"warn","code":"truncated","reason":"max_records","truncated":true}
```

Agent record schemas:

- `axt.peek.summary.v1`
- `axt.peek.entry.v1`
- `axt.peek.warn.v1`

## Cross-Platform Notes

Directory walking, metadata extraction, mtime output, and Git status are
supported on Linux, macOS, and Windows. Symlink loops and permission failures
are reported as warnings where possible. Use `--no-git` for very large
repositories when Git status is not needed.

## Error Codes

Standard axt error codes are available through `--list-errors`. Common
`axt-peek` failures map to:

- `path_not_found`: an input path does not exist.
- `permission_denied`: a root cannot be read.
- `git_unavailable`: Git status was requested but unavailable.
- `io_error`: filesystem or output serialization failed.
- `output_truncated_strict`: output was truncated under `--strict`.
