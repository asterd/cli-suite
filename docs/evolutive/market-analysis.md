# Evolutive CLI Market Analysis

Date: 2026-04-28

This document evaluates ten proposed commands for the `axt` suite. It is
deliberately non-contractual: implementation requires a reviewed update to
`docs/spec.md` or `docs/spec-addendum.md` before any new binary is added.

## Research Baseline

The market already has strong tools in adjacent spaces:

- `ripgrep`, `ripgrep-all`, `ast-grep`, `Probe`, and Sourcegraph/Cody cover
  search, structural search, semantic retrieval, and code context.
- Aider and Repomix prove demand for compact repository maps and AI-ready
  codebase packaging.
- `cargo test`, Go test JSON, Jest, Vitest, Pytest, and the existing
  `axt-test` already cover test execution, but not always in one compact
  agent-first schema.
- `git`, GitHub CLI, Onefetch, and Delta cover repository status, hosting
  state, summaries, and diff presentation, but they are human-first or
  provider-specific.
- Existing log tools such as Logdy focus on viewing and streaming logs, not on
  bounded offline triage output for coding agents.

The gap for `axt` is not another faster Unix replacement. The strongest market
space is local, deterministic, single-binary commands that return compact,
schema-versioned context for coding agents, while preserving human output and
cross-platform behavior.

Sources checked:

- Probe: https://github.com/probelabs/probe
- ast-grep: https://ast-grep.github.io/ and https://github.com/ast-grep/ast-grep
- Sourcegraph Cody context: https://sourcegraph.com/docs/cody/core-concepts/context
- Aider repository map: https://aider.chat/docs/repomap.html
- Aider tree-sitter repo map: https://aider.chat/2023/10/22/repomap.html
- Repomix: https://repomix.com/
- Cargo test: https://doc.rust-lang.org/cargo/commands/cargo-test.html
- cargo-tes: https://docs.rs/crate/cargo-tes/0.1.1
- Git Delta: https://github.com/dandavison/delta
- ripgrep-all: https://github.com/phiresky/ripgrep-all
- Logdy CLI: https://logdy.dev/docs/reference/cli

## Summary Verdict

| Proposal | Proposed binary | Optional alias | Market validity | Coverage and impact | Build? |
|---|---|---:|---|---|---|
| `ctxpack` | `axt-ctxpack` | `ctxpack` | High | High agent-loop reduction for repeated search/read cycles. | YES |
| `outline` | `axt-outline` | `outline` | High | High token reduction; overlaps with repo maps but cleaner as direct CLI. | YES |
| `slice` | `axt-slice` | `slice` | High | High precision; removes fragile line-range reads. | YES |
| `repomap` | `axt-repomap` | `repomap` | Medium | Useful, but crowded by Aider and Repomix. Differentiate on schema and local single binary. | YES, later |
| `testdigest` | Extend `axt-test` | `testdigest` not recommended | Medium | Existing `axt-test` already owns this domain. | NO as new binary |
| `gitctx` | `axt-gitctx` | `gitctx` | Medium-high | Strong daily utility; must avoid becoming a human diff pager. | YES |
| `manifests` | `axt-manifest` | `manifest` | Medium | Useful normalized project config, but parsing breadth is large. | YES, later |
| `impact` | `axt-impact` | `impact` | High | High value but highest complexity due LSP integration. | YES, research first |
| `plan-edit` | `axt-plan` | `plan-edit` | Medium-high | Valuable safety layer; crowded by ast-grep, but agent workflow is distinct. | YES |
| `logdx` | `axt-logdx` | `logdx` | Medium-high | Strong debugging value; manageable if scoped to offline triage. | YES |

## Command Evaluations

### 1. `ctxpack`

Market validity: high. Probe demonstrates strong demand for single-call,
code-aware context extraction, and ast-grep proves structural matching is a
real CLI category. The gap is a deterministic `axt` command that correlates
multiple text/regex/AST patterns in one bounded output envelope.

Coverage and impact: high. It directly compresses the common `rg -> sed -> rg`
agent loop and can replace multiple shell calls with one schema. It should not
try to become a semantic search engine or agent. Its strongest MVP is local
multi-pattern search with snippets, language detection, gitignore handling, and
optional AST classification.

Build decision: YES. Use `axt-ctxpack`; optional alias `ctxpack`.

### 2. `outline`

Market validity: high. Aider and Repomix validate symbol maps and code
compression. However, there is still room for a standalone low-level command
that returns only declarations/signatures/doc comments in stable JSON/agent JSONL,
without packaging entire repositories.

Coverage and impact: high. It is one of the best token-saving commands because
agents frequently inspect large files just to understand public surface area.
MVP should cover Rust, TypeScript/JavaScript, Python, Go, and Java through
tree-sitter where feasible, with graceful `feature_unsupported` for unavailable
language parsers.

Build decision: YES. Use `axt-outline`; optional alias `outline`.

### 3. `slice`

Market validity: high. Existing tools can extract blocks by line or use IDE
symbol navigation, but an offline CLI focused on symbol-to-source extraction is
still differentiated. Probe has extraction features, and LSP-based tools can
find symbols, but they are not packaged as `axt` schema-first primitives.

Coverage and impact: high. It removes fragile line-number workflows and is a
natural companion to `axt-outline`. It should initially use tree-sitter
definitions and only add LSP when the fallback behavior and latency are well
understood.

Build decision: YES. Use `axt-slice`; optional alias `slice`.

### 4. `repomap`

Market validity: medium. Aider and Repomix are strong incumbents. Repomix is
especially close to the AI-ready codebase-packaging need, including tree-sitter
compression and token counting. A new command only makes sense if it is not a
full repository packer, but a compact topology/schema generator for agents.

Coverage and impact: medium-high. It can replace repeated startup commands
such as directory listing, manifest reading, test discovery, and recent git
state. The risk is scope creep into `axt-peek`, `axt-doc`, `axt-test`, and
`axt-gitctx`.

Build decision: YES, later. Use `axt-repomap`; optional alias `repomap`.
Position it as an orchestrated summary over existing `axt` primitives.

### 5. `testdigest`

Market validity: medium. The problem is real, and `cargo-tes` shows explicit
agent-token demand for compact failed-test output. However, this repository
already has `axt-test`, whose design is broader and more aligned with the suite.

Coverage and impact: high, but already owned. The correct path is to harden
`axt-test`: improve failure-only summaries, rerun metadata, parser coverage,
and agent JSONL next-step hints. A separate `axt-testdigest` would fragment behavior.

Build decision: NO as a new binary. Implement as `axt-test` evolutions. If a
legacy alias is desired, make `testdigest` an optional alias or subcommand
mapping only after spec approval.

### 6. `gitctx`

Market validity: medium-high. Git itself has all raw data, and tools like
Delta, Onefetch, and GitHub CLI solve adjacent presentation and hosting
problems. The gap is a local, provider-neutral, bounded JSON/agent JSONL worktree
context command for agents.

Coverage and impact: high in daily agent workflows. It can replace `git
status`, `git branch -vv`, `git log --oneline`, `git diff --stat`, and small
diff reads. It must keep strict byte limits and avoid acting as a pager.

Build decision: YES. Use `axt-gitctx`; optional alias `gitctx`.

### 7. `manifests`

Market validity: medium. Every ecosystem has manifest parsers, but a normalized
multi-language CLI is less common. The value is not parsing one file; it is
returning a stable cross-ecosystem project configuration graph.

Coverage and impact: medium. It reduces repeated `cat package.json`,
`Cargo.toml`, `pyproject.toml`, `go.mod`, and CI YAML reads. The main risk is
breadth and fragile normalization across ecosystems.

Build decision: YES, later. Use singular `axt-manifest` for a cleaner binary
name; optional alias `manifest`. Keep the schema extensible and report unknown
sections rather than dropping them silently.

### 8. `impact`

Market validity: high. Sourcegraph/Cody, Serena-style MCP workflows, and IDEs
show that references and call-site context are extremely valuable. The gap is a
local CLI that can be used outside MCP while preserving deterministic output.

Coverage and impact: very high, but technically risky. LSP startup, workspace
configuration, language-specific behavior, and latency can make the command
hard to keep portable. MVP should support Rust first through `rust-analyzer`
when available and a tree-sitter/text fallback when not.

Build decision: YES, after a research milestone. Use `axt-impact`; optional
alias `impact`.

### 9. `plan-edit`

Market validity: medium-high. ast-grep already owns structural search and
rewrite, but the proposed workflow is different: produce an auditable,
schema-versioned edit plan for an agent, then apply exactly that plan
atomically.

Coverage and impact: high for safety. It replaces risky shell edits and large
regex replacements with previewable plans, stable IDs, diffs, and validation.
The command should start as dry-run only, then add apply mode after the plan
format is stable.

Build decision: YES. Use `axt-plan`; optional alias `plan-edit`. The shorter
binary name leaves room for future planned actions beyond edits, while the
alias preserves user intent.

### 10. `logdx`

Market validity: medium-high. Many log tools exist, but most optimize for
interactive viewing, ingestion, or dashboards. A local offline triage command
that returns top errors, deduplicated groups, stack traces, and a compact
timeline for agents remains differentiated.

Coverage and impact: medium-high. It helps when build, test, service, and CI
logs exceed useful model context. MVP should prioritize plain text, JSONL,
syslog-like lines, and common stack traces before adding deeper format support.

Build decision: YES. Use `axt-logdx`; optional alias `logdx`.

## Recommended Implementation Order

1. `axt-outline`
2. `axt-ctxpack`
3. `axt-slice`
4. `axt-gitctx`
5. `axt-plan`
6. `axt-logdx`
7. `axt-manifest`
8. `axt-repomap`
9. `axt-impact`

`axt-test` should be improved in parallel only within its existing milestone
scope or a future `axt-test` evolution. It should not become `axt-testdigest`.

## Shared Implementation Requirements

Every approved command must follow the suite contract:

- Binary name must be `axt-<name>`.
- Unprefixed alias must be opt-in through an `aliases` feature.
- Three primary output modes are required: human, `--json`, and `--agent`.
  Agent output is minified summary-first JSONL.
- Shared flags are required: `--print-schema`, `--list-errors`, `--limit`,
  `--max-bytes`, and `--strict`. Commands may add focused command-specific
  flags, but `--plain`, `--json-data`, and `--jsonl` are retired public flags.
- Diagnostics go to stderr; data goes to stdout.
- No network access in binaries.
- No `unwrap()` or `expect()` in non-test code.
- Add command docs under `docs/commands/`, man pages under `docs/man/`, skills
  under `docs/skills/`, and update `scripts/agent/install-skills.py` only after
  spec approval.
- Add snapshot tests for all output modes and focused unit tests for parsing,
  truncation, errors, and cross-platform behavior.
- Document cross-platform degradation and return `feature_unsupported` with
  exit code 9 when a feature cannot work on a platform.
