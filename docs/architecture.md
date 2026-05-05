# Architecture

`axt` is a Rust workspace of independent binaries plus shared internal crates.
Each command is installed and executed as its own `axt-*` binary; there is no
top-level router binary.

## Workspace Shape

| Area | Responsibility |
|---|---|
| `crates/axt-core` | Shared CLI flags, output-mode resolution, limits, error catalog, path and time helpers. |
| `crates/axt-output` | JSON envelope rendering and bounded summary-first agent JSONL helpers. |
| `crates/axt-fs` | Gitignore-aware local filesystem walking and file metadata helpers. |
| `crates/axt-git` | Local Git repository discovery and lightweight Git state helpers. |
| `crates/axt-*` | Command-specific parsing, domain logic, renderers, schemas, and tests. |
| `schemas/` | Public JSON schema files for stable machine-readable output. |
| `docs/commands/` | User-facing command manuals aligned with the implemented CLI surface. |

## Command Flow

Each binary follows the same high-level path:

1. Parse command-specific `clap` arguments plus shared `CommonArgs`.
2. Resolve output mode from explicit flags, `AXT_OUTPUT`, and stdout TTY state.
3. Execute only local filesystem, process, Git, parser, or test-runner work.
4. Render human output, the canonical JSON envelope, or agent JSONL.
5. Return typed exit codes from the standard error catalog.

Diagnostics are written to stderr. Structured data is written to stdout. Binaries
do not perform telemetry, analytics, postinstall fetches, or remote network
calls.

## Output Contract

Every command supports:

```bash
--json
--agent
--print-schema [human|json|agent]
--list-errors
--limit <N>
--max-bytes <BYTES>
--strict
```

Human output is for terminals and is not a stable parse target. `--json` emits
one envelope with `schema`, `ok`, `data`, `warnings`, and `errors`. `--agent`
emits minified JSONL with a summary record first and bounded detail records
after it.

## Examples

Inspect a repository before deeper work:

```bash
axt-bundle . --agent
```

Find changed files, then extract a symbol body:

```bash
axt-peek . --changed --agent
axt-outline --agent src
axt-slice src/lib.rs --symbol parse_config --agent
```

Run tests and diagnose failures without parsing framework-specific output:

```bash
axt-test --agent
axt-logdx target/test.log --severity error --top 20 --agent
```

`docs/spec.md` and `docs/spec-addendum.md` remain the contractual source of
truth for behavior and release milestones.
