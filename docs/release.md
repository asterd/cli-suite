# Release Runbook

The release pipeline is managed by `dist` (`cargo-dist`). Releases are cut from signed git tags and must keep diagnostics on stderr and data on stdout for every shipped binary.

## Preflight

Run the local gates from a clean working tree:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
python3 -m unittest scripts/release/test_scoop_manifest.py
dist generate --check
dist plan --tag v0.1.0-rc1 --output-format=json
```

Confirm the `dist plan` matrix includes the tier-1 targets from `docs/spec.md` section 12.2:

- `x86_64-unknown-linux-gnu`
- `x86_64-unknown-linux-musl`
- `aarch64-unknown-linux-gnu`
- `x86_64-apple-darwin`
- `aarch64-apple-darwin`
- `x86_64-pc-windows-msvc`

The `aarch64-pc-windows-msvc` tier-2 target is best-effort and does not gate v0.1.

## Repository Secrets And Variables

The release workflow needs the standard `GITHUB_TOKEN` plus:

| Name | Type | Purpose |
|---|---|---|
| `SCOOP_BUCKET_REPOSITORY` | repository variable | Target Scoop bucket repository, for example `ddurzo/scoop-axt`. |
| `SCOOP_BUCKET_TOKEN` | repository secret | Token with permission to push a branch and open a PR in the Scoop bucket. |

If `SCOOP_BUCKET_REPOSITORY` is set, `SCOOP_BUCKET_TOKEN` is required. A missing
token intentionally fails the Scoop job so the release is not silently published
without the Windows bucket update.

## Version And Changelog

For the release candidate, the workspace version is `0.1.0-rc1`. All binary crates use the workspace version. Internal crates that must be published for `cargo install axt-peek` also use the same version.

Update `CHANGELOG.md` in Keep-a-Changelog style before tagging. Group user-visible changes under the binary name, and keep release-pipeline changes separate.

## Cargo Publish Order

`axt-peek` depends on internal library crates, so publish or dry-run them in dependency order:

```bash
cargo publish -p axt-core --dry-run
cargo publish -p axt-output --dry-run
cargo publish -p axt-fs --dry-run
cargo publish -p axt-git --dry-run
cargo publish -p axt-peek --dry-run
```

For a real release, remove `--dry-run` and wait for each crate to be available on crates.io before publishing dependents.

## Tagging

Create the release-candidate tag only after all local checks pass:

```bash
git tag v0.1.0-rc1 -s -m "axt v0.1.0-rc1"
git push origin v0.1.0-rc1
```

The GitHub release workflow builds archives, shell and PowerShell installers, the Homebrew formula, and checksums. The release candidate must remain a prerelease.

The workflow also attests release artifacts and uploads a CycloneDX SBOM to the
GitHub Release. The normal CI workflow runs `cargo-audit` and `cargo-deny` before
release tags should be pushed.

## Scoop Manifest

The release workflow generates the Scoop manifest from the final dist manifest
and Windows archive checksum, then opens a PR against `SCOOP_BUCKET_REPOSITORY`.
To reproduce that step locally:

```bash
python3 scripts/release/scoop-manifest.py \
  --dist-manifest dist-manifest.json \
  --sha256-file axt-peek-x86_64-pc-windows-msvc.zip.sha256 \
  --output bucket/axt-peek.json
```

Open a pull request against `ddurzo/scoop-axt` with the generated
`bucket/axt-peek.json` only if the automated workflow cannot reach the bucket.

## Smoke Tests

Run each install from a clean machine or VM and verify `axt-peek --version` and `axt-peek --help`.

Linux or macOS shell installer:

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/ddurzo/axt/releases/download/v0.1.0-rc1/axt-peek-installer.sh | sh
axt-peek --version
axt-peek --help
```

macOS or Linux Homebrew:

```bash
brew install ddurzo/axt/axt-peek
axt-peek --version
axt-peek --help
```

Windows PowerShell:

```powershell
powershell -ExecutionPolicy Bypass -c "irm https://github.com/ddurzo/axt/releases/download/v0.1.0-rc1/axt-peek-installer.ps1 | iex"
axt-peek --version
axt-peek --help
```

Windows Scoop:

```powershell
scoop bucket add axt https://github.com/ddurzo/scoop-axt
scoop install axt-peek
axt-peek --version
axt-peek --help
```

Cargo:

```bash
cargo install axt-peek --version 0.1.0-rc1
axt-peek --version
axt-peek --help
```

Optional unprefixed local alias:

```bash
cargo install axt-peek --version 0.1.0-rc1 --features aliases
peek --version
```

## Recovery

If smoke tests fail, diagnose before yanking. If the release is unsafe or unusable:

1. Mark the GitHub Release as draft or prerelease.
2. Delete the bad tag locally and remotely only after confirming replacement strategy.
3. Run `cargo yank` for crates already published to crates.io.
4. Cut a new release candidate with a new prerelease version.

Promote `v0.1.0` only after the release-candidate install matrix passes.
