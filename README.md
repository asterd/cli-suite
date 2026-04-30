# axt Foundation CLI Suite

`axt` is a suite of small native CLIs for local developer and coding-agent
workflows. Each command is offline, schema-versioned, deterministic, and usable
from either a terminal or an agent loop.

The suite does not call LLMs, send telemetry, run analytics, or make network
requests from the binaries.

## Commands

| Command | Purpose | Manual |
|---|---|---|
| `axt-peek` | Snapshot files, metadata, languages, and Git state. | [docs/commands/peek.md](docs/commands/peek.md) |
| `axt-run` | Run a command with structured exit, stream, duration, and file-change data. | [docs/commands/run.md](docs/commands/run.md) |
| `axt-doc` | Diagnose command, PATH, and environment issues. | [docs/commands/doc.md](docs/commands/doc.md) |
| `axt-drift` | Mark filesystem state and report changes since the mark. | [docs/commands/drift.md](docs/commands/drift.md) |
| `axt-port` | Inspect local port holders and optionally free ports. | [docs/commands/port.md](docs/commands/port.md) |
| `axt-test` | Run and normalize test output across supported frameworks. | [docs/commands/test.md](docs/commands/test.md) |
| `axt-outline` | Emit compact source outlines without function bodies. | [docs/commands/outline.md](docs/commands/outline.md) |
| `axt-slice` | Extract exact source by symbol or enclosing line. | [docs/commands/slice.md](docs/commands/slice.md) |
| `axt-ctxpack` | Search multiple named patterns with compact snippets and AST classification. | [docs/commands/ctxpack.md](docs/commands/ctxpack.md) |
| `axt-bundle` | Emit a session warmup bundle with files, manifests, Git state, and next hints. | [docs/commands/bundle.md](docs/commands/bundle.md) |
| `axt-gitctx` | Emit bounded local Git branch, status, commit, and diff context. | [docs/commands/gitctx.md](docs/commands/gitctx.md) |
| `axt-logdx` | Diagnose large local logs with grouped failures, stack traces, timelines, and snippets. | [docs/commands/logdx.md](docs/commands/logdx.md) |

## Output Contract

Every command supports three primary modes:

| Mode | Selection | Contract |
|---|---|---|
| Human | default on TTY stdout | Compact terminal output for people. Human text is not a stable parse target. |
| JSON | `--json` | Stable envelope: `schema`, `ok`, `data`, `warnings`, `errors`. |
| Agent | default on non-TTY stdout or `--agent` | Minified JSONL, one object per line, summary record first. |

Shared flags:

- `--print-schema [human|json|agent]`
- `--list-errors`
- `--limit <N>`
- `--max-bytes <BYTES>`
- `--strict`

`AXT_OUTPUT=human|json|agent` overrides automatic TTY mode selection.
Diagnostics go to stderr; data goes to stdout. `--plain`, `--json-data`, and
`--jsonl` are retired public flags; use human output, `jq .data`, and
`--agent` respectively.

## Quick Examples

```bash
axt-bundle . --agent
axt-peek . --changed --json
axt-outline crates/axt-test/src --agent
axt-slice crates/axt-test/src/main.rs --symbol main --agent
axt-ctxpack --pattern todo=TODO --pattern panic='unwrap\(|expect\(' crates --agent
axt-gitctx . --changed-only --agent
axt-logdx target/test.log --severity error --top 20 --agent
axt-test --framework cargo --agent
axt-port free 3000 --dry-run --agent
```

Example agent output:

```jsonl
{"schema":"axt.peek.summary.v1","type":"summary","ok":true,"root":".","files":42,"dirs":8,"bytes":381204,"git":"dirty","modified":5,"untracked":2,"truncated":false,"next":["axt-outline src --agent"]}
{"schema":"axt.peek.entry.v1","type":"file","p":"Cargo.toml","b":2102,"l":"toml","g":"clean"}
```

## Installation

Until public releases are cut, install from a local checkout:

```bash
python3 scripts/install-local.py --command all
```

On Windows:

```powershell
py scripts/install-local.py --command all
```

Install one command directly:

```bash
cargo install --path crates/axt-peek --locked
```

Optional short aliases are available only when explicitly installed with the
`aliases` feature. Canonical `axt-*` names should be used in scripts and CI.

See [docs/installation.md](docs/installation.md) for the full install matrix.

## Compatibility

| Command | Linux | macOS | Windows | Notes |
|---|---:|---:|---:|---|
| `axt-peek` | Yes | Yes | Yes | Git and filesystem permission behavior is platform-dependent. |
| `axt-run` | Yes | Yes | Yes | Unix uses process groups; Windows uses Job Objects for owned child timeout cleanup. |
| `axt-doc` | Yes | Yes | Yes | Windows symlink checks are best effort. |
| `axt-drift` | Yes | Yes | Yes | Hash mode is portable and slower than metadata mode. |
| `axt-port` | Yes | Yes | Yes | Process metadata and cwd are best effort where OS permissions restrict access. |
| `axt-test` | Yes | Yes | Yes | Requires local framework toolchains. Nothing is downloaded. |
| `axt-outline` | Yes | Yes | Yes | Uses embedded tree-sitter grammars. |
| `axt-slice` | Yes | Yes | Yes | Uses embedded tree-sitter grammars for supported source files. |
| `axt-ctxpack` | Yes | Yes | Yes | Embedded tree-sitter where supported, heuristic fallback otherwise. |
| `axt-bundle` | Yes | Yes | Yes | Git state is included only inside a readable worktree. |
| `axt-gitctx` | Yes | Yes | Yes | Uses local Git data only; no fetch, pull, push, or remote API calls. |
| `axt-logdx` | Yes | Yes | Yes | UTF-8-ish byte logs are decoded lossily when needed; parsing is deterministic and bounded. |

When a feature is unavailable on a platform, commands return
`feature_unsupported` with exit code `9` rather than silently fabricating data.

## Documentation

- Command manuals: [docs/commands/](docs/commands/)
- Agent JSONL contract: [docs/agent-mode.md](docs/agent-mode.md)
- Installation matrix: [docs/installation.md](docs/installation.md)
- Release runbook: [docs/release.md](docs/release.md)
- Security review: [docs/security-hardening.md](docs/security-hardening.md)
- Agent skills: [docs/skills/](docs/skills/)

Manual pages are maintained in `docs/man/*.1`.

## Development

Quality gates:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Additional audit commands:

```bash
cargo check --workspace --all-features
rg "unwrap\\(|expect\\(" crates/*/src
rg "reqwest|ureq|hyper|isahc" crates Cargo.toml
```

## Security Notes

- Commands are local-only and do not make network calls.
- JSON and agent JSONL are serialized through `serde_json`.
- `axt-doc` redacts secret-like environment variables by default.
- `axt-run`, `axt-test`, and `axt-drift run` execute local commands you provide.
- `axt-port free` can terminate local processes; start with `--dry-run`.
- `axt-run` and `axt-drift` may write local artifacts below `.axt/`.
- `axt-logdx` reads local logs that may contain secrets; review snippets before sharing output.

Security policy and disclosure guidance live in [SECURITY.md](SECURITY.md).

## Roadmap

Implemented suite surface is the twelve commands listed above. Proposed future
commands and follow-up work live in [docs/evolutive/](docs/evolutive/) and must
be promoted into the spec/addendum before implementation. Current high-priority
follow-ups are:

- improve real-world `axt-test` fixture coverage for the seven supported
  frameworks;
- add broader OS smoke tests for `axt-port free`;
- keep command manuals, man pages, and `--print-schema` output aligned.
