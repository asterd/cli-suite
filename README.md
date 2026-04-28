# axt Foundation CLI Suite

`axt` is a Rust suite of small native command-line tools for coding agents and developers. Each command emits stable, schema-versioned output for machines and compact human output by default. The suite is offline by design: no usage reporting, no analytics, no network calls from the binaries.

The v1 surface is intentionally fixed at six commands:

| Command | Short purpose | Mutates state |
|---|---|---:|
| `axt-peek` | Snapshot directories, file metadata, languages, and Git state. | No |
| `axt-run` | Run a command with structured exit, stream, duration, and file-change data. | Yes, through the child command |
| `axt-doc` | Diagnose local PATH, command, and environment problems. | No |
| `axt-drift` | Mark filesystem state and report changes since the mark. | Writes `.axt/drift` marks |
| `axt-port` | Inspect local TCP/UDP port holders and optionally free ports. | Only `free` |
| `axt-test` | Run and normalize test suites across common frameworks. | Through test commands |

## Compatibility Matrix

| Command | Linux | macOS | Windows | Notes |
|---|---:|---:|---:|---|
| `axt-peek` | Yes | Yes | Yes | Git and filesystem permission behavior is platform-dependent. |
| `axt-run` | Yes | Yes | Yes | Unix uses process groups for timeout cleanup; Windows uses Job Objects. |
| `axt-doc` | Yes | Yes | Yes | Windows symlink checks are best effort. |
| `axt-drift` | Yes | Yes | Yes | Hash mode is portable and slower than metadata mode. |
| `axt-port` | Yes | Yes | Yes | macOS uses local `lsof`; Windows uses local `netstat`, PowerShell process lookup, and `taskkill`. |
| `axt-test` | Yes | Yes | Yes | Framework support depends on the local toolchain being installed. |

When a feature cannot be implemented on a platform, commands must return `feature_unsupported` with exit code `9` rather than silently degrading.

## Output Modes

Every command supports the shared modes:

| Mode | Flag | Use |
|---|---|---|
| Human | default | Compact terminal output for people. |
| Plain | `--plain` | Human-readable output without decoration. |
| JSON | `--json` | Stable envelope: `schema`, `ok`, `data`, `warnings`, `errors`. |
| JSON data | `--json-data` | Only the command payload from the JSON envelope. |
| JSONL | `--jsonl` | Streaming newline-delimited records. |
| Agent | `--agent` | ACF, the compact line-oriented format for LLM contexts. |

Shared flags include `--print-schema`, `--list-errors`, `--limit`, `--max-bytes`, and `--strict`. Diagnostics go to stderr; data goes to stdout.

## Installation

Releases are prepared for GitHub Releases, shell and PowerShell installers, Homebrew, Scoop, and Cargo through `cargo-dist`. Until a public release is cut, install from a local checkout:

| System | Local install | Release channel |
|---|---|---|
| Linux | `cargo install --path crates/axt-peek --locked` | Shell installer, Cargo, GitHub archive |
| macOS | `cargo install --path crates/axt-peek --locked` | Homebrew, shell installer, Cargo, GitHub archive |
| Windows | `cargo install --path crates/axt-peek --locked` | Scoop, PowerShell installer, Cargo, GitHub archive |

Repeat the local install command for each command crate, or use `cargo install --path crates/<crate> --locked`.

### Optional Aliases

Canonical binary names are `axt-<command>`. Each binary crate also provides opt-in aliases:

| Canonical | `ax-*` alias | Short alias |
|---|---|---|
| `axt-peek` | `ax-peek` | `peek` |
| `axt-run` | `ax-run` | `run` |
| `axt-doc` | `ax-doc` | `doc` |
| `axt-drift` | `ax-drift` | `drift` |
| `axt-port` | `ax-port` | `port` |
| `axt-test` | `ax-test` | `test` |

Install aliases explicitly:

```bash
cargo install --path crates/axt-peek --locked --features aliases
```

Short names are generic and may collide with existing commands. Prefer canonical `axt-*` names in scripts and CI.

## Commands

### `axt-peek`

Snapshots one or more directory roots. It reports entry type, size, language, Git status, modified time, optional BLAKE3 hash, and summary counts.

```bash
axt-peek .
axt-peek crates/axt-peek --depth 3 --agent
axt-peek . --changed --json
```

Important values: `git` is `clean`, `modified`, `untracked`, `added`, `deleted`, `renamed`, `mixed`, or `none`; `lang` is a lowercase language guess or `null`; `bytes` is raw byte count.

### `axt-run`

Runs a child command and returns an execution envelope: command, exit code, duration, stdout/stderr line counts and tails, saved log paths, timeout state, and changed files.

```bash
axt-run -- cargo test
axt-run --timeout 30s --json -- npm test
axt-run show last --stderr
axt-run list
axt-run clean --older-than 7d
```

Artifacts are stored below `.axt/runs/<name>/` unless `--no-save` is used.

### `axt-doc`

Diagnoses local environment issues without network calls. It resolves commands, checks duplicate or missing PATH entries, finds broken symlinks where supported, and redacts secret-like environment variables.

```bash
axt-doc which cargo --json
axt-doc path --agent
axt-doc env
axt-doc all rustc
```

Use `--show-secrets` only for local debugging; values are redacted by default.

### `axt-drift`

Creates filesystem marks and later reports created, modified, and deleted files. It is useful after builds, generators, and test runs.

```bash
axt-drift mark --name before
axt-drift diff --since before --json
axt-drift run -- cargo build
axt-drift reset
```

Marks are stored under `.axt/drift`. `--hash` uses BLAKE3 to detect content changes beyond metadata changes.

### `axt-port`

Inspects local TCP/UDP sockets and maps listening ports to process metadata. The `free` subcommand can signal holders.

```bash
axt-port who 3000
axt-port list --proto both
axt-port free 3000 --dry-run --agent
axt-port watch 3000 --timeout 5s
```

Safety controls include `--dry-run`, `--confirm`, `--signal term|kill|int`, `--grace`, `--tree`, and `--force-self`. The command refuses PID 1 and its own process.

### `axt-test`

Detects and runs project test suites, then normalizes results across Jest, Vitest, Pytest, Cargo, Go, Bun, and Deno.

```bash
axt-test
axt-test --framework cargo --json
axt-test --changed --agent
axt-test list-frameworks
```

Normalized values include framework, suite, case name, status, duration, file, line, message, stdout, and stderr when available.

## Security and Production Notes

- Source crates deny `unwrap()` and `expect()` through Clippy and currently contain no non-test `unwrap()` or `expect()` calls.
- Shared libraries deny unsafe code. Platform-specific process control uses narrowly scoped `unsafe` blocks in `axt-run` and `axt-port` with documented safety comments.
- Binaries do not include HTTP client dependencies and do not perform network calls.
- `axt-port free`, `axt-run`, `axt-drift mark/reset/run`, and `axt-test` can mutate local state or run mutating child commands. Use `--dry-run` where available.
- Schemas live in `schemas/`; command behavior is documented in `docs/commands/`.
- The hardening review is tracked in `docs/security-hardening.md`.

## Manpages and Agent Skills

Manual pages are maintained in `docs/man/*.1`. Agent skill instructions are maintained in `docs/skills/axt-suite/SKILL.md` and can be copied into Codex, Claude Code, or other agent skill directories.

## Contributing

Follow `AGENTS.md`, `CONTRIBUTING.md`, `docs/spec.md`, and `docs/spec-addendum.md`. Do not add commands beyond the six-command v1 surface. Do not add usage reporting, network calls, or postinstall fetch scripts.

Before submitting changes:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## Next Steps

- Run a full cross-platform CI pass on Linux, macOS, and Windows.
- Validate release artifacts from `cargo-dist` before cutting the first public release.
- Expand real framework reporter integration for `axt-test` where stable native machine output is not available.
- Add platform smoke tests for `axt-port free` using owned fixture processes.
