# axt-plan Evolution Brief

Status: proposed. Requires spec approval before implementation.

## Purpose

`axt-plan` creates auditable edit plans for structural or regex-based changes.
It previews all changes, emits a stable plan, and can later apply exactly that
plan atomically.

## Market Position

Existing tools:

- `sed -i` is fast but unsafe for agents because broad replacements can corrupt
  files silently.
- ast-grep already provides high-quality structural search and rewrite in Rust.
- The gap is not the matching engine; it is a schema-versioned dry-run/apply
  workflow with validation and bounded agent output.

Market validity: medium-high.

Coverage and impact: high for safety-critical refactors.

Build decision: YES.

## Naming

- Binary: `axt-plan`
- Optional alias: `plan-edit`
- Crate: `crates/axt-plan`
- Schema prefix: `axt.plan.v1`

The binary name leaves room for future planned operations; the alias preserves
the edit-specific user wording.

## MVP Scope

- Dry-run only in the first implementation milestone.
- Support literal and regex replacement for text files.
- Support ast-grep-backed structural matching only when the dependency and
  language support are stable.
- Emit per-file hunks, match counts, plan checksum, and apply preconditions.
- Refuse ambiguous or binary-file edits.

## Apply Scope

Add only after dry-run schema stabilizes:

- `--apply <PLAN_FILE>`
- Verify file hashes and plan checksum before writing.
- Write atomically through temp files and rename.
- Optional backup directory.

## CLI Sketch

```bash
axt-plan --pattern 'old_name' --replacement 'new_name' src --json
axt-plan --lang rust --ast-pattern 'unwrap()' --replacement '?' --agent
axt-plan --apply plan.json --json
```

## Output Requirements

```json
{
  "plan_id": "sha256:...",
  "mode": "dry-run",
  "summary": {"files": 2, "replacements": 5, "truncated": false},
  "files": [
    {
      "path": "src/lib.rs",
      "matches": 3,
      "pre_hash": "blake3:...",
      "diff": "@@ ..."
    }
  ],
  "next": ["axt-plan --apply plan.json --agent"]
}
```

## Cross-Platform Matrix

| Feature | Linux | macOS | Windows |
|---|---:|---:|---:|
| Dry-run diff | yes | yes | yes |
| Atomic replace | yes | yes | yes |
| Symlink edit policy | yes | yes | limited; document refusal/default |
| File permission preservation | yes | yes | partial; document Windows behavior |

## Tests

- Literal replacement fixture tests.
- Regex replacement fixture tests.
- No-match tests.
- Binary-file refusal tests.
- Plan checksum and stale-file apply tests.
- Atomic write failure tests where practical.
- Snapshot tests for all output modes.

## Skill Requirements

Create `docs/skills/axt-plan/SKILL.md` with rules:

- Use dry-run before broad replacements.
- Inspect match counts and diffs before applying.
- Never use `--apply` on a plan generated from different file hashes.

Update the skill installer after spec approval.
