# ax-peek

`ax-peek` returns a compact snapshot of a directory tree: paths, entry kind,
size, language, git status, mtime, and a summary.

## Usage

```bash
ax-peek [PATHS]...
```

When no path is provided, `ax-peek` scans `.`.

## Examples

```bash
ax-peek
ax-peek crates/ax-peek --depth 3
ax-peek fixtures/fs-small --json
ax-peek fixtures/fs-small --jsonl --summary-only
ax-peek fixtures/fs-small --agent --limit 50
ax-peek . --changed
ax-peek . --changed-since HEAD~1
ax-peek . --hash blake3 --lang rust
ax-peek --list-errors
ax-peek --print-schema json
ax-peek --print-schema agent
```

## Output Modes

- Human mode is the default and prints a compact table plus a summary.
- `--plain` uses the same non-decorative table shape.
- `--json` emits an `ax.peek.v1` JSON envelope.
- `--json-data` emits only the envelope `data` object.
- `--jsonl` emits a summary record first, then one entry record per row.
- `--agent` emits ACF table output for LLM/tool consumption.

## JSON Schema

`--print-schema json` prints `schemas/ax.peek.v1.schema.json`.
`--print-schema agent`, `--print-schema jsonl`, and `--print-schema human`
print compact descriptions for those output contracts.

The JSON envelope shape is:

```json
{
  "schema": "ax.peek.v1",
  "ok": true,
  "data": {
    "root": ".",
    "summary": {
      "files": 42,
      "dirs": 8,
      "bytes": 381204,
      "git_state": "dirty",
      "modified": 5,
      "untracked": 2,
      "ignored": 138,
      "truncated": false
    },
    "entries": []
  },
  "warnings": [],
  "errors": []
}
```

## JSONL Records

The first record always has `type:"summary"` and schema
`ax.peek.summary.v1`.

Entry records use schema `ax.peek.entry.v1` and include:

- `type`: `file`, `dir`, `symlink`, or `other`
- `path`: path relative to the scan root
- `bytes`: raw file byte count; directories use `0`
- `lang`: lower-case language guess, or `null`
- `git`: `clean`, `modified`, `untracked`, `added`, `deleted`, `renamed`,
  `mixed`, or `none`
- `mtime`: RFC 3339 UTC timestamp, or `null`

When output truncates, JSONL appends:

```json
{"schema":"ax.peek.warn.v1","type":"warn","code":"truncated","reason":"max_records","truncated":true}
```

## Agent Output

Agent mode follows ACF. The first line always includes `schema`, `ok`, `mode`,
and `truncated`.

```text
schema=ax.peek.agent.v1 ok=true mode=table root=. cols=path,kind,bytes,lang,git,mtime rows=4 total=42 truncated=false
Cargo.toml,file,2102,toml,clean,2026-04-26T18:02:11Z
```

Agent keys used by `ax-peek`:

- `schema`: `ax.peek.agent.v1`
- `ok`: top-level success marker
- `mode`: `table`
- `root`: displayed scan root
- `cols`: `path,kind,bytes,lang,git,mtime`
- `rows`: emitted entry rows
- `total`: total entry rows before output truncation
- `truncated`: whether output was truncated
- `code`: warning code on `W` records
- `reason`: compact warning reason on `W` records
- `path`: warning path on `W` records, when available

Truncation appends a compact warning record:

```text
W code=truncated reason=max_records truncated=true
```

All keys are part of the shared dictionary in `docs/agent-mode.md`.

## Flags

- `--depth <N>`: maximum traversal depth below each root. Default: `2`.
- `--files-only`: include only regular files. Conflicts with `--dirs-only`.
- `--dirs-only`: include only directories. Conflicts with `--files-only`.
- `--include-hidden`: include dotfiles and hidden paths.
- `--no-ignore`: disable `.ignore`, `.gitignore`, global gitignore, and
  standard ignore filters.
- `--git`: enable automatic git detection. This is the default behavior.
- `--no-git`: skip git discovery and status lookup.
- `--changed`: include only entries with non-clean git status.
- `--changed-since <REF>`: include files changed between `<REF>` and `HEAD`.
- `--type <KIND>`: filter by `text`, `binary`, `image`, `archive`, `code`,
  `config`, or `data`.
- `--lang <LANG>`: include only entries whose language matches exactly, using
  lower-case language names such as `rust`, `markdown`, or `javascript`.
- `--hash none|blake3`: compute no hash or BLAKE3 hashes for regular files.
  Default: `none`.
- `--summary-only`: emit only the summary.
- `--sort name|size|mtime|git|type`: sort output entries. Default: `name`.
- `--reverse`: reverse the selected sort.
- `--max-file-size <SIZE>`: skip regular files larger than this many bytes.
- `--follow-symlinks`: follow symlinks while walking.
- `--cross-fs`: allow traversal across filesystem boundaries.
- `--json`: emit the standard JSON envelope.
- `--jsonl`: emit newline-delimited JSON.
- `--agent`: emit ACF.
- `--json-data`: emit only the JSON `data` payload.
- `--plain`: emit plain human-readable output.
- `--color auto|always|never`: accepted for shared CLI parity.
- `--limit <N>`: cap line-oriented output records before truncation metadata.
  Default: `200`.
- `--max-bytes <SIZE>`: cap line-oriented output bytes. Default: `65536`.
- `--strict`: exit non-zero when output truncation is required.
- `--quiet`: accepted for shared CLI parity.
- `--verbose`, `-v`: accepted for shared CLI parity.
- `--print-schema [human|json|jsonl|agent]`: print an output schema or compact
  output contract description and exit. The default format is `json`.
- `--list-errors`: print the standard error catalog as JSONL and exit.
- `--version`: print the version and exit.
- `--help`: print help and exit.

Output mode flags are mutually exclusive.

## Error Codes

`--list-errors` prints the full standard catalog from `ax-core`.

Full catalog:

- `ok`: success.
- `runtime_error`: generic runtime failure.
- `usage_error`: invalid CLI flags, including conflicting modes.
- `path_not_found`: an input path does not exist.
- `permission_denied`: a subtree cannot be read on the current platform.
- `timeout`: reserved standard timeout code.
- `output_truncated_strict`: `--strict` was set and output was truncated.
- `interrupted`: SIGINT / Ctrl-C received.
- `io_error`: filesystem or output stream I/O failed.
- `feature_unsupported`: feature unavailable on this platform.
- `schema_violation`: produced data failed its own schema validation.
- `command_failed`: reserved for command-running tools.
- `git_unavailable`: git discovery or status lookup failed.
- `config_error`: user config file malformed.
- `network_disabled`: offline command attempted network.

Non-fatal warning records use `code` values such as `permission_denied`,
`symlink_loop`, `path_not_utf8`, and `git_capped`.

## Cross-Platform Notes

Directory walking, metadata extraction, mtime output, and git status are
supported on Linux, macOS, and Windows. Windows reserved names are treated as
ordinary paths. Platform-specific filesystem permissions may affect whether a
permission-denied fixture can be reproduced locally.

Submodules are treated as directories with mixed status when git reports mixed
state. `--no-git` is the escape hatch for very large repositories.
