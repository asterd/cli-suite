# Contributing

This repository follows the implementation milestones in `docs/spec.md` and `docs/spec-addendum.md`.

Before opening a change:

- Stay inside the active milestone.
- Run `cargo fmt --all --check`.
- Run `cargo clippy --workspace --all-targets -- -D warnings`.
- Run `cargo test --workspace`.
- Use conventional commits.
