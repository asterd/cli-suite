# axt-doc

`axt-doc` diagnoses local development environment problems without making network calls.

## CLI

```bash
axt-doc [FLAGS] which <CMD> [--timeout <DURATION>]
axt-doc [FLAGS] path
axt-doc [FLAGS] env
axt-doc [FLAGS] all <CMD> [--timeout <DURATION>]
```

Shared flags are available before the subcommand: `--json`, `--json-data`, `--jsonl`, `--agent`, `--plain`, `--print-schema`, `--list-errors`, `--limit`, `--max-bytes`, `--strict`, and `--show-secrets`.

`axt-doc all <CMD>` combines `which`, `path`, and `env` in one response.

## Output

JSON mode emits the `axt.doc.v1` envelope:

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

JSONL mode starts with `axt.doc.summary.v1`, then emits detail records for command matches, PATH entries, secret-like variables, and suspicious variables.

Agent mode emits ACF records:

```text
schema=axt.doc.agent.v1 ok=true mode=records which=none path_entries=0 env_vars=42 truncated=false
W code=secret_like_env name=GITHUB_TOKEN
```

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

Values are redacted as `<redacted>` unless `--show-secrets` is passed. `--show-secrets` always prints a warning to stderr, regardless of output mode.

## Cross-Platform Matrix

| Feature | Linux | macOS | Windows | Notes |
| --- | --- | --- | --- | --- |
| PATH duplicate detection | Yes | Yes | Yes | Canonical paths are used when available. |
| Missing PATH directories | Yes | Yes | Yes | Non-existent entries are reported. |
| Broken symlink detection | Yes | Yes | Best effort | Windows symlink metadata depends on permissions and filesystem support. |
| Command resolution | Yes | Yes | Yes | Uses platform command resolution; PATHEXT is honored on Windows. |
| Version manager attribution | Best effort | Best effort | Best effort | Path-pattern based for Homebrew, mise, asdf, rustup, cargo, pyenv, rbenv, volta, nvm, Scoop, Chocolatey, and winget; local manager queries are used when available. |
| Version probing | Yes | Yes | Yes | Runs `<CMD> --version` with a timeout and records failure without failing the command. |
