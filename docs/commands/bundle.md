# axt-bundle

`axt-bundle` emits a compact session warmup bundle: a shallow file inventory,
recognized local manifests, Git state when available, and dynamic next-step
hints. It is designed for the first command an agent runs in an unfamiliar
repository.

## Usage

```bash
axt-bundle [OPTIONS] [ROOT]
axt-bundle .
axt-bundle . --agent
axt-bundle . --json
axt-bundle . --depth 3 --max-files 80 --agent
```

`ROOT` defaults to `.`. The command is read-only and does not execute project
code.

## Options

| Option | Description |
|---|---|
| `ROOT` | Root directory to inspect. Defaults to `.`. |
| `--depth <N>` | File inventory traversal depth. Default `2`. |
| `--max-files <N>` | Maximum file/directory records retained in the bundle. Default `40`. |
| `--include-hidden` | Include hidden paths. Hidden paths are skipped by default. |
| `--no-ignore` | Disable ignore, gitignore, global gitignore, and git exclude filters. |
| `--json` | Emit the `axt.bundle.v1` JSON envelope. |
| `--agent` | Emit minified summary-first JSONL records. |
| `--print-schema [human|json|agent]` | Print the selected output contract and exit. |
| `--list-errors` | Print the standard error catalog as JSONL and exit. |
| `--limit <N>` | Maximum line-oriented records for agent output. Default `200`. |
| `--max-bytes <BYTES>` | Maximum line-oriented output bytes. Default `65536`. |
| `--strict` | Exit with `output_truncated_strict` when truncation is required. |

## Examples

Warm up an agent session from the repository root:

```bash
axt-bundle . --agent
```

Inspect a project as JSON for a script:

```bash
axt-bundle /work/repo --json
```

Capture a deeper first-pass inventory:

```bash
axt-bundle . --depth 3 --max-files 80 --agent
```

Include dotfiles when project configuration is likely hidden:

```bash
axt-bundle . --include-hidden --no-ignore
```

## Manifest Detection

The command previews up to 12 lines from recognized local manifests:

| Manifest | Kind |
|---|---|
| `Cargo.toml` | `rust` |
| `package.json`, `package-lock.json`, `pnpm-lock.yaml`, `bun.lock`, `deno.json` | `js` |
| `pyproject.toml` | `python` |
| `go.mod` | `go` |

Manifest previews are intended for quick orientation, not full dependency
analysis.

## Output

Human mode prints a one-line summary plus manifest and Git summaries:

```text
root=. files=42 dirs=12 manifests=2 git=true truncated=false
manifest Cargo.toml 1842B
git branch=main modified=1 untracked=0
```

JSON mode emits the canonical envelope:

```json
{
  "schema": "axt.bundle.v1",
  "ok": true,
  "data": {
    "root": ".",
    "summary": {
      "files": 42,
      "dirs": 12,
      "manifests": 2,
      "git": true,
      "truncated": false
    },
    "files": [],
    "manifests": [],
    "git": null,
    "next": ["axt-peek . --agent", "axt-outline . --agent"]
  },
  "warnings": [],
  "errors": []
}
```

Agent mode emits summary-first JSONL:

```jsonl
{"schema":"axt.bundle.summary.v1","type":"summary","ok":true,"root":".","files":42,"dirs":12,"manifests":2,"git":true,"truncated":false,"next":["axt-peek . --agent","axt-outline . --agent","axt-test --agent"]}
{"schema":"axt.bundle.manifest.v1","type":"manifest","p":"Cargo.toml","k":"rust","b":1842,"preview":"[workspace]\nresolver = \"2\""}
{"schema":"axt.bundle.git.v1","type":"git","root":"/repo","branch":"main","modified":1,"untracked":0}
{"schema":"axt.bundle.file.v1","type":"file","p":"src/lib.rs","k":"file","b":4210,"l":"rust"}
```

Agent record schemas:

- `axt.bundle.summary.v1`
- `axt.bundle.manifest.v1`
- `axt.bundle.git.v1`
- `axt.bundle.file.v1`
- `axt.bundle.warn.v1`

## Next Hints

`next` always includes `axt-peek <ROOT> --agent` and
`axt-outline <ROOT> --agent`. It adds `axt-peek <ROOT> --changed --agent` when a
readable Git repository has modified or untracked files, and `axt-test --agent`
when a recognized manifest is present.

## Cross-Platform Notes

Filesystem walking, manifest detection, and JSON/agent rendering are supported
on Linux, macOS, and Windows. Git state is included only when `ROOT` is inside a
readable local Git worktree; absence of Git is represented as `git: null` in
JSON and omitted from agent detail records.

## Error Codes

Standard axt error codes are available through `--list-errors`. Common
`axt-bundle` failures map to:

- `path_not_found`: `ROOT` does not exist.
- `permission_denied`: a directory or manifest cannot be read.
- `git_unavailable`: Git metadata was detected but could not be read.
- `io_error`: filesystem or output serialization failed.
- `output_truncated_strict`: output was truncated under `--strict`.
