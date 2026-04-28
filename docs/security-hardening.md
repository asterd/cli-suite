# Security Hardening Review

This document records the current security posture of the `axt` suite and the
remaining hardening work required before a stable public release.

## Current Controls

| Area | Current control | Evidence |
|---|---|---|
| Memory safety | Shared libraries deny unsafe code; binary unsafe blocks are limited to process control. | `#![deny(unsafe_code)]` in shared crates; `SAFETY:` comments in `axt-run` and `axt-port`. |
| Panic avoidance | Non-test source denies `unwrap()` and `expect()` through Clippy policy and contains no direct calls. | `cargo clippy --workspace --all-targets -- -D warnings`; `rg "unwrap\\(|expect\\(" crates/*/src`. |
| Network isolation | Binaries do not depend on HTTP client crates and do not perform remote calls. | Cargo manifests contain no `reqwest`, `ureq`, `hyper`, or `isahc`. |
| Output injection | JSON/JSONL uses `serde_json`; ACF values are quoted when needed by `axt-output`. | `axt-output::format_agent_fields`. |
| Secret handling | `axt-doc` redacts secret-like environment values by default. | `docs/commands/doc.md`; `axt-doc env`. |
| Process mutation | `axt-port free` supports `--dry-run`, refuses PID 1 and self, and requires explicit subcommand. | `crates/axt-port/src/signal.rs`; `crates/axt-port/tests/modes.rs`. |
| Command execution | `axt-run --shell` is opt-in; normal mode passes argv directly. | `crates/axt-run/src/execute.rs`. |
| Schema stability | JSON and JSONL outputs validate against committed schemas in tests. | `crates/*/tests/modes.rs`. |

## OWASP-Oriented Mapping

| Risk class | Applicability | Mitigation |
|---|---|---|
| Injection | CLI args, shell mode, env files, glob filters. | Direct argv execution by default; shell execution requires `--shell`; structured output uses serializers. |
| Sensitive data exposure | Environment diagnostics and command output capture. | `axt-doc` redaction by default; saved run artifacts are local and explicit; docs warn about mutating/captured data. |
| Broken access control | Process inspection and signaling. | OS permissions are respected; permission failures become typed errors; dangerous PIDs are refused. |
| Security misconfiguration | Install aliases and release artifacts. | Aliases are opt-in; CI verifies alias targets; release follow-up packages docs/skills as explicit artifacts. |
| Vulnerable dependencies | Rust dependency supply chain. | CI runs `cargo audit` and `cargo deny check advisories`. |
| Logging and monitoring exposure | Captured stdout/stderr from child commands. | Capturing is configurable; saved logs are local under `.axt/runs`; no remote reporting exists. |
| SSRF / remote network abuse | Not a network tool. | No binary performs remote network calls. `axt-port` rejects remote host:port syntax. |

## Remaining Work Before Stable Release

| Gap | Resolution |
|---|---|
| Cross-platform destructive behavior needs broader validation. | Add OS-specific smoke tests for `axt-port free` on Linux, macOS, and Windows CI using owned fixture processes. |
| Parser fuzzing is not yet present. | Add property/fuzz tests for ACF formatting, duration parsing, JSONL streaming, env-file parsing, and path normalization. |
| Release artifacts need end-to-end install checks. | After tagging, verify shell, PowerShell, Homebrew, Scoop, and Cargo installs on clean hosts. |
| `axt-test` framework reporter coverage is still best effort for some ecosystems. | Prefer native machine formats where stable; keep fallback parsing deterministic and covered by fixtures. |
| Unsafe process-control blocks need platform audit. | Review Unix `pre_exec`/signals and Windows Job Object/TerminateProcess blocks with platform maintainers before v1.0. |

## Local Audit Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo check --workspace --all-features
rg "unwrap\\(|expect\\(" crates/*/src
rg "reqwest|ureq|hyper|isahc" crates Cargo.toml
```
