# axt — Production-Readiness Audit & Hardening Plan

Audit date: 2026-04-29
Workspace version: 0.1.0-rc1
Scope: all 12 user-facing commands + 4 shared crates (`axt-core`, `axt-output`, `axt-fs`, `axt-git`).

This document is **diagnostic and prescriptive only** — no code is changed.
Each section describes the gap, severity, and the concrete intervention to
move the command from "MVP that works on the happy path" to "battle-tested,
production-grade".

## Severity legend

| Tag | Meaning |
|-----|---------|
| **Critical** | Can crash the binary, corrupt user data, or be exploited. Must fix before GA. |
| **High** | Incorrect behavior in realistic conditions (hangs, OOM, wrong output). Must fix before declaring GA. |
| **Medium** | Edge-case correctness, race conditions in low-probability paths, missing observability. Should fix in the RC2 cycle. |
| **Low** | Polish, defense-in-depth, future-proofing, performance micro-issues. Nice to have. |

## Executive summary

The suite is **architecturally sound**: shared crates are clean, lints already
deny `unwrap`/`expect` in non-test code, agent-mode JSONL contract is well
respected, error catalogs and exit codes are stable. None of the reviewed
commands has the kind of hidden `todo!()`/`unimplemented!()` placeholders that
would make them fake. They run, they produce the right output on the happy
path, and they shut down on the unhappy path.

Where they are **not yet production-grade**, the gaps are concentrated in four
recurring families:

1. **Resource bounds on hostile or pathological input** — regex DoS in
   `axt-ctxpack`, unbounded stderr capture and missing child-process timeouts
   in `axt-test`, hash-the-world on huge files in `axt-drift`, O(n²) tail
   buffer in `axt-run`.
2. **Filesystem durability** — non-atomic snapshot writes (`axt-drift`),
   missing `fsync()` on artifacts (`axt-drift`, `axt-run`), partial-file
   visibility on crash.
3. **Process safety** — TOCTOU PID reuse window in `axt-port free`,
   non-configurable grace timings, parent-PID fallback to `0`.
4. **Parser fragility on third-party output** — `axt-test` framework parsers
   silently default missing fields, swallow stderr-reader panics; `axt-doc`
   secret list is incomplete; `axt-gitctx` synthesizes diff text without
   escaping path components.

Below, one section per command (and per shared crate) with the full punch
list. Each entry: **what's wrong → severity → how I would fix it**.

---

## Per-command audit

### `axt-peek` — filesystem snapshot

Status: **production-ready**, minor performance and consistency notes.

| # | Severity | Location | Issue | Fix |
|---|----------|----------|-------|-----|
| 1 | Low | `crates/axt-peek/src/collect.rs:110-130` | `ignored_count()` walks the tree twice (once respecting ignore, once raw) just to count ignored entries. Doubles I/O on cold cache for large monorepos. | Fold both walks into the existing single pass: keep two counters in the visitor closure. Alternatively, drop the count from default output and gate it behind `--with-ignored-count`. |
| 2 | Low | `crates/axt-peek/src/collect.rs:391-392` | Path display normalizes Windows backslashes only at render time; internal comparisons see raw `\`. Risk of de-dup misses in pathological mixed-case scenarios. | Normalize to forward-slash form once, immediately after canonicalization, and use that form everywhere downstream. |

### `axt-run` — structured command runner

Status: **production-ready** with two correctness fixes recommended.

| # | Severity | Location | Issue | Fix |
|---|----------|----------|-------|-----|
| 1 | High | `crates/axt-run/src/execute.rs:416-449` (`TailBuffer`) | The tail buffer is a `VecDeque<u8>` with per-byte `push_back` + `pop_front` for every output byte. On a child that emits hundreds of MB of output (CI build logs), this is O(n²) on the byte stream and saturates a single core. | Replace with a pre-sized ring buffer over a `Box<[u8]>` plus head/tail indices, or with a chunked deque keyed on lines. Either reduces the hot path to amortized O(1) per byte. |
| 2 | Medium | `crates/axt-run/src/storage.rs:200-210` | The `axt-run.toml` parser is a hand-rolled string splitter. Malformed config silently degrades to defaults instead of warning the user. | Switch to the `toml` crate with a typed config struct; on parse error, emit a `warn` JSONL record and proceed with defaults. |
| 3 | Medium | `crates/axt-run/src/execute.rs:273-278` | Unix shell fallback reads `$SHELL` with no validation. A poisoned env (`SHELL=/tmp/evil`) executes attacker-controlled code under the user's identity. This is intentional behavior for some users but should be opt-in. | Validate `$SHELL` against an allow-list (`/bin/sh`, `/bin/bash`, `/bin/zsh`, `/usr/bin/...`), or require `--shell <path>` when not in the allow-list, or fall back to `/bin/sh` and warn. |
| 4 | Medium | `crates/axt-run/src/storage.rs` (artifact writes) | Run artifacts (`meta.json`, `stdout.log`, `stderr.log`, `summary.agent.acf`) are written without `sync_all()`. Power loss or `kill -9` mid-write leaves a partially-written run that subsequent `axt-run show` will choke on. | Write artifacts via `tempfile::NamedTempFile::persist` (atomic rename) and `sync_all()` the parent directory before declaring the run complete. |
| 5 | Low | `crates/axt-run/src/storage.rs:123` | `metadata.modified().unwrap_or(now)` in `--older-than` cleanup means a filesystem that doesn't expose mtime would delete *or keep* runs unpredictably. | Skip entries where mtime is unavailable and emit a `warn` record. |

### `axt-doc` — environment doctor

Status: **production-ready**, secret-detection coverage is the main gap.

| # | Severity | Location | Issue | Fix |
|---|----------|----------|-------|-----|
| 1 | Medium | `crates/axt-doc/src/command.rs:484-494` (`is_secret_like`) | Pattern list misses common real-world secret env names: `*_DSN`, `*_BEARER`, `JWT*`, `STRIPE_*`, `OPENAI_API_*` (covered by `_KEY`), `GH_TOKEN` (covered by `_TOKEN`), `DATABASE_URL` (containing creds), `*_WEBHOOK_URL`. False negatives leak. | Layered approach: (a) keep the suffix list, (b) add a name allow-list of known secret-bearing names, (c) add a value heuristic — if value matches `^([a-zA-Z0-9_-]{20,}|[A-Za-z0-9+/=]{40,})$` and the name is not on a known-safe list, redact and warn. |
| 2 | Medium | `crates/axt-doc/src/command.rs:22,33,59` | Manager hint probes use hard-coded 300/1500 ms timeouts. On slow corp laptops these false-positive into "manager unavailable". | Expose `--probe-timeout-ms` (default 1500) and document it. Cap at 10 s. |
| 3 | Low | `crates/axt-doc/src/command.rs:156-157` | `env::split_paths()` silently drops malformed entries. A user with a corrupt PATH never knows. | After split, walk the *raw* PATH string; if `count(separators)+1 != entries.len()` emit a warning with the dropped fragment. |

### `axt-drift` — filesystem mark + diff

Status: **MVP-grade**. Three blocking issues for production use on large repos.

| # | Severity | Location | Issue | Fix |
|---|----------|----------|-------|-----|
| 1 | High | `crates/axt-drift/src/snapshot.rs:88-107` (`write`) | Snapshot file is created and then written line-by-line. A SIGKILL or power loss in the middle of `axt-drift mark` leaves a half-written `.jsonl` that the next `axt-drift diff` will read as truth. Silent data corruption. | Write to `<name>.jsonl.tmp` via `BufWriter`, call `sync_all()`, then `rename()` over the final path. Use `tempfile::NamedTempFile::persist` for the cross-platform-safe variant. Also `sync_all()` the parent dir on Unix. |
| 2 | High | `crates/axt-drift/src/snapshot.rs:23,68` | `capture()` and `read()` both materialize the full tree into a `BTreeMap` in memory. On a 5M-file monorepo this is several GB. | Stream both sides as sorted JSONL and run a merge-style diff (two cursors). Output is already deterministic by path, so the input is too. Memory becomes O(longest path). |
| 3 | Medium | `crates/axt-drift/src/command.rs:46-50` | TOCTOU between `path.exists()` and `Snapshot::read(&path)`. Concurrent `reset` deletes the file, user sees a confusing IO error rather than `MarkNotFound`. | Drop the `exists()` check; map `io::ErrorKind::NotFound` returned by `read` to `MarkNotFound` directly. |
| 4 | Medium | `crates/axt-drift/src/snapshot.rs:254-272` (`hash_file`) | `--hash` reads files synchronously in 8KiB chunks during the walk. A single 100 GB log file blocks the whole snapshot. No size cap. | Add `--hash-max-bytes` (default 256 MiB). Above that: emit metadata-only entry plus a `hash_skipped_size` warning. Optionally parallelize hashing with `rayon` (the walk is already deterministic, sort the output). |
| 5 | Medium | `crates/axt-drift/src/snapshot.rs:101` | No `sync_all()` before declaring write complete (related to #1, but worth calling out separately for the non-atomic path that remains for streaming writes). | Always call `file.sync_all()` then sync parent dir on Unix. |
| 6 | Low | `crates/axt-drift/src/snapshot.rs:24` | Symlinks are silently excluded (`follow_links(false)`) but a tree full of symlinks shows zero entries with no diagnostic. | Track symlinks as `kind: "symlink"` records *or* emit a single warning "skipped N symlinks". |

### `axt-port` — port lister and freer

Status: **dangerous in adversarial timing**. The PID-reuse window is the
classic kill-the-wrong-process trap; mitigate before relying on
`axt-port free` in CI.

| # | Severity | Location | Issue | Fix |
|---|----------|----------|-------|-----|
| 1 | High | `crates/axt-port/src/signal.rs:224-237`, `:95` | TOCTOU between PID lookup and `kill(2)`/`TerminateProcess`: the holder process can exit and its PID can be reused by an unrelated process before we send the signal. On Linux this is a known privesc/foot-gun pattern. | (Linux) Open `/proc/<pid>` (or `pidfd_open(2)` on kernel ≥5.3) once, and use `pidfd_send_signal(2)` so the kernel rejects the signal if the descriptor no longer points to the original process. (macOS) capture `proc_pidinfo` start-time and re-check before each kill. (Windows) `OpenProcess(PROCESS_TERMINATE, ...)` once, then `TerminateProcess` on the handle — the handle is bound to the original process. |
| 2 | High | `crates/axt-port/src/command.rs:76-99` (`watch`) | The watch loop can in theory tick past the user's `--timeout` if the per-iteration sleep returns late. | Compute `deadline = Instant::now() + timeout` at entry; check `Instant::now() < deadline` at the top of every iteration, not after the sleep. |
| 3 | Medium | `crates/axt-port/src/signal.rs:91-94,116` | Hard-coded 100 ms grace and 100 ms post-escalation. Too short on macOS-on-Apple-silicon under load; too long for fast CI. | Surface as `--grace-ms` (already exists for one phase, add for both) with documented defaults. |
| 4 | Medium | `crates/axt-port/src/signal.rs:309-314` (`parent_pid`) | If `sysinfo` cannot resolve the parent, the code uses parent_pid `0`, which never matches a real parent — so the parent-refusal safety check silently no-ops. | Treat lookup failure as "parent unknown"; refuse the kill unless `--force-self` (or a new `--allow-unknown-parent`) is passed. |
| 5 | Medium | `crates/axt-port/src/signal.rs:202`, `command.rs:102` | `u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX)` mints `u64::MAX` on overflow. With `elapsed` capped by the watch timeout this is fine; if the cap is removed the value becomes meaningless. | Use `elapsed.as_millis().min(u64::MAX as u128) as u64` — same numeric outcome but the intent is explicit and lints don't trip on the `try_from` round-trip. |
| 6 | Low | `crates/axt-port/src/main.rs:73-75` | `listener_fixture()` test helper sleeps 60 s in a loop with no shutdown signal — leaks a thread on test panics. | Take a `oneshot::Receiver<()>` and `select!` against the sleep. |

### `axt-test` — multi-framework test runner

Status: **production-ready for the happy path**, several real risks under
hostile or malformed framework output.

| # | Severity | Location | Issue | Fix |
|---|----------|----------|-------|-----|
| 1 | High | `crates/axt-test/src/command.rs:304,370` | `child.wait()` has no upper-bound timeout. A hung test framework (deadlocked test, infinite loop without output) blocks `axt-test` indefinitely; in CI this looks like the runner is broken. | Add `--max-duration <DURATION>` (default unset, but with a 30 min hard ceiling under `--strict`). On expiry, kill the child via existing process-group/job-object teardown logic. |
| 2 | High | `crates/axt-test/src/command.rs:340-342` | stderr is captured with `read_to_string` into an unbounded `String`. A misbehaved framework spamming stderr (e.g., a test that prints in a tight loop) OOMs the runner. | Use the same `TailBuffer` approach as `axt-run` (after fixing it per the `axt-run` audit), capped at `--max-output-bytes` (already a shared flag). On overflow, set `truncated: true` and continue. |
| 3 | High | `crates/axt-test/src/frontend.rs:281-391` | Per-framework parsers default missing fields silently to `"unnamed test"`, `"test failed"`, etc. A future framework version that renames a field will produce a green-looking run that has actually swallowed all real failures. | (a) Use distinct sentinels (e.g., `"<axt-test:parse-error>"`) and treat them as a hard error in `--strict`; (b) version-pin each framework — record the detected framework version and emit a `warn` record when it falls outside the supported range. |
| 4 | Medium | `crates/axt-test/src/command.rs:339-376` | Stderr-reader thread panic is mapped to `TestError::Io("framework stderr reader panicked")` — a generic IO error, no backtrace. | Wrap the reader body in `std::panic::catch_unwind`, capture the panic message, and return `TestError::ParserPanic { framework, message }` so users have something to file a bug with. |
| 5 | Medium | `crates/axt-test/src/command.rs:250,439` | Streaming JSONL writer flushes per event with no backpressure. Under a million-event run (large pytest), the agent consumer can fall behind and we keep buffering. | Cap the in-flight buffer at `limits.max_output_lines`; on overflow, drop with a single `truncated:true` summary and stop writing further events. |
| 6 | Medium | `crates/axt-test/src/frontend.rs:277,298,355,427` | 14× `unwrap_or(...)` on JSON keys. Hard to distinguish "field absent in this framework version" from "value was actually 0". | Centralize defaults in a single `extract_*` helper that records every defaulted field into a per-event `defaulted_fields: [...]` audit list, surfaced under `--debug-parser`. |

### `axt-outline` — source outlines

Status: **production-ready**.

| # | Severity | Location | Issue | Fix |
|---|----------|----------|-------|-----|
| 1 | Low | `crates/axt-outline/src/tree.rs:495` | Direct slicing `source[node.start_byte()..node.end_byte()]`. tree-sitter byte ranges are reliable on valid input but a future grammar bug or a malformed file pre-loaded into a parser could produce out-of-range or non-char-boundary indices. | `source.get(start..end).unwrap_or("")` and emit a `warn` record on miss. |
| 2 | Low | `crates/axt-outline/src/tree.rs:414-415` | Python signature extraction guesses the body offset; on highly nested decorators the heuristic produces a truncated signature. | Use the AST `body` field directly (it's exposed by tree-sitter-python) instead of inferring from text. |

### `axt-slice` — extract source by symbol or line

Status: **production-ready** after one defensive guard.

> Note: a previous review flagged `text[line.start..matched.start()].chars().count()` as a UTF-8 panic. That call site is **safe**: `regex::Regex::find_iter` on `&str` always returns offsets aligned to char boundaries. The defensive guards below cover the *next* time a similar pattern is added on raw bytes.

| # | Severity | Location | Issue | Fix |
|---|----------|----------|-------|-----|
| 1 | Medium | `crates/axt-slice/src/command.rs:323` (`extract_spans`) | `source[span.start_byte..span.end_byte]` trusts spans produced upstream. If a future change feeds in spans built from anything other than tree-sitter on the same buffer, an out-of-range slice will panic. | Replace direct indexing with `source.get(span.start_byte..span.end_byte)` and surface a warning when `None`. |
| 2 | Low | `crates/axt-slice/src/command.rs:378-386` | Binary detection is a bespoke null-byte scan; `axt-ctxpack` uses a different one. | Move both to `axt-fs::is_text_file()` and call from each site. |

### `axt-ctxpack` — multi-pattern AST search

Status: **production-ready except for regex DoS**.

| # | Severity | Location | Issue | Fix |
|---|----------|----------|-------|-----|
| 1 | High | `crates/axt-ctxpack/src/command.rs:147-150` | User regex compiled with `Regex::new` and run via `find_iter` against arbitrarily large source. Patterns like `(a+)+b` or `.*.*.*x` against a long line produce catastrophic backtracking — well, the `regex` crate is linear-time on its supported subset, but it does not bound the *DFA size*. A maliciously crafted pattern (`a{10}{10}{10}`) can blow up `RegexBuilder::dfa_size_limit` failures, *and* can make compilation itself take seconds. There is no per-call CPU/time budget. | Use `RegexBuilder::new(pat).size_limit(SIZE).dfa_size_limit(SIZE).build()` with an explicit cap (e.g., 10 MiB). Add a wall-clock budget enforced via a polling thread that calls `Cancel` on the regex (the `regex` crate exposes `Regex::cap_per_match` indirectly via `RegexSet`); simpler: time-out the whole `axt-ctxpack` run with a `--max-duration` (default 30 s). |
| 2 | Medium | `crates/axt-ctxpack/src/ast.rs:62-72` | TSX content is parsed with `tree-sitter-typescript` (TS, not TSX). JSX inside `.tsx` parses as syntax errors and the AST classifier reports unhelpful enclosing-symbol info. | Pass the file path into `classify_with_tree_sitter`; dispatch to `LANGUAGE_TSX` for `.tsx` and `LANGUAGE_TYPESCRIPT` for `.ts`. (The dispatch already exists in `axt-slice`, lift it into `axt-ctxpack`.) |
| 3 | Medium | `crates/axt-ctxpack/src/ast.rs:213-215` | `preceding_source` truncates to 512 bytes when looking for `#[test]` / `cfg(test)`. Misses test functions with long doc-comments above them. | Either extend to 4 KiB or, better, walk parents in the AST and look for a `test_attribute` ancestor — already covered for some languages, generalize. |
| 4 | Low | `crates/axt-ctxpack/src/command.rs:63-69` | Binary detection is null-byte only; UTF-16-LE files with even-aligned null bytes might pass through and produce mojibake snippets. | Reuse `content_inspector` (already a workspace dep) — `inspect(bytes).is_binary()` and `is_utf16le()` decisions in one place. |

### `axt-bundle` — session warmup bundle

Status: **production-ready**.

| # | Severity | Location | Issue | Fix |
|---|----------|----------|-------|-----|
| 1 | Low | `crates/axt-bundle/src/main.rs:199` | `.take(args.max_files)` truncates the file list; deterministic ordering depends on the upstream walker's sort. | Add an explicit `BTreeSet`/sorted-vec assertion at the truncation point and a unit test that pins the cut-off to a stable subset. |
| 2 | Low | `crates/axt-bundle/src/main.rs:239` | Manifest read uses `fs::read_to_string`, which fails (silently warned in summary, but no output for the file) on UTF-16/Latin-1 manifests. Some Windows tooling still writes UTF-16-BOM `package.json`. | Read bytes, then decode via `encoding_rs` if a BOM is present; emit an `encoding_converted` warning. |

### `axt-gitctx` — bounded git context

Status: **production-ready**, two correctness fixes around path escaping.

| # | Severity | Location | Issue | Fix |
|---|----------|----------|-------|-----|
| 1 | High | `crates/axt-gitctx/src/app.rs:493` (`synthetic_untracked_diff`) | The synthetic diff header is built by string concatenation with raw paths. A path containing `\n` or `+++ ` produces a diff that breaks any downstream consumer that re-parses unified-diff. | Always quote the path per the unified-diff spec (`"path with\nspecial"`), never inline raw bytes. Reuse the same quoting `git` itself uses (`core.quotePath`-style C-quoting). |
| 2 | Medium | `crates/axt-gitctx/src/app.rs:277` (`upstream_ref`) | `@{upstream}` ref string is built character-by-character via `push`. Cosmetic; gives the impression of dynamic input where it's static. | Replace with a `const UPSTREAM: &str = "@{upstream}";`. |
| 3 | Medium | `crates/axt-gitctx/src/app.rs:485` | Non-UTF-8 diff bytes return an empty diff with no record. User cannot tell "no changes" from "changes that didn't decode". | Decode lossily (`from_utf8_lossy`), set `encoding: "lossy-utf8"` on the record, and emit a `warn`. |
| 4 | Medium | `crates/axt-gitctx/src/app.rs:305-306` | `from_utf8` on `git status -z` output errors hard on non-UTF-8 filenames (legitimate on Linux). | Use `OsString::from_vec` (Unix) / `OsString::from_wide` (Windows) and render via `String::from_utf8_lossy` only at the rendering boundary. |
| 5 | Low | `crates/axt-gitctx/src/app.rs:616` (`shell_quote`) | Hand-rolled POSIX shell escaping; misses backslash and `$()` edge cases. | Use the `shell-escape` or `shlex::quote` crate (single dep), or only emit shell-quoted suggestions when the path is `[A-Za-z0-9._/-]+` and otherwise output the path as a JSON-escaped argv item. |

### `axt-logdx` — log triage

Status: **production-ready**. The streaming design is the model the rest of
the suite should aspire to.

| # | Severity | Location | Issue | Fix |
|---|----------|----------|-------|-----|
| 1 | Medium | `crates/axt-logdx/src/command.rs:176` | Stack-line capture is hard-capped at `MAX_STACK_LINES=24`, but the resulting record does not carry a `stack_truncated: bool` flag. Consumers cannot tell "short stack" from "long stack we cut". | Add `stack_truncated` to the record schema (under `axt.logdx.failure.v1`) and bump the schema minor. |
| 2 | Low | `crates/axt-logdx/src/command.rs:789` | `parse_rfc3339` trims a fixed character set including `]` and `"` symmetrically; `[2026-01-01T00:00:00Z}` is accepted. Cosmetic. | Pair each opening delimiter with its matching closing one. |
| 3 | Low | `crates/axt-logdx/src/command.rs:797` | Epoch detection only matches 10- or 13-digit integers. Logs that emit fractional milliseconds (15+ digits) silently drop to "no timestamp". | Accept any `[0-9]{10}(\.[0-9]+)?` / `[0-9]{13}(\.[0-9]+)?` pattern. |

---

## Shared crates

### `axt-core`

Status: **production-ready**. No findings.

### `axt-output`

| # | Severity | Location | Issue | Fix |
|---|----------|----------|-------|-----|
| 1 | Low | `crates/axt-output/src/lib.rs:250-266` (`finish`) | The truncation warning record is written but the writer is not explicitly flushed before returning; it relies on `Drop`. On signal, the warning may never reach stderr. | Call `self.inner.flush()` immediately after writing the truncation warning, before returning. |

### `axt-fs`

Status: **production-ready**. The 8 KiB streaming hash and explicit symlink-loop detection are textbook.

### `axt-git`

| # | Severity | Location | Issue | Fix |
|---|----------|----------|-------|-----|
| 1 | Medium | `crates/axt-git/src/lib.rs:119,213-227` | Submodules are reported by their parent path only; files inside a dirty submodule are invisible. Not wrong per se, but undocumented. Affects `axt-bundle` and `axt-drift` reports on monorepos with submodules. | Detect submodules via `gix::submodule::list()`, expose them in a separate `submodules: [{path, status, head}]` block in the dependent commands' output. |
| 2 | Medium | `crates/axt-git/src/lib.rs` (repo discovery) | Shallow clones are accepted silently. `diff_paths()` against `HEAD~N` where `N > shallow-depth` returns empty. | Call `repo.is_shallow()` at discovery; surface as `shallow: true` in the git block of every dependent command's output, and refuse history-deep operations with `feature_unsupported` (exit 9). |
| 3 | Low | `crates/axt-git/src/lib.rs:361-385` | `head_index_entries` returns empty on missing HEAD; correct for an unborn repo, but indistinguishable from "I gave up parsing HEAD". | Return `Result<Option<...>>` and let callers branch. Tests already cover the unborn case. |

---

## Cross-cutting recommendations

These apply to the whole suite; addressing them once removes a class of issue
across many commands.

1. **Wall-clock budget shared flag.** Add `--max-duration <DURATION>` to the
   shared flag set. Plumb it through every command that runs a child or an
   unbounded loop (`axt-run`, `axt-test`, `axt-port watch`, `axt-ctxpack`,
   `axt-drift`). Default unset; under `--strict`, default to 5 minutes.

2. **`fsync()` everywhere artifacts are written.** Wrap the four artifact
   writers (`axt-run`, `axt-drift`, future `axt-test --report-dir`,
   `axt-bundle --output`) into a single helper in `axt-core` —
   `axt_core::fs::write_atomic(path, bytes)` — that does temp-write +
   `sync_all` + rename + parent-dir sync. Audit all writers to use it.

3. **`pidfd` / handle-bound kill** on Linux ≥ 5.3, with feature detection at
   runtime; same pattern via process handles on Windows. Lifts a class of
   PID-reuse bugs out of `axt-port` and any future command that signals
   external processes.

4. **Bounded streaming everywhere.** Every path that reads child output
   (`axt-run`, `axt-test`, future `axt-impact`) must use a single shared
   `BoundedTailBuffer` from `axt-core`. Fix the O(n²) issue once, get it for
   free everywhere.

5. **Regex hardening helper in `axt-core`.** A `compile_user_regex(pat,
   limits)` that sets `size_limit`, `dfa_size_limit`, `case_insensitive`
   policy, and rejects patterns over a length cap. Use it in `axt-ctxpack`
   and any future search-shaped command.

6. **Encoding helper in `axt-fs`.** A `read_to_string_smart(path)` that
   handles BOM, UTF-16 LE/BE, common Latin-1 fallback, and emits a uniform
   `encoding_converted` warning. Adopt in `axt-bundle`, `axt-doc`,
   `axt-gitctx`, `axt-logdx`, `axt-test` (config files).

7. **Framework version pinning** for `axt-test`. Detect and record the
   version of every supported framework (jest/vitest/pytest/cargo/go/bun/deno)
   on first invocation; refuse to run under `--strict` if outside the tested
   range. Add nightly CI matrix that exercises each framework's two latest
   minor versions.

8. **Property-based tests** using `proptest` (already a dev-dep) for the
   parsers most exposed to third-party output: `axt-test/frontend.rs`,
   `axt-logdx/command.rs`, `axt-gitctx/app.rs` git porcelain parser. Each
   should accept arbitrary byte input without panicking.

9. **Fuzz targets** for the same three parsers, runnable under
   `cargo +nightly fuzz` on a CI weekly job. One target per parser, seeded
   from existing fixtures.

10. **OS smoke matrix.** Today CI builds on the three platforms but the
    cross-platform behavior matrix in the docs is mostly aspirational.
    Add nightly smoke jobs that run a shared `tests/smoke/*.sh` (or `.ps1`
    on Windows) on real fixtures: at least `axt-port free` against a
    locally-bound listener, `axt-run` with timeout, `axt-test` against each
    framework, `axt-drift` with `--hash`.

---

## Suggested execution order (RC2 → GA)

### Wave 1 — must ship before GA (≈1 sprint)

- `axt-port` PID-reuse fix (#1) — security/correctness blocker.
- `axt-drift` atomic snapshot writes (#1) and streaming diff (#2).
- `axt-ctxpack` regex DoS hardening (#1).
- `axt-test` child timeout (#1) and bounded stderr (#2).
- `axt-run` `TailBuffer` rewrite (#1).
- `axt-gitctx` synthetic-diff path quoting (#1).

### Wave 2 — should ship before GA (≈1 sprint)

- Cross-cutting #1 (`--max-duration`), #2 (`write_atomic`), #4 (bounded
  streaming) implemented in shared crates and adopted everywhere.
- `axt-doc` secret detection layered approach (#1).
- `axt-test` parser fragility fixes (#3, #4, #5, #6).
- `axt-git` submodule + shallow surfacing (#1, #2).

### Wave 3 — RC3 / hardening (≈1 sprint)

- `axt-port` configurable grace (#3) and parent-PID failure handling (#4).
- `axt-drift` `--hash-max-bytes`, parallel hashing (#4).
- `axt-bundle`, `axt-gitctx`, `axt-logdx` encoding helper adoption.
- All shared `axt-output` flush fix.
- proptest + fuzz targets.

### Wave 4 — polish (post-GA, continuous)

- `axt-peek` single-pass ignore counter.
- All Low-severity items.
- OS smoke matrix expansion.
- Regression fixtures from any bug found in production.

---

## Out of scope of this audit

- Public API of the schemas (`axt.*.v*`) — covered by the spec, not the code.
- Documentation completeness (covered by `docs/` review).
- Release-pipeline (cargo-dist) hardening — covered by `docs/release.md`.
- Performance benchmarking — once Waves 1–2 land, run `criterion` targets per
  command on representative repos (Linux kernel, chromium, large monorepo) and
  set per-command latency budgets.

---

## Closing note

None of the above invalidates the architecture. The suite picks the right
abstractions: typed errors, schema-versioned envelopes, stable exit codes,
deterministic output, no network, no telemetry. The remaining work is
tightening the resource bounds, the durability story, and the parser robustness
in the corners — exactly the work that turns an MVP into something you can put
under load without flinching.
