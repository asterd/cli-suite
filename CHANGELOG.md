# Changelog

All notable changes to the `axt` Foundation CLI Suite are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the
project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.1.0-rc1

### axt-peek

- Add the `axt-peek` MVP with human, JSON, JSONL, plain, and ACF agent output modes.
- Add `axt.peek.v1` JSON schema generation and validation coverage.
- Add filesystem and git-backed collection with deterministic output, filters, limits, hashes, and non-fatal warnings.

### axt-run

- Add the `axt-run` MVP with `tokio::process` execution, structured exit, duration, and stream summaries.
- Add saved run artifacts at `.axt/runs/<NAME>/` (`meta.json`, `stdout.log`, `stderr.log`, `changed.json`, `summary.agent.acf`).
- Add `axt-run show [<NAME>|last]`, `axt-run list`, and `axt-run clean [--older-than]` for run history management.
- Add `--watch-files` / `--no-watch-files` with include/exclude globs and optional BLAKE3 hashing for change detection.
- Add Unix process-group cleanup via `setpgid` and Windows Job Object cleanup for reliable timeout termination.
- Add `axt.run.v1` JSON schema and per-mode integration tests.

### axt-doc

- Add the `axt-doc` MVP with `which`, `path`, `env`, and `all <CMD>` subcommands.
- Add version-manager attribution covering Homebrew, mise, asdf, rustup, cargo bin, pyenv, rbenv, volta, nvm, Scoop, Chocolatey, and winget.
- Add PATH analysis: duplicates, missing directories, broken symlinks, and ordering issues.
- Add secret-like environment detection with redaction by default and `--show-secrets` opt-in (with stderr warning).
- Add `axt.doc.v1` JSON schema and integration tests.

### axt-drift

- Add the `axt-drift` MVP with `mark`, `diff`, `run`, `list`, and `reset` subcommands.
- Add JSONL snapshots stored at `.axt/drift/<NAME>.jsonl` with optional BLAKE3 hashes via `--hash`.
- Add diff output sorted by absolute size delta covering created, modified, and deleted files.
- Add `axt.drift.v1` JSON schema and integration tests.

### axt-port

- Add the `axt-port` MVP with `list`, `who`, `free`, and `watch` subcommands.
- Use `netstat2` for cross-platform socket→PID mapping and `sysinfo` for process metadata, eliminating runtime dependence on `lsof`, `netstat`, or PowerShell.
- Use `nix` on Unix and `windows-sys` on Windows for direct `kill(2)` / `TerminateProcess` calls; no shelling out to `kill` or `taskkill`.
- Add safety controls: refuse PID 1, refuse self, refuse parent unless `--force-self`, signal escalation (`term` → `kill`) after `--grace`, and `--dry-run` / `--confirm`.
- Add `--tree` propagation and `--timeout` for `watch`.
- Refuse remote-host syntax (`who example.com:443`) with a usage error.
- Add `axt.port.v1` plus per-record schemas (`action`, `holder`, `socket`, `summary`, `warn`).

### axt-test

- Add the `axt-test` MVP with auto-detection for jest, vitest, pytest, cargo test, go test, bun, and deno.
- Add normalized test event schema with streaming JSONL flush during runs.
- Add `--filter`, `--files`, `--changed`, `--changed-since`, `--bail`, `--workers`, `--top-failures`, `--include-output`, `--pass-through`, `--single`, and `list-frameworks`.
- Add config override discovery via `axt-test.toml`, `[tool.axt-test]` (pyproject), and `package.json#axt-test`.
- Add `axt.test.v1`, `axt.test.case.v1`, `axt.test.suite.v1`, `axt.test.summary.v1`, `axt.test.framework.v1`, and `axt.test.warn.v1` schemas.

### Shared crates

- Add `axt-core` with `ErrorCode` enum, exit-code mapping, `OutputMode`, output limits, color resolution honoring `NO_COLOR` / `CLICOLOR_FORCE` / `FORCE_COLOR`, `Clock` trait, `--print-schema`, and `--list-errors` shared flags.
- Add `axt-output` with `Renderable`, `JsonEnvelope`, `JsonlWriter`, `AgentCompactWriter`, and TTY-aware styling helpers.
- Add `axt-fs` with an `ignore::WalkBuilder`-based walker, deterministic per-file metadata, language and MIME detection, generated-file heuristics, symlink-loop guarding, and opt-in BLAKE3 hashing.
- Add `axt-git` as a thin `gix` wrapper for repo discovery, cached per-path status, current branch, dirty counts, and ref-to-ref diffs, all with graceful "no git" handling.

### Release

- Configure the cargo-dist release pipeline for the v0.1.0 release candidate (shell, PowerShell, and Homebrew installers; tier-1 targets).
- Add a release follow-up workflow that produces GitHub Artifact Attestations, a CycloneDX SBOM, and an automated Scoop bucket pull request.
- Add publishable package metadata for every crate `cargo install axt-*` reaches.
- Add `cargo-audit` and `cargo-deny` advisory checks to CI.

## 0.0.1

- Initial workspace scaffolding.
