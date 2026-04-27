# `axt` Foundation CLI Suite — Development Specification v2

**Status**: Ready for implementation.
**Audience**: Claude Code, Codex, Aider, or any coding agent that will implement this in steps. Also any human contributor.
**Format**: This document is the single source of truth. Implementation must not deviate without updating this spec first.

---

## 0. TL;DR for the implementing agent

Build a small suite of single-binary CLI tools, written in Rust, designed to be **agent-friendly** (low token cost, compact ACF output, stable schemas) and **human-friendly** (colored output, sensible defaults). Each binary is independently installable via Homebrew, Scoop, Cargo, GitHub Releases, or shell installer. They live together in one monorepo and share internal libraries.

The suite is named **`axt`** ("agent eXperience" / "axe", short and unique). All binaries are prefixed with `axt-` so they cluster under one namespace in `$PATH` and in package registries.

**Phase 1 deliverables** (the only thing to build first):
- `axt-peek` — directory & repo snapshot (replaces the old `snapx`).
- Shared library crates (`axt-core`, `axt-output`, `axt-fs`, `axt-git`).
- Full release pipeline producing GitHub Releases + Homebrew tap + Scoop bucket + Cargo + curl|sh installer.

**Phase 2+ deliverables** (after Phase 1 is shipping):
- `axt-run` — observable command execution.
- `axt-doc` — environment & toolchain doctor (merges old `whichx` + `envx`).
- `axt-drift` — filesystem diff from a marker (replaces the old `sincex`).

**Explicitly removed from scope** (with rationale): `colsx`, `fmetax`, `psx`. See section 3.

---

## 1. Why this exists, what it is, what it is not

### 1.1 The problem

Coding agents (Claude Code, Codex CLI, Gemini CLI, Aider, OpenCode, Droid, etc.) have become the largest new consumer of CLI tools in the last two years. The tools they call were designed for humans typing in a terminal. The mismatch is well documented:

- A frequently cited measurement is that roughly **a third of agent tokens are spent parsing CLI output**, not reasoning about code.
- CLI output is unstable across OS versions, locales, and tool versions.
- Pattern matching on free text breaks silently when format changes.
- Re-running commands to retrieve missed information wastes time and tokens.
- Many classic tools have no machine-readable mode at all (`ps`, `which`, `ls -l`, `df`, `mount`, `systemctl`).

Tools like `ripgrep`, `eza`, `fd`, `bottom`, `jc`, and `procs` partially address this. They are excellent tools but were not designed from the ground up around the assumption that an LLM will read the output. `axt` fills that gap: every command emits a stable, compact, schema-versioned representation specifically optimized for agent consumption, while remaining pleasant for humans.

### 1.2 Tesi del prodotto in una riga

> Coding agents need a small set of system primitives whose output is stable, compact, schema-versioned, and the same on Linux/macOS/Windows. `axt` is that set.

### 1.3 Out of scope

This is what `axt` is not, and will not become:

- A replacement for coreutils. We do not aim to rewrite `ls`, `cat`, `grep`, etc. Tools like `eza`, `bat`, `ripgrep` already do this well.
- An output-converter for legacy tools. `jc` (kellyjonbrazil/jc) already JSON-ifies the output of 100+ classic CLIs and is mature, distributed everywhere. We integrate with it, we do not duplicate it.
- An observability or APM platform. There is no daemon, no telemetry, no network communication.
- An AI client. `axt` does not call LLMs.
- A TUI. Outputs are streams, not interactive screens.

### 1.4 What "agent-friendly" actually means

This is the central design constraint. A command is agent-friendly when:

1. **One call returns enough information to make a decision.** Avoid forcing the agent to round-trip through several commands.
2. **Output is stable** across OS, locale, terminal width, tool versions, and time of day.
3. **Output is small.** Token cost is a first-class concern. Default limits are tight; verbose modes are explicit.
4. **Output is schema-versioned.** JSON/JSONL records carry `schema`; ACF declares it on the first line. Breaking changes bump the schema major.
5. **Errors are typed.** Codes like `path_not_found`, never `error: something went wrong`.
6. **Errors echo the input** that caused them, so the agent can construct a fix.
7. **Errors suggest a next command** when a useful one exists.
8. **Idempotent operations are preferred.** No "ensure exists, fail if exists" defaults.
9. **No interactive prompts**, ever, in non-TTY mode. No "are you sure?".
10. **Self-describing.** Every command has `--print-schema`, `--list-errors`, `--list-flags`.

Items 5–7 follow the conventions documented in the Anthropic and OpenAI guides for writing CLIs that agents use well; we adopt them as hard requirements.

---

## 2. Naming, identity, package strategy

### 2.1 The name

The suite is called **`axt`**. Pronounced like "axe tee" or read as "AX Tools". Three characters. Easy to type, easy to remember, and verified as available on crates.io for the root package, command crates, and internal crates. The repo is `github.com/<org>/axt`.

### 2.2 Binary names

Each binary is independently installable but visibly part of the same family:

| Binary | Phase | Replaces (old spec) | One-line purpose |
|---|---|---|---|
| `axt-peek` | 1 | `snapx` | Snapshot of a directory + repo + git + language metadata in one shot. |
| `axt-run` | 2 | `runx` | Run a command and produce a structured envelope: exit, duration, stdout/stderr summary, files changed. |
| `axt-doc` | 3 | `whichx` + `envx` | Diagnose the dev environment: PATH issues, version-manager conflicts, secret-like vars, missing dirs, broken symlinks. |
| `axt-drift` | 4 | `sincex` | Mark filesystem state, then later report what changed since the mark. Useful in CI, builds, and tests. |

There is no top-level `axt` router binary in v1. Each tool is a standalone binary. A router (`axt peek …`) may be added in v2 if user demand justifies it; designing for it now adds complexity without payoff.

Unprefixed command aliases (`peek`, `run`, `doc`, `drift`, `port`, `test`) are optional because most are generic names and several are already taken as crates.io packages. Cargo packages are always `axt-*`; installers may create aliases only on explicit user opt-in. Binary crates expose an `aliases` feature where practical, for example `cargo install axt-peek --features aliases` installs both `axt-peek` and `peek`.

### 2.3 What was removed and why

| Old name | Reason for removal |
|---|---|
| `colsx` | The market is fully served by `jc` (kellyjonbrazil/jc): 100+ parsers for legacy CLI output, mature, multi-platform, `brew install jc` works, it has streaming NDJSON parsers. Reinventing this is months of work for parity that already exists. The `axt` docs will explicitly recommend `jc` for this use case. |
| `fmetax` | The useful pieces (language, MIME, encoding, generated/not-generated) are folded into `axt-peek` per-file output. A standalone `fmetax` adds little. |
| `psx` | `procs`, `bottom`, `ps`, `pgrep` cover the human and most agent use cases. Cross-platform process introspection (especially cwd on Windows and macOS) is genuinely hard and the agent value is marginal compared to peek/run/doc. May return in v2 if a clear unmet need surfaces. |

### 2.4 Package availability check

A real check before publishing is required. From available data on crates.io (April 2026):
- `envx` is **taken** (a Rust env-variable manager) — confirmed why we abandoned it.
- `snapx`, `runx`, `peekx` were not visibly taken in searches, but the `axt-` prefix is the safer route and gives suite identity.
- The `axt-*` namespace appears clear in major registries.

Concrete pre-publish checklist (run by the maintainer before each new crate):
- `cargo search axt-peek` returns no exact match.
- `brew search axt-peek` returns no exact match.
- `scoop search axt-peek` (or check the bucket index) returns nothing conflicting.
- `npm view axt-peek` (for the optional npm-binary-wrapper distribution) returns 404.

### 2.5 Versioning policy

The suite ships under one synchronized version line (`axt 0.1.0`, `axt 0.2.0`, …). Within that:
- All **binary crates** (`axt-peek`, `axt-run`, …) share the same version number for clarity.
- All **internal library crates** (`axt-core`, `axt-output`, `axt-fs`, `axt-git`) are explicitly **non-public-API**: their version may bump in any release, even patch, to support binary changes. Their `README.md` and crate description state "internal use only, no stability guarantees". This avoids the diamond-dependency horror of trying to coordinate 16 independently-versioned crates.
- Public stability promises (CLI flags, JSON schemas, agent-mode schemas, exit codes) are version-tagged. Breaking changes to any of these require a major version bump of the affected binary.

---

## 3. Output modes — the contract

Every binary supports four primary output modes. These are the contract; everything else is implementation.

### 3.1 Human mode (default)

- TTY-aware: colors only when stdout is a TTY, `NO_COLOR` and `CLICOLOR_FORCE` and `FORCE_COLOR` honored.
- Designed to be skimmed, not parsed. No promises of stability across versions.
- Diagnostics on stderr; data on stdout.
- Width-aware: respects `COLUMNS` and terminal size, with sensible defaults when neither is set.
- Suggestions only when actionable.

### 3.2 JSON mode (`--json`)

- A single JSON document on stdout.
- Top-level envelope, always:

```json
{
  "schema": "axt.peek.v1",
  "ok": true,
  "data": { "...": "..." },
  "warnings": [],
  "errors": []
}
```

- `data` is the command-specific payload.
- `warnings` and `errors` are arrays of `{ "code": "...", "message": "...", "context": {...} }`. `code` is from the central error catalog (section 5).
- All timestamps are RFC 3339 UTC.
- All sizes in bytes (no `KB`/`KiB`).
- All durations in milliseconds.
- All paths are normalized (`/` separators on all platforms when the path is repo-relative; native separators only when the path is absolute and OS-specific).
- snake_case keys.
- The `schema` field is **versioned independently per command**: `axt.peek.v1`, `axt.run.v1`, etc. Breaking the JSON shape bumps it.

### 3.3 JSONL mode (`--jsonl`)

`--jsonl` emits newline-delimited JSON (NDJSON) for streaming, Unix pipelines, log collectors, `jq`, `duckdb`, `mlr`, and incremental processors.

The format:

- Each line is a single minified JSON object terminated by `\n`.
- Each object has `schema` and `type` fields.
- The first record is a summary record when the command emits a finite report.
- Record keys use the same snake_case names as JSON mode.
- No array wrapper.

`--jsonl` is not the primary agent format. It intentionally repeats keys per record because parser ergonomics and streaming are more important than token cost in this mode.

Example (`axt-peek . --jsonl`):

```json
{"schema":"axt.peek.summary.v1","type":"summary","ok":true,"root":".","files":42,"dirs":8,"bytes":381204,"git":"dirty","modified":5,"untracked":2,"truncated":false}
{"schema":"axt.peek.entry.v1","type":"file","path":"Cargo.toml","bytes":2102,"lang":"toml","git":"clean"}
{"schema":"axt.peek.entry.v1","type":"file","path":"src/main.rs","bytes":12003,"lang":"rust","git":"modified"}
{"schema":"axt.peek.entry.v1","type":"dir","path":"src","children":2,"git":"mixed"}
{"schema":"axt.peek.warn.v1","type":"warn","code":"truncated","reason":"max_records","shown":200,"total":1832,"hint":"--limit 1000"}
```

### 3.4 Agent mode (`--agent`) — ACF

`--agent` is the headline LLM-first format. It emits ACF, the Agent Compact Format: line-oriented text with schema declared once, no ANSI, no prose, stable keys, raw numeric values, and explicit truncation metadata.

ACF exists because NDJSON is excellent for parsers but wasteful for coding-agent context: a large uniform list repeats keys on every row. `axt` should emit RTK-like compact output at the source instead of relying on an external compressor.

Rules:

- The first line is always the summary/schema line.
- The first line includes `schema=...`, `ok=...`, `mode=records|table`, and `truncated=...`.
- Values are `key=value`; values containing whitespace, commas, quotes, or control characters are JSON-string quoted.
- Paths are relative when possible.
- Sizes are bytes, durations are milliseconds, timestamps are RFC 3339 UTC.
- No ANSI and no decorative prose.
- Important records come first.
- Large outputs use native semantic compression: filtering, grouping, deduplication, truncation, top-N relevance, and "show detail" hints.
- Uniform lists should use `mode=table`, declaring columns once with `cols=...`; heterogeneous results use `mode=records` with short record prefixes.

Example table output (`axt-peek . --agent`):

```text
schema=axt.peek.agent.v1 ok=true mode=table root=. cols=path,kind,bytes,lang,git rows=4 total=42 truncated=false
Cargo.toml,file,2102,toml,clean
README.md,file,8902,markdown,modified
src,dir,0,,mixed
src/main.rs,file,12003,rust,modified
```

Example records output (`axt-run --agent -- npm test`):

```text
schema=axt.run.agent.v1 ok=false mode=records cmd="npm test" exit=1 ms=12405 stdout_lines=842 stderr_lines=37 changed=5 saved=last truncated=false
X code=command_failed exit=1
E stream=stderr line=12 text="FAIL tests/checkout.test.ts"
E stream=stderr line=13 text="Error: expected 200, got 500"
F path=coverage/index.html action=created bytes=183204
F path=src/app.ts action=modified bytes=12003
S run="axt-run show last --stderr"
```

The summary line is sufficient for many agent decisions. Agents only consume detail lines when the task needs them.

#### 3.4.1 Agent table mode

`--agent-table` is reserved as a future shorthand for commands that naturally emit a single uniform table. It is equivalent to `--agent` with `mode=table`. Commands may choose table mode automatically under `--agent`; they must not invent per-command table flags.

#### 3.4.2 Shared ACF key dictionary

```
schema     schema identifier, usually axt.<command>.agent.v<N>
ok         bool, top-level success
mode       records|table
cols       comma-separated table columns
rows       emitted row count
total      total row count before truncation/filtering
truncated  bool, output was truncated
root       repo-relative root when relevant
path       repo-relative path when possible
kind       file|dir|symlink|other or command-specific entity kind
bytes      raw byte count
ms         milliseconds
ts         RFC 3339 UTC timestamp
lang       language (rust, python, ts, ...)
git        git status (clean|modified|untracked|added|deleted|renamed|mixed)
mime       mime type
enc        encoding (utf-8, utf-16le, ...)
nl         newline style (lf|crlf|mixed|none)
generated  bool, "looks generated"
code       error or warning code
hint       human-or-agent next-step suggestion
```

Record prefixes are one uppercase letter for ACF `mode=records`: `X` error, `W` warning, `E` stream/error line, `F` file, `S` suggestion, plus command-specific prefixes documented in `docs/commands/<cmd>.md`.

Each command's `docs/commands/<cmd>.md` lists command-specific keys and prefixes. The dictionary may grow but never shrink.

### 3.5 Plain mode (`--plain`)

Same as human mode but no color, no decorations, fixed simple layout. Useful for scripts that want human format without TTY heuristics.

### 3.6 Mode selection rules

- `--json`, `--jsonl`, `--agent`, and `--plain` are mutually exclusive. The CLI parser rejects multiple at parse time.
- If none is set, human is the default.
- `--json-data` exists as a shortcut that emits only `data` (unwrapped envelope), useful for piping to `jq`.
- `--agent-table` is reserved for a future uniform-table shorthand and must remain mutually exclusive with the other mode flags if implemented.
- Stdin tty-ness has no effect on mode.

---

## 4. Cross-platform support matrix

This is honest. Where a feature degrades or is unsupported on a platform, we say so up front instead of pretending parity.

| Capability | Linux | macOS | Windows | Notes |
|---|---|---|---|---|
| `axt-peek`: directory walking, sizes, mtime | ✅ full | ✅ full | ✅ full | |
| `axt-peek`: git status integration | ✅ full | ✅ full | ✅ full | via `gix` |
| `axt-peek`: language detection | ✅ full | ✅ full | ✅ full | extension + magic bytes |
| `axt-peek`: hidden file handling | dotfiles | dotfiles | dotfiles + NTFS hidden attribute | |
| `axt-peek`: symlink loop detection | ✅ | ✅ | ✅ partial | NTFS junctions handled best-effort |
| `axt-peek`: case-sensitive paths | ✅ | ⚠️ usually case-insensitive FS | ⚠️ case-insensitive FS | We preserve the FS-reported case |
| `axt-run`: spawn, capture, exit code | ✅ | ✅ | ✅ | |
| `axt-run`: process group kill on timeout | ✅ via setsid/setpgid | ✅ | ✅ via Job Objects | |
| `axt-run`: shell mode (`--shell`) | `$SHELL -lc` | `$SHELL -lc` | `cmd /C` or `pwsh -c` | Default off; agents should not use shell |
| `axt-run`: file change snapshot | ✅ | ✅ | ✅ | metadata-based |
| `axt-doc`: PATH duplicate / missing detection | ✅ | ✅ | ✅ | PATHEXT honored on Windows |
| `axt-doc`: version manager detection | mise, asdf, rustup, cargo, pyenv, rbenv, volta, nvm-shim | + Homebrew | + Scoop, Chocolatey, winget | best-effort, opt-in version probe |
| `axt-doc`: env secret-like detection | ✅ | ✅ | ✅ | |
| `axt-drift`: snapshot+diff (metadata) | ✅ | ✅ | ✅ | |
| `axt-drift`: snapshot+diff (with hash) | ✅ | ✅ | ✅ | slower, opt-in |
| Process introspection (cwd of pid) | ✅ via `/proc` | ⚠️ private API, best-effort | ❌ requires elevated privileges | Reason `psx` is deferred |

If a user invokes a feature on a platform where it does not work, we exit with code 9 (`feature_unsupported`) and a clear error code. We never silently fail.

---

## 5. Standard error catalog

A central catalog of error codes used across all commands. Stable across versions; new codes are additive.

| Code | Exit | Meaning | Retryable |
|---|---|---|---|
| `ok` | 0 | Success | n/a |
| `runtime_error` | 1 | Generic runtime failure | maybe |
| `usage_error` | 2 | CLI argument or flag invalid | no |
| `path_not_found` | 3 | A required path does not exist | no |
| `permission_denied` | 4 | Insufficient permissions | no |
| `timeout` | 5 | Operation exceeded `--timeout` | yes |
| `output_truncated_strict` | 6 | `--strict` and output had to be truncated | no |
| `interrupted` | 7 | SIGINT / Ctrl-C received | no |
| `io_error` | 8 | Filesystem or stream IO failure | maybe |
| `feature_unsupported` | 9 | Feature unavailable on this platform | no |
| `schema_violation` | 10 | Internal: produced data violated its own schema | no — bug |
| `command_failed` | 11 | (`axt-run` only) wrapped command exited non-zero | depends |
| `git_unavailable` | 12 | Git repo expected but not found / not readable | no |
| `config_error` | 13 | User config file malformed | no |
| `network_disabled` | 14 | An offline command attempted network. We never do this — defensive | no |

Each error in JSON / agent output also carries a `context` object with the relevant input that caused the failure (path, pid, command, etc.). This is the agent-friendly "echo the failing input" rule.

`--list-errors` on every binary prints the full catalog as JSONL, so an agent or script can discover the surface programmatically.

---

## 6. Repository layout

```
axt/
├── Cargo.toml                  # workspace root
├── Cargo.lock
├── README.md                   # top-level pitch + install matrix
├── LICENSE-MIT
├── LICENSE-APACHE
├── CHANGELOG.md
├── CONTRIBUTING.md
├── SECURITY.md
├── dist-workspace.toml         # cargo-dist config
├── docs/
│   ├── architecture.md
│   ├── agent-mode.md           # the ACF contract + key dictionary
│   ├── error-catalog.md        # auto-generated, source of truth in code
│   ├── installation.md         # full per-platform install matrix
│   ├── design-principles.md
│   ├── release.md              # the release runbook
│   └── commands/
│       ├── peek.md             # axt-peek (Phase 1)
│       ├── run.md              # axt-run  (Phase 2)
│       ├── doc.md              # axt-doc  (Phase 3)
│       └── drift.md            # axt-drift(Phase 4)
├── crates/
│   ├── axt-core/                # shared: errors, exit codes, paths, time, limits, terminal detection
│   ├── axt-output/               # shared: human/json/agent renderers, truncation
│   ├── axt-fs/                  # shared: walking, ignore, metadata, classification, hashing
│   ├── axt-git/                 # shared: gix wrapper, status, branch, dirty
│   ├── axt-peek/                # bin
│   ├── axt-run/                 # bin (Phase 2)
│   ├── axt-doc/                 # bin (Phase 3)
│   └── axt-drift/               # bin (Phase 4)
├── schemas/
│   ├── axt.peek.v1.schema.json  # JSON Schema for envelope
│   ├── axt.peek.summary.v1.schema.json
│   ├── axt.peek.entry.v1.schema.json
│   ├── axt.run.v1.schema.json
│   └── ...
├── fixtures/
│   ├── fs-small/               # deterministic small tree
│   ├── fs-with-git/            # tree + .git
│   ├── fs-monorepo/            # tree + multi-package
│   └── runs/                   # captured command outputs for axt-run tests
├── xtask/
│   └── src/main.rs             # bench, schema-gen, error-doc-gen
└── .github/
    └── workflows/
        ├── ci.yml
        ├── release.yml         # cargo-dist generated
        ├── audit.yml           # cargo-audit + cargo-deny
        └── coverage.yml
```

A per-project user config is supported at `<project>/.axt/config.toml`; a per-user global config at `${XDG_CONFIG_HOME:-~/.config}/axt/config.toml` on Unix and `%APPDATA%\axt\config.toml` on Windows. CLI flags > project config > user config > built-in defaults.

---

## 7. Workspace `Cargo.toml`

```toml
[workspace]
resolver = "2"
members = [
  "crates/axt-core",
  "crates/axt-output",
  "crates/axt-fs",
  "crates/axt-git",
  "crates/axt-peek",
  # phases 2-4 added later:
  # "crates/axt-run",
  # "crates/axt-doc",
  # "crates/axt-drift",
  "xtask",
]

[workspace.package]
edition = "2021"
license = "MIT OR Apache-2.0"
rust-version = "1.78"
repository = "https://github.com/<org>/axt"
homepage  = "https://github.com/<org>/axt"
authors = ["..."]

[workspace.dependencies]
anyhow = "1"
thiserror = "1"
clap = { version = "4", features = ["derive", "env", "wrap_help"] }
serde = { version = "1", features = ["derive"] }
serde_json = { version = "1", features = ["preserve_order"] }
time = { version = "0.3", features = ["formatting", "parsing", "serde"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
anstream = "0.6"
anstyle = "1"
ignore = "0.4"
walkdir = "2"
jwalk = "0.8"
gix = { version = "0.77", default-features = false, features = ["max-performance-safe"] }
blake3 = "1"
content_inspector = "0.2"
infer = "0.16"
mime_guess = "2"
encoding_rs = "0.8"
which = "7"
camino = { version = "1", features = ["serde1"] }
dunce = "1"
pathdiff = "0.2"
rayon = "1"
tempfile = "3"
schemars = "0.8"
jsonschema = "0.18"

# tests
assert_cmd = "2"
predicates = "3"
insta = { version = "1", features = ["json", "yaml"] }
```

Notes:
- `gix` is configured with `default-features = false` and `max-performance-safe`. We never shell out to `git`. If a feature is missing in `gix`, document it as missing rather than fall back to subprocess.
- `serde_json` with `preserve_order` ensures stable JSON key ordering — useful for snapshot tests.
- The `xtask` pattern keeps CI-only tools (schema generation, benchmark scaffolding, error-doc generation) out of the published binary builds.

---

## 8. The shared library crates

These crates are **internal**. Their `README.md` says so explicitly. No public API stability promise.

### 8.1 `axt-core`

Owns: errors, exit codes, paths, time, limits, terminal detection, config discovery, common CLI flags trait.

Key types (every binary imports these):

```rust
pub enum OutputMode { Human, Json, JsonData, Jsonl, Agent, Plain }

pub struct OutputLimits {
    pub max_records: usize,        // default 200
    pub max_bytes: usize,          // default 65_536
    pub strict: bool,              // exit non-zero on truncation
}

pub enum ErrorCode {               // matches section 5 catalog exactly
    Ok, RuntimeError, UsageError, PathNotFound, /* ... */
}

pub struct CommandContext {
    pub cwd: camino::Utf8PathBuf,
    pub mode: OutputMode,
    pub limits: OutputLimits,
    pub color: ColorChoice,
    pub config: ResolvedConfig,
    pub clock: Box<dyn Clock>,     // for deterministic tests
}
```

The `Clock` trait is mandatory. Every place that records `ts` uses it. Tests inject a fixed-time clock so snapshots are reproducible.

### 8.2 `axt-output`

The renderer trait every command implements:

```rust
pub trait Renderable {
    fn render_human(&self, w: &mut dyn Write, ctx: &RenderContext) -> Result<()>;
    fn render_json(&self,  w: &mut dyn Write, ctx: &RenderContext) -> Result<()>;
    fn render_jsonl(&self, w: &mut dyn Write, ctx: &RenderContext) -> Result<()>;
    fn render_agent(&self, w: &mut dyn Write, ctx: &RenderContext) -> Result<()>;
}
```

Concrete shared helpers:
- `JsonEnvelope::new(schema, data, warnings, errors)` — produces the standard top-level envelope.
- `JsonlWriter` — writes one minified JSON object per line, enforces `--max-bytes` and `--limit`, emits a `truncated` warn record at the end if either is hit.
- `AgentCompactWriter` — writes ACF lines, enforces `--max-bytes` and `--limit`, emits a compact `W code=truncated ...` record at the end if either is hit.
- Color and styling helpers using `anstyle` + `anstream`, with TTY detection and `NO_COLOR` / `CLICOLOR_FORCE` / `FORCE_COLOR` honored in that precedence order.

### 8.3 `axt-fs`

Owns directory walking and per-file classification.

- Walker built on `ignore::WalkBuilder`, with `.gitignore` and `.ignore` honored by default; `--no-ignore` disables.
- Per-file metadata: size, mtime, kind (file/dir/symlink/other), is_executable, language guess, mime guess, text-vs-binary, encoding (utf-8/utf-16/latin1/unknown), newline style (LF/CRLF/mixed/none), generated-likely heuristic.
- Symlink policy: do not follow by default; `--follow-symlinks` opts in. Loops detected by tracking inode pairs.
- The "generated" heuristic combines: path contains `dist/`, `build/`, `target/`, `out/`, `node_modules/`, `vendor/`, `.next/`, `coverage/`; first 200 bytes contain a "generated" / "do not edit" marker; minified JS heuristic (long lines, low whitespace ratio). Heuristic, not authoritative.
- Hashing is opt-in (`--hash blake3`), `blake3` only for v1; sha256 in v2 if requested.

### 8.4 `axt-git`

Thin `gix` wrapper:
- `repo_root_for(path)` → `Option<RepoHandle>`.
- `status_for(repo, path)` → `clean | modified | untracked | added | deleted | renamed | mixed`.
- `current_branch(repo)`.
- `dirty_count(repo)` → `(modified, untracked)`.
- `diff_paths(repo, ref_a, ref_b)` for the `--changed-since <REF>` use case.

If a path is outside any git repo, all functions return graceful "no git". We never throw on absence of git.

---

## 9. Phase 1 — `axt-peek`

`axt-peek` is the first deliverable, end to end. Build it, ship it, get user feedback, then move to Phase 2.

### 9.1 Purpose

Produce a single compact answer to "what is in this directory and what is its current state?" In one call, return:
- The list of files and (optionally) directories within a depth bound.
- For each file: size, language, git status, mtime.
- A summary block: total files, dirs, bytes, git state (clean/dirty/none), counts of modified/untracked/ignored.
- Optional: BLAKE3 hash (opt-in, off by default), staged-vs-unstaged breakdown, changed-since-ref.

### 9.2 Non-goals

- Not a full `find` replacement.
- No file content (only metadata).
- No full-text search — that is `ripgrep`.
- No tree-drawing ASCII art in agent/JSON modes (only in human mode).

### 9.3 CLI surface

```
axt-peek [PATHS]...                  # default: "."
  --depth <N>                       # default: 2
  --files-only
  --dirs-only
  --include-hidden
  --no-ignore                       # do not respect .gitignore
  --git / --no-git                  # default: auto-detect
  --changed                         # only files with non-clean git status
  --changed-since <REF>             # files differing from REF (commit/branch/tag)
  --type <KIND>                     # text|binary|image|archive|code|config|data
  --lang <LANG>                     # rust|ts|python|...
  --hash none|blake3                # default: none
  --summary-only
  --sort name|size|mtime|git|type
  --reverse
  --max-file-size <SIZE>            # skip files larger than this
  --follow-symlinks
  --cross-fs                        # cross filesystem boundaries
  --json
  --jsonl
  --agent
  --json-data
  --plain
  --color auto|always|never
  --limit <N>                       # default 200
  --max-bytes <SIZE>                # default 64KiB
  --strict
  --quiet
  --verbose, -v
  --print-schema human|json|jsonl|agent
  --list-errors
  --version
  --help
```

### 9.4 Output: human mode

```
./
  Cargo.toml          2.1 KB  toml       clean
  README.md           8.9 KB  markdown   modified
  src/
    main.rs          12.0 KB  rust       modified
    cli.rs            4.2 KB  rust       clean
  tests/
    cli.rs            7.1 KB  rust       untracked

Summary
  files     42         modified   5
  dirs       8         untracked  2
  bytes    381.2 KB    ignored  138
  git      dirty       truncated  no
```

### 9.5 Output: agent mode (ACF)

```text
schema=axt.peek.agent.v1 ok=true mode=table root=. cols=path,kind,bytes,lang,git,mtime rows=4 total=42 truncated=false
Cargo.toml,file,2102,toml,clean,2026-04-26T18:02:11Z
README.md,file,8902,markdown,modified,2026-04-27T08:14:00Z
src,dir,0,,mixed,
src/main.rs,file,12003,rust,modified,2026-04-27T09:01:22Z
```

When `--limit` or `--max-bytes` triggers truncation, the last record is:

```text
W code=truncated reason=max_records shown=200 total=1832 hint="--limit 1000"
```

### 9.6 Output: JSONL mode

```json
{"schema":"axt.peek.summary.v1","type":"summary","ok":true,"root":".","files":42,"dirs":8,"bytes":381204,"git":"dirty","modified":5,"untracked":2,"ignored":138,"truncated":false}
{"schema":"axt.peek.entry.v1","type":"file","path":"Cargo.toml","bytes":2102,"lang":"toml","git":"clean","mtime":"2026-04-26T18:02:11Z"}
{"schema":"axt.peek.entry.v1","type":"file","path":"README.md","bytes":8902,"lang":"markdown","git":"modified","mtime":"2026-04-27T08:14:00Z"}
{"schema":"axt.peek.entry.v1","type":"dir","path":"src","children":2,"git":"mixed"}
{"schema":"axt.peek.entry.v1","type":"file","path":"src/main.rs","bytes":12003,"lang":"rust","git":"modified","mtime":"2026-04-27T09:01:22Z"}
```

### 9.7 Output: JSON mode

```json
{
  "schema": "axt.peek.v1",
  "ok": true,
  "data": {
    "root": ".",
    "summary": {
      "files": 42, "dirs": 8, "bytes": 381204,
      "git_state": "dirty",
      "modified": 5, "untracked": 2, "ignored": 138,
      "truncated": false
    },
    "entries": [
      {
        "path": "src/main.rs",
        "kind": "file",
        "bytes": 12003,
        "language": "rust",
        "mime": "text/x-rust",
        "encoding": "utf-8",
        "newline": "lf",
        "is_generated": false,
        "git": "modified",
        "mtime": "2026-04-27T09:01:22Z",
        "hash": null
      }
    ]
  },
  "warnings": [],
  "errors": []
}
```

### 9.8 Internal architecture

```
crates/axt-peek/src/
├── main.rs              # tiny: parse args, build context, run, exit
├── cli.rs               # clap derive, flag validation
├── command.rs           # orchestrates collect → render
├── model.rs             # PeekData, Entry, Summary
├── collect.rs           # uses axt-fs and axt-git to populate model
├── render.rs            # impl Renderable for PeekData
└── error.rs
```

Algorithm:
1. Resolve and canonicalize input paths.
2. For each root, detect git repo (cached so monorepo passes are cheap).
3. Build an `ignore::WalkBuilder` configured with current flags.
4. Walk in parallel with `rayon` for classification, single-thread for ordering.
5. For each entry, compute metadata via `axt-fs`, status via `axt-git`.
6. Apply filters (`--changed`, `--type`, `--lang`).
7. Sort.
8. Apply `--limit` and `--max-bytes`. The renderer decides when to truncate based on the running byte counter.
9. Render via the appropriate `Renderable` method.

### 9.9 Performance targets

Baseline hardware: a mid-2020s laptop (M-class Apple Silicon or comparable x86_64 with NVMe SSD), filesystem APFS or ext4, fixture pre-warmed in pagecache. We commit to:

- **Small tree** (≤ 200 files, depth 2): p50 < 50 ms cold, < 15 ms warm.
- **Medium tree** (10 000 files, depth 3): p50 < 300 ms warm, hashing off.
- **Large tree** (100 000 files): completes within `--max-bytes` or returns a `truncated` warn record; never OOMs.

Numbers are reproducible via `cargo xtask bench` against a generated fixture. CI includes a coarse perf regression check (10× slowdown is a fail; we do not gate on tighter bounds because CI runners vary).

### 9.10 Edge cases the implementation must handle

- Symlink loops: detected via inode-pair tracking; first repeat aborts the branch and emits a warn record.
- Permission denied on a subtree: emit a warn record, continue walking the rest.
- Non-UTF-8 paths on Unix: render in human mode with lossy UTF-8; in JSON/agent, use the standard `serde_json` lossless escaping.
- Windows reserved names (`CON`, `NUL`, `AUX`, etc.): treat as regular files, no special handling.
- Huge git repos (linux-kernel scale): `--no-git` is the escape hatch and must be fast; with `--git`, we cap operations within `--max-bytes` and emit a warn.
- Files with mtime in the future: keep the value, do not normalize.
- Submodules: treated as directories with `g: "mixed"`; we do not recurse into the submodule's git status by default.

### 9.11 Definition of done for `axt-peek` v0.1

All of the following must be true to ship v0.1:

1. Binary builds and runs on Linux (x86_64, aarch64), macOS (x86_64, aarch64), Windows (x86_64). Aarch64 Windows is best-effort.
2. `axt-peek --version`, `axt-peek --help`, `axt-peek --print-schema agent`, `axt-peek --list-errors` all work without flags.
3. Human, JSON, JSONL, agent, and plain modes all produce output for every fixture.
4. Snapshot tests via `insta` cover human and agent for: small tree, tree with git, tree with `.gitignore`, empty dir, missing path, permission-denied dir, depth=0, depth=10, `--changed`, `--changed-since HEAD`, `--summary-only`.
5. JSON output validates against `schemas/axt.peek.v1.schema.json` for every test case (test runs `jsonschema` on output).
6. JSONL output: every line individually validates as a JSON object; first record always has `type:"summary"`; the `schema` field is present on every record.
7. Agent output follows ACF: first line has `schema`, `ok`, `mode`, and `truncated`; table outputs declare `cols` once; truncation emits a compact `W code=truncated` record.
8. `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` all pass on all three OSes in CI.
9. The release pipeline (section 12) successfully publishes a draft release for a tag like `v0.1.0-rc1` to GitHub, including a Homebrew tap update, a Scoop manifest update, and a `cargo publish --dry-run` for `axt-peek` (and dependencies).
10. `docs/commands/peek.md` is written and matches the implementation. Every flag is documented. Every error code is listed. Every key in agent output is in the dictionary.

---

## 10. Phase 2 — `axt-run`

Deferred until `axt-peek` v0.1 ships. Everything below is the spec to implement when Phase 2 starts.

### 10.1 Purpose

Run a command and emit a structured envelope of what happened: exit code, duration, stdout/stderr summary (with file storage of full streams), files changed in cwd during the run, and saved artifacts. This collapses the agent pattern of `time cmd > out.log 2>&1; echo $?; git status` into one call.

### 10.2 CLI surface

```
axt-run [OPTIONS] -- <COMMAND> [ARGS]...
axt-run show [<NAME>|last]
axt-run list
axt-run clean [--older-than <DURATION>]

  --save <NAME>                      # name this run (default: timestamp slug)
  --no-save                          # do not persist artifacts
  --cwd <DIR>
  --env KEY=VALUE                    # repeatable
  --env-file <FILE>                  # simple .env loader
  --timeout <DURATION>               # e.g. 30s, 5m
  --capture always|never|auto        # default: auto
  --max-log-bytes <SIZE>             # per stream, default 5MiB
  --watch-files / --no-watch-files   # default on inside cwd
  --include <GLOB>                   # for --watch-files filter
  --exclude <GLOB>
  --shell                            # run via $SHELL or cmd /C; default OFF
  --summary-only
  --tail-bytes <N>                   # show last N bytes of stderr in agent output
  + standard --json/--jsonl/--agent/--plain/--limit/--max-bytes/--strict
```

### 10.3 Agent output (ACF, illustrative)

```text
schema=axt.run.agent.v1 ok=false mode=records cmd="npm test" exit=1 ms=12405 stdout_lines=842 stderr_lines=37 changed=5 saved=last truncated=false
X code=command_failed exit=1
E stream=stderr line=12 text="FAIL tests/checkout.test.ts"
E stream=stderr line=13 text="Error: expected 200, got 500"
F path=coverage/index.html action=created bytes=183204
F path=src/app.ts action=modified bytes=12003
S run="axt-run show last --stderr"
```

### 10.4 Storage layout

```
.axt/runs/2026-04-27T10-12-44Z-npm-test/
├── meta.json
├── stdout.log         # full stdout, truncated only if --max-log-bytes hit
├── stderr.log
├── changed.json
└── summary.agent.acf
```

`.axt/` is added to `.gitignore` only on user opt-in (printed suggestion, never auto-modify). `axt-run clean --older-than 7d` is the GC. Default retention is 30 days; configurable in `.axt/config.toml`.

### 10.5 Implementation notes

- Use `tokio::process::Command` for async streams + timeout. No `duct`.
- Stream both stdout and stderr to disk and to a ring buffer simultaneously, so the summary is available without re-reading the file.
- File-change detection uses size + mtime + (when available) inode. `--hash` flag for stricter detection at cost.
- On Unix, set `setpgid` so timeout sends SIGTERM to the group then SIGKILL. On Windows, use a Job Object.
- Never inherit our own tty; allocate a pipe so capture is reliable. (We do not animate progress bars; agents do not need them.)
- `--shell` is opt-in because shell quoting bugs are the #1 source of agent confusion.

### 10.6 Definition of done for `axt-run` v0.2

Same shape as 9.10 but for runs: snapshot tests with deterministic fixture commands (`echo`, `false`, `sleep 0.1`, a script that creates/modifies/deletes files), timeout test, environment-passing test, retention/GC test, cross-platform test.

---

## 11. Phase 3 & 4 — `axt-doc` and `axt-drift`

Specs below are concise; full design is finalized when each phase begins. The shape, however, is committed.

### 11.1 `axt-doc` (replaces old `whichx` + `envx`)

A single command that diagnoses the local dev environment.

**Subcommands**:
- `axt-doc which <CMD>` — what does this command resolve to, all matches in PATH, version-manager attribution, version probe with timeout.
- `axt-doc path` — PATH analysis: duplicates, missing dirs, broken symlinks, ordering issues.
- `axt-doc env` — environment summary: var count, secret-like vars (redacted), suspicious or empty vars.
- `axt-doc all` — runs all three and emits a single combined response in the selected mode; `--jsonl` is the streaming JSONL form.

Manager detection (best-effort, by path patterns + by querying the manager when present): Homebrew, mise, asdf, rustup, cargo bin, pyenv, rbenv, volta, nvm-shim, Scoop, Chocolatey, winget.

Secret detection rule (case-insensitive): name matches one of `*_TOKEN`, `*_SECRET*`, `*_KEY`, `*_PASSWORD`, `PASS`, `*_CREDENTIAL*`, `*_PRIVATE*`, `*_AUTH*`. Values are never printed unless `--show-secrets` is passed, which prints a stderr warning regardless of mode.

### 11.2 `axt-drift` (replaces old `sincex`)

```
axt-drift mark [--name <NAME>]
axt-drift diff [--since <NAME>]
axt-drift run [--name <NAME>] -- <CMD>
axt-drift list
axt-drift reset
```

Snapshot stored at `.axt/drift/<NAME>.jsonl`. Each record is a file's `(path, size, mtime, optionally hash)`. Diff produces created/modified/deleted, sorted by size delta.

This is the build-verification primitive: "I ran `npm run build`; what files appeared?" Done well, this saves agents hundreds of tokens vs. parsing tool-specific output.

---

## 12. Distribution pipeline

This is what gets `axt` into users' hands. Day-1 priorities (per your decision): GitHub Releases + Homebrew + Cargo + Scoop + curl|sh installer (and PowerShell for Windows where reasonable).

### 12.1 Tooling

- **`cargo-dist`** (now branded `dist`) is the spine. It generates the GitHub Actions release workflow, builds prebuilt binaries for the configured targets, attaches them to the GitHub Release, generates a Homebrew formula and pushes it to a tap, generates the curl|sh and PowerShell installers, and optionally publishes to crates.io.
- **Scoop** is not natively supported by `cargo-dist`; we maintain a tiny `scripts/release/scoop-manifest.py` (Python or Rust, your call) that templates `bucket/axt-peek.json` from the released archive metadata, and a separate repo `<org>/scoop-axt` that hosts the bucket. The release workflow runs the script and pushes the manifest update via `peter-evans/create-pull-request`.
- **`cargo-binstall`** support is automatic when the Cargo.toml `repository` field is set and the GitHub Releases follow the `cargo-dist` naming convention. Documented but not gated.

### 12.2 Targets

| Target triple | Tier | Notes |
|---|---|---|
| `x86_64-unknown-linux-gnu` | tier-1 | gnu, glibc 2.31+ |
| `x86_64-unknown-linux-musl` | tier-1 | static, alpine-friendly |
| `aarch64-unknown-linux-gnu` | tier-1 | via cargo-zigbuild |
| `x86_64-apple-darwin` | tier-1 | |
| `aarch64-apple-darwin` | tier-1 | |
| `x86_64-pc-windows-msvc` | tier-1 | |
| `aarch64-pc-windows-msvc` | tier-2 | best-effort |

Tier-1 targets gate every release. Tier-2 targets do not block.

### 12.3 Channels

For each binary (e.g., `axt-peek`):

| Channel | Format | OS | Mechanism |
|---|---|---|---|
| GitHub Release | `.tar.xz`, `.zip` | All | `cargo-dist` |
| `curl https://.../install.sh \| sh` | shell installer | Linux, macOS | `cargo-dist` |
| `irm https://.../install.ps1 \| iex` | PS installer | Windows | `cargo-dist` |
| Homebrew | formula in `<org>/homebrew-axt` | macOS, Linux | `cargo-dist` |
| Scoop | manifest in `<org>/scoop-axt` | Windows | custom script |
| Cargo | `cargo install axt-peek` | All | `cargo publish` |
| binstall | from GitHub Release | All | automatic via `cargo-binstall` |

Day-2 (post-v0.1) channels we intend to add: APT/Debian (`cargo-deb`), RPM (`cargo-generate-rpm`), AUR (`cargo-aur`), Nixpkgs PR, npm wrapper package (downloads the right binary in `postinstall`). These are explicitly **not** required to ship v0.1.

### 12.4 Release runbook (`docs/release.md`)

A typed, tested checklist. The skeleton:

1. `cargo dist plan` locally — ensures the workspace and dist config are coherent.
2. `cargo test --workspace --all-features` on all three OSes via CI matrix (passes already in CI; this is to confirm the tag is releasable).
3. Bump version in `Cargo.toml` for the binary crates and any internal lib that needs it (use `cargo-release` or manual; document the choice).
4. Update `CHANGELOG.md` (Keep-a-Changelog format).
5. Tag: `git tag v0.1.0 -s -m "axt v0.1.0"`. Push tag.
6. The release workflow runs, builds all targets, uploads artifacts, opens PRs against the Homebrew tap, runs the Scoop manifest script, and (if configured) publishes to crates.io.
7. Smoke test from a clean machine: shell installer on Linux, brew on macOS, scoop on Windows. Each should install and run `--version` successfully.
8. Mark the GitHub Release as "Latest" (the workflow drafts; a human promotes).

If smoke-test step 7 fails, yank the release: delete the tag, mark the GitHub Release as draft, run `cargo yank` for any crates already published.

### 12.5 Signing & supply chain

- GitHub Artifact Attestations enabled (cargo-dist supports this since 0.31).
- `cargo-audit` and `cargo-deny` run in CI; advisories fail the build.
- `cargo-cyclonedx` produces an SBOM attached to each release.
- No `postinstall` scripts that fetch arbitrary data. The npm wrapper, when added, downloads only from GitHub Releases of this repo, with checksum verification.
- The Scoop manifest includes an `hash` field with SHA-256 of the published archive.

### 12.6 Why this matters for an agent-focused tool

The 2025-2026 supply chain attacks on coding-agent CLI packages (Cline 2.3.0, Codex CLI CVE-2025-61260, others) make it explicit: any tool an agent invokes can become an attack vector. Pinning the artifact attestation, signing releases, and shipping an SBOM are not optional — they are the price of being trusted by autonomous tooling.

---

## 13. Quality gates

### 13.1 Lint and style

Workspace-level:

```rust
#![deny(unsafe_code)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
```

`unwrap()` and `expect()` are forbidden in non-test code. Library code uses `thiserror` typed errors. Binary code may use `anyhow` only at the top edge (`main`).

`cargo fmt` enforced. `cargo clippy --workspace --all-targets -- -D warnings` is a CI gate.

### 13.2 Testing

- **Unit tests** for every parser, classifier, renderer.
- **Integration tests** with `assert_cmd` for full CLI invocations.
- **Snapshot tests** with `insta` for human, JSONL, and agent output. Snapshots are committed.
- **JSON schema validation** in tests: every JSON output is validated against the published schema.
- **JSONL validation**: every output line is validated as an individual JSON object, schemas matched by the `schema` field.
- **ACF validation**: first line has `schema`, `ok`, `mode`, and `truncated`; table mode declares columns once; record mode uses documented prefixes.
- **Cross-platform CI**: Ubuntu, macOS, Windows matrix on every PR.
- **Determinism**: tests use a fixed `Clock` and a fixed-tree fixture. No real-world dates or paths.
- **Property tests** (via `proptest`) for the agent-mode escape logic and any text parsers we ship.

### 13.3 Performance

- `cargo xtask bench` produces a JSON report.
- CI runs a single coarse perf check: 10× regression vs. the committed baseline fails the build. Tighter budgets are not enforced because runners vary.
- Fixtures for benches are generated, not committed (committing 100k-file trees bloats the repo).

### 13.4 Documentation

- Every flag in `--help` has a one-liner. Long help (`--help`) gives examples.
- Every command has a `docs/commands/<cmd>.md` file with: purpose, examples, human/JSON/JSONL/agent output samples, full flag list, error codes, performance, cross-platform notes, agent-usage guide.
- `docs/agent-mode.md` is the canonical reference for ACF and the shared agent key/prefix dictionary.
- `docs/error-catalog.md` is auto-generated from the code at release time (xtask).

---

## 14. The implementation plan, ordered

This is the only section the implementing agent needs to follow strictly. Each milestone is small and ends in a shipped, useful state.

### Milestone 0 — workspace scaffolding (target: 1–2 days)

1. Create the monorepo skeleton from section 6.
2. Workspace `Cargo.toml` from section 7.
3. Empty crate stubs for `axt-core`, `axt-output`, `axt-fs`, `axt-git`, `axt-peek` (and `xtask`).
4. CI workflow: matrix Linux/macOS/Windows; runs fmt, clippy, test on stable Rust.
5. License files, README, CONTRIBUTING.
6. `cargo dist init --ci=github --installer shell --installer powershell --installer homebrew` and commit the generated files. Configure tier-1 targets only at this stage.

**Done when**: pushing to `main` produces a green CI run; tagging `v0.0.1` produces a draft GitHub Release with placeholder binaries.

### Milestone 1 — `axt-core` and `axt-output` foundations (3–5 days)

1. Implement `ErrorCode` enum, exit-code mapping, and the standard error catalog as `pub const` data.
2. Implement `OutputMode` parsing and conflict detection in clap.
3. Implement `JsonEnvelope`, `JsonlWriter`, and `AgentCompactWriter`.
4. Implement `Clock` trait + `SystemClock` + `FixedClock` (test-only).
5. Implement TTY detection and color choice resolution honoring `NO_COLOR`, `CLICOLOR_FORCE`, `FORCE_COLOR`.
6. Implement `--print-schema` and `--list-errors` shared flags via a small derive helper or trait.
7. Tests for everything above.

**Done when**: a "hello world" binary built on this scaffolding can emit the same hello-world data in human, JSON, JSONL, and agent modes, passes snapshot tests, and validates its JSON output against a tiny schema.

### Milestone 2 — `axt-fs` and `axt-git` (5–7 days)

1. Walker on `ignore::WalkBuilder` with all the flags peek will need.
2. Per-file metadata extraction.
3. Language detection (extension table + fallback to `infer`).
4. Generated heuristic.
5. `gix` wrapper: repo detection, status per path (cached for monorepos), branch.
6. Tests against committed fixtures (`fixtures/fs-small`, `fixtures/fs-with-git`).

**Done when**: a unit test can build a `Vec<EntryMetadata>` for a fixture in <50 ms and the data is byte-identical run-to-run.

### Milestone 3 — `axt-peek` MVP (5–7 days)

1. Wire CLI per section 9.3 (flags, mutual exclusion, defaults).
2. Implement `model::PeekData`, `Entry`, `Summary`.
3. Implement `collect.rs` using `axt-fs` + `axt-git`.
4. Implement `Renderable` for human, JSON, agent.
5. Snapshot tests for each mode against each fixture.
6. JSON schema generation via `schemars`; commit `schemas/axt.peek.v1.schema.json`; validate every test output against it.
7. Write `docs/commands/peek.md`.

**Done when**: every item of section 9.10 (Done criteria) is true.

### Milestone 4 — release pipeline shakedown (1–3 days)

1. Tag `v0.1.0-rc1`. Watch the workflow run end to end.
2. Smoke test installs on a Linux VM (curl|sh), a Mac (brew), a Windows VM (scoop and irm|iex).
3. `cargo install axt-peek` from a fresh `cargo` install works.
4. Fix anything that breaks. Iterate until clean.
5. Tag `v0.1.0`. Promote the release. Announce.

**Done when**: any developer in the world can run one of the documented install commands and have `axt-peek` working in 30 seconds.

### Milestone 5+ — phases 2, 3, 4

Each phase follows the same pattern: flesh out the spec section here, scaffold the binary crate, implement, snapshot-test, document, ship. No phase begins until the previous phase has shipped a release and gathered at least a week of real use.

---

## 15. Decisions explicitly deferred

To prevent scope creep, these are decisions to **not** make in v0.x:

- A top-level `axt` router binary. Each tool is its own binary; if users keep asking for `axt peek`, add it in v1.0.
- A plugin system.
- Network-dependent features.
- Shell completions beyond `clap_complete` defaults (which we generate, but do not heavily customize).
- Configuration via remote URLs.
- An MCP server wrapper. Agents already invoke CLIs; MCP is for stateful, multi-call workflows that this tool does not need.
- An LSP-like daemon mode.
- Telemetry of any kind. The string "telemetry" must not appear in the codebase except in this document.

---

## 16. Prompt to give to the implementing agent (Claude Code / Codex)

Copy-paste the block below to start Phase 1. Do not modify it without updating section 14 first.

---

**You are implementing the `axt` Foundation CLI Suite, defined by `docs/spec.md` in this repository. Read the entire spec first. Implement Milestone 0 only. Do not start Milestone 1.**

**Hard rules**:
- Rust 2021 edition, stable toolchain, MSRV 1.78.
- One Cargo workspace, one repository.
- Each binary becomes a separate crate under `crates/`.
- `unwrap()` and `expect()` are forbidden outside `#[cfg(test)]`.
- All primary output modes must be implemented, even if some are stubs in Milestone 0.
- Every command must support `--json`, `--jsonl`, `--agent`, `--plain`, `--print-schema`, `--list-errors`, `--version`, `--help`.
- Diagnostics on stderr, data on stdout, always.
- Cross-platform: Linux, macOS, Windows. CI must be green on all three.
- Prefer small, simple, debuggable code over clever generic abstractions.
- Do not invent new commands. Do not skip writing documentation.

**Deliverables for Milestone 0**:
1. The repository structure of section 6.
2. A workspace that compiles with `cargo build --workspace`.
3. A green CI run on Linux, macOS, Windows.
4. `cargo dist init` configured for tier-1 targets only.
5. A draft GitHub Release produced by tagging `v0.0.1`, containing placeholder binaries.
6. `README.md` introducing the project.

**When Milestone 0 is complete, stop and report**:
- What was implemented.
- What was not implemented and why.
- How to run the tests and the release pipeline locally.
- Any deviations from the spec, with justifications.

**Do not begin Milestone 1 until I confirm.**

---

For Milestones 1+, use this continuation prompt:

> Continue with Milestone N from `docs/spec.md`. Preserve the existing public schemas. Add tests before or alongside code. Run `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`. If a requirement is ambiguous, choose the simplest behavior consistent with the spec, document it inline, and continue. Stop when the milestone's Done criteria are met.

---

## 17. Glossary

- **Agent mode**: ACF output, a compact line-oriented format designed for LLM consumption. First line is a summary/schema line.
- **JSONL mode**: newline-delimited JSON output for streaming, pipelines, and log collectors. Also known as NDJSON.
- **Binary crate**: a Rust crate that produces an executable. In this project, every `axt-*` is one.
- **Internal library crate**: a Rust crate that produces a library, used only inside this workspace. We use four: `axt-core`, `axt-output`, `axt-fs`, `axt-git`. They are not stable API.
- **Schema version**: the `schema` field in JSON, JSONL, and the first ACF line. Format: `axt.<command>.<recordtype>.v<N>` for JSONL records and `axt.<command>.agent.v<N>` for ACF.
- **Tier-1 target**: a target triple whose CI build must succeed for any release to ship.
- **Truncation**: cutting output short when `--limit` or `--max-bytes` is hit. Always announced via explicit truncation metadata (`W code=truncated` in ACF, a warn record in JSONL/JSON).

---

## 18. Open questions for the maintainer

These do not block implementation but should be answered before v1.0:

1. Do we want a centralized config-file format shared across all `axt-*` binaries, or per-binary configs? Default in v0.x: per-binary, in `.axt/<binary>.toml`. Re-evaluate at v0.5.
2. Should `axt-peek` integrate with `jc` (e.g., `axt-peek --pipe-from-jc <command>` to merge structured legacy output)? Probably no — they are orthogonal; document the composition pattern instead.
3. Should we ship an npm wrapper for first-class adoption by JS-tooling agents? Likely yes in v0.5; deferred from v0.1 to keep the release pipeline simple.
4. Should ACF be promoted to a separately-maintained spec (so other tools can reuse the conventions)? Worth considering at v1.0; not now.
