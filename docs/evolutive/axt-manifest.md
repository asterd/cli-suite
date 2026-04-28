# axt-manifest Evolution Brief

Status: proposed. Requires spec approval before implementation.

## Purpose

`axt-manifest` reads project configuration files and emits a normalized schema
for dependencies, scripts, runtime versions, build targets, CI jobs, and tool
configuration.

## Market Position

Existing tools:

- Each ecosystem has parsers and native commands for its own manifest.
- Cargo manifest parser crates exist for Rust.
- No dominant local single-binary CLI appears to normalize `package.json`,
  `Cargo.toml`, `pyproject.toml`, `go.mod`, Dockerfiles, and CI files into one
  agent-first schema.

Market validity: medium.

Coverage and impact: medium. It removes repeated manifest reads and helps
agents answer dependency/build questions quickly.

Build decision: YES, later.

## Naming

- Binary: `axt-manifest`
- Optional alias: `manifest`
- Crate: `crates/axt-manifest`
- Schema prefix: `axt.manifest.v1`

Use singular naming for consistency with one normalized manifest graph.

## MVP Scope

- Detect and parse:
  - `Cargo.toml`
  - `package.json`
  - `tsconfig.json`
  - `pyproject.toml`
  - `go.mod`
  - `Dockerfile`
  - `.github/workflows/*.yml`
- Emit dependencies, dev dependencies, scripts/tasks, runtime versions, package
  names, workspace members, and CI job names.
- Preserve raw unknown sections as `unknown` counts or summaries, not silent
  drops.

## Deferred Scope

- Vulnerability checking.
- Lockfile solving.
- Network package metadata.
- Full Dockerfile semantic analysis.

## CLI Sketch

```bash
axt-manifest --json
axt-manifest --root . --include-ci --agent
axt-manifest --ecosystem rust --json-data
```

## Output Requirements

```json
{
  "root": ".",
  "ecosystems": ["rust", "node"],
  "packages": [{"name": "axt-core", "ecosystem": "rust", "path": "crates/axt-core/Cargo.toml"}],
  "dependencies": [{"name": "serde", "ecosystem": "rust", "scope": "normal"}],
  "scripts": [{"name": "test", "command": "cargo test --workspace"}],
  "ci_jobs": [{"path": ".github/workflows/ci.yml", "name": "test"}],
  "next": ["axt-test --agent"]
}
```

## Cross-Platform Matrix

| Feature | Linux | macOS | Windows |
|---|---:|---:|---:|
| JSON/TOML/YAML parsing | yes | yes | yes |
| Dockerfile detection | yes | yes | yes |
| Path normalization | yes | yes | yes |
| Shell command interpretation | limited | limited | limited |

## Tests

- Ecosystem-specific fixture manifests.
- Malformed manifest error tests.
- Multi-workspace fixture tests.
- Unknown section preservation tests.
- Snapshot tests for all output modes.

## Skill Requirements

Create `docs/skills/axt-manifest/SKILL.md` with rules:

- Use before reading raw manifest files one by one.
- Use `--ecosystem` to narrow output in large monorepos.
- Do not infer installed package versions from manifests alone.

Update the skill installer after spec approval.
