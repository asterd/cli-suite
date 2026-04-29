# axt Foundation CLI Suite

`axt` is a set of small, native command-line tools for inspecting local projects,
running commands, checking environment problems, tracking file changes, inspecting
ports, normalizing test output, and extracting source outlines.

The goal is to make local developer and agent workflows easier to automate
without turning every task into a custom shell script. Each command does one job,
runs offline, returns compact human output by default, and can also emit stable
machine-readable formats for scripts, CI jobs, and coding agents. The binaries do
not send telemetry, perform analytics, or make network calls.

The suite is intentionally narrow:

| Command | Short purpose |
|---|---|
| `axt-peek` | Snapshot directories, file metadata, language guesses, and Git state. |
| `axt-run` | Run a command with structured exit, stream, duration, and file-change data. |
| `axt-doc` | Diagnose local PATH, command, and environment problems. |
| `axt-drift` | Mark filesystem state and report changes since the mark. |
| `axt-port` | Inspect local TCP/UDP port holders and optionally free ports. |
| `axt-test` | Run and normalize test suites across common frameworks. |
| `axt-outline` | Emit compact tree-sitter source outlines without function bodies. |

## Compatibility Matrix

| Command | Linux | macOS | Windows | Notes |
|---|---:|---:|---:|---|
| `axt-peek` | Yes | Yes | Yes | Git and filesystem permission behavior is platform-dependent. |
| `axt-run` | Yes | Yes | Yes | Unix uses process groups for timeout cleanup; Windows uses Job Objects. |
| `axt-doc` | Yes | Yes | Yes | Windows symlink checks are best effort. |
| `axt-drift` | Yes | Yes | Yes | Hash mode is portable and slower than metadata mode. |
| `axt-port` | Yes | Yes | Yes | macOS uses local `lsof`; Windows uses local `netstat`, PowerShell process lookup, and `taskkill`. |
| `axt-test` | Yes | Yes | Yes | Framework support depends on the local toolchain being installed. |
| `axt-outline` | Yes | Yes | Yes | Uses embedded tree-sitter grammars; no parser tools or LSP servers required. |

When a feature cannot be implemented on a platform, commands return
`feature_unsupported` with exit code `9` rather than silently degrading.

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

Shared flags include `--print-schema`, `--list-errors`, `--limit`,
`--max-bytes`, and `--strict`. Diagnostics go to stderr; data goes to stdout.

## Installation

Use the published packages once a release is available. Until then, install from
a local source checkout.

### From Published Releases

After the first public release, installation will be available through the
platform package channels prepared by the project:

| System | Intended install path |
|---|---|
| Linux | GitHub release archive, shell installer, or `cargo install axt-peek --locked` |
| macOS | Homebrew, GitHub release archive, shell installer, or Cargo |
| Windows | Scoop, PowerShell installer, GitHub release archive, or Cargo |

Each command is distributed as its own binary package. Install only the commands
you need, or install the full suite when your package channel supports it.

### From Source

Install the complete suite from a checkout:

```bash
python3 scripts/install-local.py --command all
```

On Windows:

```powershell
py scripts/install-local.py --command all
```

Install one command:

```bash
python3 scripts/install-local.py --command peek
cargo install --path crates/axt-peek --locked
```

Canonical command names are always `axt-*`. Optional short aliases are available
only when explicitly installed with the `aliases` feature:

| Canonical | Optional short alias |
|---|---|
| `axt-peek` | `peek` |
| `axt-run` | `run` |
| `axt-doc` | `doc` |
| `axt-drift` | `drift` |
| `axt-port` | `port` |
| `axt-test` | `test` |
| `axt-outline` | `outline` |

There are no `ax-*` aliases. Prefer canonical `axt-*` names in scripts and CI.
See [docs/installation.md](docs/installation.md) for the full install matrix and
verification commands.

## Commands

Each command has a dedicated command page with the complete option list, output
contracts, examples, error codes, and cross-platform notes:

| Command | Manual |
|---|---|
| `axt-peek` | [docs/commands/peek.md](docs/commands/peek.md) |
| `axt-run` | [docs/commands/run.md](docs/commands/run.md) |
| `axt-doc` | [docs/commands/doc.md](docs/commands/doc.md) |
| `axt-drift` | [docs/commands/drift.md](docs/commands/drift.md) |
| `axt-port` | [docs/commands/port.md](docs/commands/port.md) |
| `axt-test` | [docs/commands/test.md](docs/commands/test.md) |
| `axt-outline` | [docs/commands/outline.md](docs/commands/outline.md) |

### `axt-peek`

Snapshots one or more directory roots. It reports entry type, size, language,
Git status, modified time, optional BLAKE3 hash, and summary counts.

```bash
axt-peek .
axt-peek crates/axt-peek --depth 3 --agent
axt-peek . --changed --json
```

Output examples:

```text
path        kind  bytes  lang      git       mtime
Cargo.toml  file  2102   toml      clean     2026-04-26T18:02:11Z
```

```text
schema=axt.peek.agent.v1 ok=true mode=table root=. cols=path,kind,bytes,lang,git,mtime rows=4 total=42 truncated=false
Cargo.toml,file,2102,toml,clean,2026-04-26T18:02:11Z
```

Full options and output contracts: [docs/commands/peek.md](docs/commands/peek.md).

### `axt-run`

Runs a child command and returns an execution envelope: command, exit code,
duration, stdout/stderr line counts and tails, saved log paths, timeout state,
and changed files.

```bash
axt-run -- cargo test
axt-run --timeout 30s --json -- npm test
axt-run show last --stderr
axt-run list
axt-run clean --older-than 7d
```

Output examples:

```text
ok=true exit=0 duration=2.13s stdout_lines=18 stderr_lines=0 changed=0 saved=.axt/runs/last
```

```text
schema=axt.run.agent.v1 ok=true exit=0 timed_out=false duration_ms=2130 stdout_lines=18 stderr_lines=0 changed=0
```

Artifacts are stored below `.axt/runs/<name>/` unless `--no-save` is used.
Full options and output contracts: [docs/commands/run.md](docs/commands/run.md).

### `axt-doc`

Diagnoses local environment issues without network calls. It resolves commands,
checks duplicate or missing PATH entries, finds broken symlinks where supported,
and redacts secret-like environment variables.

```bash
axt-doc which cargo --json
axt-doc path --agent
axt-doc env
axt-doc all rustc
```

Output examples:

```text
cargo: found /Users/me/.cargo/bin/cargo
```

```text
schema=axt.doc.agent.v1 ok=true command=cargo found=true path=/Users/me/.cargo/bin/cargo
```

Use `--show-secrets` only for local debugging; values are redacted by default.
Full options and output contracts: [docs/commands/doc.md](docs/commands/doc.md).

### `axt-drift`

Creates filesystem marks and later reports created, modified, and deleted files.
It is useful after builds, generators, and test runs.

```bash
axt-drift mark --name before
axt-drift diff --since before --json
axt-drift run -- cargo build
axt-drift reset
```

Output examples:

```text
created=2 modified=1 deleted=0 since=before
```

```text
schema=axt.drift.agent.v1 ok=true created=2 modified=1 deleted=0 since=before truncated=false
```

Marks are stored under `.axt/drift`. `--hash` uses BLAKE3 to detect content
changes beyond metadata changes. Full options and output contracts:
[docs/commands/drift.md](docs/commands/drift.md).

### `axt-port`

Inspects local TCP/UDP sockets and maps listening ports to process metadata. The
`free` subcommand can signal holders.

```bash
axt-port who 3000
axt-port list --proto both
axt-port free 3000 --dry-run --agent
axt-port watch 3000 --timeout 5s
```

Output examples:

```text
3000 tcp listen pid=12345 name=node
```

```text
schema=axt.port.agent.v1 ok=true port=3000 holders=1 action=inspect
P port=3000 proto=tcp state=listen pid=12345 name=node
```

Safety controls include `--dry-run`, `--confirm`, `--signal term|kill|int`,
`--grace`, `--tree`, and `--force-self`. The command refuses PID 1 and its own
process. Full options and output contracts:
[docs/commands/port.md](docs/commands/port.md).

### `axt-test`

Detects and runs project test suites, then normalizes results across Jest,
Vitest, Pytest, Cargo, Go, Bun, and Deno.

```bash
axt-test
axt-test --framework cargo --json
axt-test --changed --agent
axt-test list-frameworks
```

Output examples:

```text
framework=cargo passed=42 failed=0 skipped=0 duration=4.8s
```

```text
schema=axt.test.agent.v1 ok=true framework=cargo passed=42 failed=0 skipped=0 duration_ms=4800
```

Normalized values include framework, suite, case name, status, duration, file,
line, message, stdout, and stderr when available. Full options and output
contracts: [docs/commands/test.md](docs/commands/test.md).

### `axt-outline`

Emits declarations, signatures, doc comments, visibility, paths, and source
ranges without function bodies. It uses embedded tree-sitter grammars for Rust,
TypeScript, JavaScript, Python, Go, Java, and PHP, and reports
source/signature byte counts to make compression visible.

```bash
axt-outline crates/axt-outline/src --agent
axt-outline src/lib.rs --public-only --json
axt-outline app --lang typescript --jsonl
```

Output examples:

```text
src/lib.rs:42 pub fn parse_config(input: &str) -> Result<Config, Error>
```

```text
schema=axt.outline.agent.v1 ok=true mode=records files=1 symbols=3 warnings=0 source_bytes=8192 signature_bytes=240 truncated=false
Y path=src/lib.rs lang=rust kind=fn visibility=pub name=parse_config line=42 end_line=57 parent=- signature="pub fn parse_config(input: &str) -> Result<Config, Error>"
```

Use it before reading large source files when symbol-level context is enough.
Full options and output contracts:
[docs/commands/outline.md](docs/commands/outline.md).

## Security and Production Notes

- `axt` commands are local-only and do not make network calls.
- There is no telemetry, analytics, or background reporting.
- Data output is written to stdout; diagnostics are written to stderr.
- `axt-run`, `axt-test`, and `axt-drift run` execute local commands you provide.
  Review those commands the same way you would review running them directly.
- `axt-port free` can terminate local processes. Start with `--dry-run` and use
  `--confirm` for intentional process cleanup.
- `axt-drift` and `axt-run` may write local artifacts under `.axt/`.
- JSON, JSONL, and agent output are schema-versioned so scripts can detect
  incompatible changes.

Security policy and disclosure guidance live in [SECURITY.md](SECURITY.md).

## Manpages and Agent Skills

Manual pages are maintained in `docs/man/*.1`. Agent skills live under
`docs/skills/` and can be installed into Codex or Claude Code:

```bash
python3 scripts/agent/install-skills.py --agent both --scope project --skill axt-suite
python3 scripts/agent/install-skills.py --agent both --scope project --skill all
```

The first command installs the suite-level skill. The second installs the
suite-level skill plus one focused skill per command.

## Contributing

Contributors should start from [CONTRIBUTING.md](CONTRIBUTING.md), not internal
agent instructions or design specs. Contributions should preserve the project
shape:

- each command is a focused single-binary tool;
- behavior is cross-platform by default;
- commands stay ultra-fast on normal repositories;
- defaults are conservative and safe;
- no telemetry, analytics, or network access is added to binaries;
- scripts and CI use canonical `axt-*` command names.

Before submitting changes:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## Next Steps

- Cut the first public release and publish installable artifacts for the
  supported package channels.
- Keep command manuals aligned with `--help`, `--print-schema`, and
  `--list-errors`.
- Add more real-world framework fixtures for `axt-test`.
- Expand platform smoke tests around process and port handling.
- Collect user feedback on which short aliases are useful enough to keep
  opt-in.
