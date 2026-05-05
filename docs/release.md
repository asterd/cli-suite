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

For the release candidate, the workspace version is `0.1.0-rc1`. All binary crates use the workspace version. Internal crates that must be published for `cargo install axt-*` also use the same version.

Update `CHANGELOG.md` in Keep-a-Changelog style before tagging. Group user-visible changes under the binary name, and keep release-pipeline changes separate.

### Schema versioning

Public JSON and agent JSONL schemas are versioned per binary (`axt.<cmd>.v1`,
`axt.<cmd>.v2`, ...). The contract starts at the first published release
(`v0.1.0`). Pre-release stub payloads — anything shipped before that tag,
including the M0/M1 placeholders — do not establish the contract; we may evolve
their shape without bumping the schema major. Once `v0.1.0` is on crates.io and
the GitHub release exists, any breaking shape change requires a major schema
bump (`v1` → `v2`, the new schema id committed under `schemas/`, both schemas
served in parallel for at least one release for migration), regardless of how
small the change looks.

## Cargo Publish Order

The binaries depend on internal library crates, so publish or dry-run them in dependency order. Library crates first, then binaries:

```bash
cargo publish -p axt-core --dry-run
cargo publish -p axt-output --dry-run
cargo publish -p axt-fs --dry-run
cargo publish -p axt-git --dry-run

cargo publish -p axt-peek --dry-run
cargo publish -p axt-run --dry-run
cargo publish -p axt-doc --dry-run
cargo publish -p axt-drift --dry-run
cargo publish -p axt-port --dry-run
cargo publish -p axt-test --dry-run
cargo publish -p axt-outline --dry-run
cargo publish -p axt-slice --dry-run
cargo publish -p axt-ctxpack --dry-run
cargo publish -p axt-bundle --dry-run
cargo publish -p axt-gitctx --dry-run
cargo publish -p axt-logdx --dry-run
```

For a real release, remove `--dry-run` and wait for each crate to be available on crates.io before publishing dependents.

## Tagging

Create the release-candidate tag only after all local checks pass:

```bash
git tag v0.1.0-rc1 -s -m "axt v0.1.0-rc1"
git push origin v0.1.0-rc1
```

The GitHub release workflow builds archives, shell and PowerShell installers, the Homebrew formula, and checksums. The release candidate must remain a prerelease.

The workflow also packages `README.md`, `docs/installation.md`, manpages, and
agent skills as `axt-docs.tar.gz`, attests release artifacts, and uploads a
CycloneDX SBOM to the GitHub Release. The normal CI workflow runs `cargo-audit`
and `cargo-deny` before release tags should be pushed.

## Scoop Manifest

The release workflow generates a Scoop manifest per binary from the final dist
manifest and the matching Windows archive checksum, then opens a PR against
`SCOOP_BUCKET_REPOSITORY`. To reproduce that step locally for a single binary:

```bash
python3 scripts/release/scoop-manifest.py \
  --dist-manifest dist-manifest.json \
  --sha256-file axt-peek-x86_64-pc-windows-msvc.zip.sha256 \
  --output bucket/axt-peek.json
```

Repeat for the other binaries (`axt-run`, `axt-doc`, `axt-drift`, `axt-port`,
`axt-test`, `axt-outline`, `axt-slice`, `axt-ctxpack`, `axt-bundle`,
`axt-gitctx`, `axt-logdx`) by swapping the `--sha256-file` and `--output`
arguments to match each binary name. Open a pull request against
`ddurzo/scoop-axt` with the generated manifests only if the automated workflow
cannot reach the bucket.

## Smoke Tests

Run each install from a clean machine or VM and verify `--version` and
`--help` for every shipped binary: `axt-peek`, `axt-run`, `axt-doc`,
`axt-drift`, `axt-port`, `axt-test`, `axt-outline`, `axt-slice`,
`axt-ctxpack`, `axt-bundle`, `axt-gitctx`, and `axt-logdx`.

Use this helper after each install so all shipped binaries are exercised:

```bash
for bin in axt-peek axt-run axt-doc axt-drift axt-port axt-test axt-outline axt-slice axt-ctxpack axt-bundle axt-gitctx axt-logdx; do
  "$bin" --version
  "$bin" --help >/dev/null
done
```

Linux or macOS shell installer (one binary per installer URL — repeat for each):

```bash
for bin in axt-peek axt-run axt-doc axt-drift axt-port axt-test axt-outline axt-slice axt-ctxpack axt-bundle axt-gitctx axt-logdx; do
  curl --proto '=https' --tlsv1.2 -LsSf \
    "https://github.com/ddurzo/axt/releases/download/v0.1.0-rc1/${bin}-installer.sh" | sh
done
```

macOS or Linux Homebrew:

```bash
for bin in axt-peek axt-run axt-doc axt-drift axt-port axt-test axt-outline axt-slice axt-ctxpack axt-bundle axt-gitctx axt-logdx; do
  brew install "ddurzo/axt/${bin}"
done
```

Windows PowerShell:

```powershell
$bins = @('axt-peek','axt-run','axt-doc','axt-drift','axt-port','axt-test','axt-outline','axt-slice','axt-ctxpack','axt-bundle','axt-gitctx','axt-logdx')
foreach ($bin in $bins) {
  powershell -ExecutionPolicy Bypass -c "irm https://github.com/ddurzo/axt/releases/download/v0.1.0-rc1/$bin-installer.ps1 | iex"
}
```

Windows Scoop:

```powershell
scoop bucket add axt https://github.com/ddurzo/scoop-axt
foreach ($bin in 'axt-peek','axt-run','axt-doc','axt-drift','axt-port','axt-test','axt-outline','axt-slice','axt-ctxpack','axt-bundle','axt-gitctx','axt-logdx') {
  scoop install $bin
}
```

Cargo:

```bash
for bin in axt-peek axt-run axt-doc axt-drift axt-port axt-test axt-outline axt-slice axt-ctxpack axt-bundle axt-gitctx axt-logdx; do
  cargo install "$bin" --version 0.1.0-rc1
done
```

## Recovery

If smoke tests fail, diagnose before yanking. If the release is unsafe or unusable:

1. Mark the GitHub Release as draft or prerelease.
2. Delete the bad tag locally and remotely only after confirming replacement strategy.
3. Run `cargo yank` for crates already published to crates.io.
4. Cut a new release candidate with a new prerelease version.

Promote `v0.1.0` only after the release-candidate install matrix passes.
