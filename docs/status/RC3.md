# RC3 / Wave 3 Status Report

## What was implemented
- Added configurable `axt-port free --kill-grace` for the post-kill recheck delay.
- Hardened `axt-port` parent-process refusal so unknown parent PID resolution refuses unless `--force-self` is used.
- Added `axt-drift --hash-max-bytes` to `mark`, `diff`, and `run`, with `hash_skipped_size` recorded in snapshots and output.
- Parallelized `axt-drift` hashing with `rayon` while preserving deterministic sorted snapshot output.
- Added `axt-fs::read_to_string_smart` and `decode_text_smart` for UTF-8, UTF-16 BOM, and Windows-1252 fallback decoding.
- Adopted the encoding helper in `axt-bundle`, `axt-gitctx`, and `axt-logdx`.
- Flushed `axt-output` truncation warnings before returning from `AgentJsonlWriter::finish`.
- Added property tests for `axt-test` frontend parsing, `axt-logdx` log parsing, and `axt-gitctx` porcelain parsing.
- Added `cargo-fuzz` targets for the same three parser surfaces.

## What was not implemented and why
- No CI weekly fuzz job was added; Wave 3 asked for fuzz targets, while CI scheduling belongs with the broader OS smoke matrix work.

## Deviations from the spec
- None from the approved CLI surface. `axt-drift` schema was extended with `hash_skipped_size` to make skipped hashes explicit.

## How to run tests
```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo check --manifest-path fuzz/Cargo.toml
```

## How to run the release pipeline locally
- Not run in this hardening session. Use the existing release dry-run flow from `docs/agent-prompts.md` section 10 when preparing the next RC tag.

## Quality gates: pass / fail
- fmt: pass
- clippy: pass
- test: pass
- fuzz target compile: pass
- CI on Linux/macOS/Windows: not run locally

## Open questions for the maintainer
- Decide whether fuzz targets should be wired into a scheduled CI workflow in the next session.
- Decide whether encoding-conversion warnings should become structured JSON envelope warnings for `axt-bundle`/`axt-gitctx`, rather than stderr diagnostics.

## Next recommended milestone
- Wave 4: polish items and OS smoke matrix expansion.
