# axt-slice Evolution Brief

Status: proposed. Requires spec approval before implementation.

## Purpose

`axt-slice` extracts source by symbol, not fragile line ranges. It returns the
exact declaration or implementation block, optionally including nearby docs,
attributes, imports, and tests.

## Market Position

Existing tools:

- `sed -n`, `head`, and `tail` are line-oriented and brittle after edits.
- LSP clients can navigate symbols but are not standardized as simple
  schema-first CLIs.
- Probe has code extraction features, validating the need, but it is a broader
  context engine.

Market validity: high.

Coverage and impact: high. It is the natural follow-up to `axt-outline` and
reduces incorrect line-range reads.

Build decision: YES.

## Naming

- Binary: `axt-slice`
- Optional alias: `slice`
- Crate: `crates/axt-slice`
- Schema prefix: `axt.slice.v1`

Verify package-name availability again before publish.

## MVP Scope

- Extract by `--symbol <NAME>` from one file.
- Extract by `--line <N>` as compatibility fallback, expanding to the enclosing
  symbol.
- Include docs and attributes by default.
- Optional `--include-imports`, `--include-tests`, `--before-symbol`, and
  `--after-symbol`.
- Detect ambiguous symbols and return candidates instead of guessing.

## Deferred Scope

- Whole-workspace symbol resolution.
- Cross-language LSP integration.
- Edit application.

## CLI Sketch

```bash
axt-slice src/lib.rs --symbol parse_config --json
axt-slice src/lib.rs --line 120 --agent
axt-slice src/lib.rs --symbol Parser::parse --include-imports --plain
```

## Output Requirements

The JSON envelope must include the selected symbol, exact range, source text,
ambiguity metadata, and next-step hints.

```json
{
  "path": "src/lib.rs",
  "selection": {"kind": "symbol", "query": "parse_config"},
  "symbol": {"name": "parse_config", "kind": "fn"},
  "range": {"start_line": 42, "end_line": 57},
  "source": "pub fn parse_config(...) { ... }",
  "next": ["axt-impact src/lib.rs --symbol parse_config --agent"]
}
```

## Cross-Platform Matrix

| Feature | Linux | macOS | Windows |
|---|---:|---:|---:|
| UTF-8 source extraction | yes | yes | yes |
| CRLF preservation | yes | yes | yes |
| Symbol extraction | parser-dependent | parser-dependent | parser-dependent |
| Import inclusion | language-dependent | language-dependent | language-dependent |

## Tests

- Exact extraction snapshots for Rust, TypeScript, Python, and Go fixtures.
- Ambiguous symbol tests.
- Line-to-enclosing-symbol tests.
- CRLF fixture tests.
- Byte and record truncation tests.
- `--include-imports` tests per language where supported.

## Skill Requirements

Create `docs/skills/axt-slice/SKILL.md` with rules:

- Prefer `axt-slice --symbol` over `sed -n` for source reads.
- If only a line is known, use `--line` and let the command expand safely.
- If ambiguity is returned, call `axt-outline` to choose the intended symbol.

Update the skill installer after spec approval.
