# axt-repomap Evolution Brief

Status: proposed. Requires spec approval before implementation.

## Purpose

`axt-repomap` returns a compact repository topology summary: layout, detected
languages, build systems, entry points, test frameworks, manifests, and recent
local git context.

## Market Position

Existing tools:

- Aider has a mature repository map focused on symbols and relevance.
- Repomix packages entire repositories into AI-ready formats and includes
  tree-sitter compression.
- `onefetch` summarizes repositories for humans.

Market validity: medium. The space is crowded, so `axt-repomap` must be a small
schema-first topology command, not a full packer.

Coverage and impact: medium-high. Useful at session start to avoid repeated
`ls`, manifest reads, test discovery, and git summaries.

Build decision: YES, later.

## Naming

- Binary: `axt-repomap`
- Optional alias: `repomap`
- Crate: `crates/axt-repomap`
- Schema prefix: `axt.repomap.v1`

Verify package-name availability again before publish.

## MVP Scope

- Detect repository root and workspace layout.
- Summarize directories by role: `source`, `tests`, `examples`, `docs`,
  `scripts`, `config`, `generated`, `vendor`.
- Reuse or align with `axt-peek`, `axt-test`, `axt-git`, and future
  `axt-manifest` data shapes.
- Include recent local commits when git is available.
- Include `next` hints for `axt-outline`, `axt-test`, and `axt-gitctx`.

## Deferred Scope

- Full source packing.
- Embedding or semantic ranking.
- Remote repository support.
- Package-manager network queries.

## CLI Sketch

```bash
axt-repomap --json
axt-repomap . --agent --depth 3
axt-repomap --include-git --include-tests --jsonl
```

## Output Requirements

```json
{
  "root": ".",
  "summary": {
    "primary_language": "rust",
    "build_systems": ["cargo"],
    "test_frameworks": ["cargo"],
    "git_state": "dirty"
  },
  "layout": [{"path": "crates", "role": "source", "children": 7}],
  "entry_points": [{"path": "crates/axt-test/src/main.rs", "kind": "binary"}],
  "next": ["axt-outline crates --public-only --agent"]
}
```

## Cross-Platform Matrix

| Feature | Linux | macOS | Windows |
|---|---:|---:|---:|
| Layout detection | yes | yes | yes |
| Manifest detection | yes | yes | yes |
| Git summary | yes | yes | yes, if git available |
| Executable-bit hints | yes | yes | no; report unsupported |

## Tests

- Multi-language fixture repositories.
- Monorepo/workspace fixtures.
- Dirty and clean git fixtures.
- No-git directory fixture.
- Truncation and role-classification tests.

## Skill Requirements

Create `docs/skills/axt-repomap/SKILL.md` with rules:

- Use at the start of unfamiliar repository sessions.
- Use `axt-outline` for symbol detail after topology selection.
- Do not use as a replacement for exact file reads.

Update the skill installer after spec approval.
