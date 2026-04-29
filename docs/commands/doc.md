# axt-doc

`axt-doc` diagnoses local development environment problems without network
calls. It resolves commands, checks PATH health, and summarizes secret-like or
suspicious environment variables.

## Usage

```bash
axt-doc [OPTIONS] which <CMD> [--timeout <DURATION>]
axt-doc [OPTIONS] <CMD>
axt-doc [OPTIONS] path
axt-doc [OPTIONS] env
axt-doc [OPTIONS] all <CMD> [--timeout <DURATION>]
```

Passing `axt-doc <CMD>` without a subcommand is shorthand for
`axt-doc all <CMD>`.

## Options

| Option | Description |
|---|---|
| `which <CMD>` | Resolve a command on `PATH` and optionally probe its version. |
| `path` | Report duplicate, missing, and suspicious PATH entries. |
| `env` | Report secret-like and suspicious environment variables. |
| `all <CMD>` | Combine `which`, `path`, and `env` in one response. |
| `--timeout <DURATION>` | Version probe timeout where supported. |
| `--show-secrets` | Show secret-like values. Default is redacted. Always warns on stderr. |
| `--json` | Emit the `axt.doc.v1` JSON envelope. |
| `--agent` | Emit minified summary-first JSONL records. |
| `--print-schema [human|json|agent]` | Print the selected output contract and exit. |
| `--list-errors` | Print the standard error catalog as JSONL and exit. |
| `--limit <N>` | Maximum agent records. Default `200`. |
| `--max-bytes <BYTES>` | Maximum agent output bytes. Default `65536`. |
| `--strict` | Exit with `output_truncated_strict` when truncation is required. |

## Output

JSON mode emits `axt.doc.v1`:

```json
{
  "schema": "axt.doc.v1",
  "ok": true,
  "data": {
    "which": null,
    "path": null,
    "env": null
  },
  "warnings": [],
  "errors": []
}
```

Agent mode emits summary-first JSONL beginning with `axt.doc.summary.v1`,
followed by command matches, PATH entries, secret-like variables, suspicious
variables, and warnings.

## Secret Handling

Secret-like environment variable names are detected case-insensitively:

- `*_TOKEN`
- `*_SECRET*`
- `*_KEY`
- `*_PASSWORD`
- `PASS`
- `*_CREDENTIAL*`
- `*_PRIVATE*`
- `*_AUTH*`

Values are redacted as `<redacted>` unless `--show-secrets` is passed.

## Cross-Platform Notes

| Feature | Linux | macOS | Windows | Notes |
|---|---:|---:|---:|---|
| PATH duplicate detection | Yes | Yes | Yes | Canonical paths are used when available. |
| Missing PATH directories | Yes | Yes | Yes | Non-existent entries are reported. |
| Broken symlink detection | Yes | Yes | Best effort | Windows metadata depends on permissions and filesystem support. |
| Command resolution | Yes | Yes | Yes | PATHEXT is honored on Windows. |
| Version manager attribution | Best effort | Best effort | Best effort | Path-pattern based for Homebrew, mise, asdf, rustup, cargo, pyenv, rbenv, volta, nvm, Scoop, Chocolatey, and winget. |
| Version probing | Yes | Yes | Yes | Runs `<CMD> --version` with a timeout and records failure without failing the command. |

## Error Codes

Standard axt error codes are available through `--list-errors`. Common
`axt-doc` failures map to:

- `usage_error`: missing command or invalid arguments.
- `path_not_found`: command or PATH target cannot be resolved.
- `permission_denied`: metadata or version probing was denied.
- `timeout`: version probing exceeded the timeout.
- `io_error`: filesystem, process IO, or output serialization failed.
- `output_truncated_strict`: output was truncated under `--strict`.
