# axt-impact Evolution Brief

Status: proposed research-track command. Requires spec approval before
implementation.

## Purpose

`axt-impact` estimates the blast radius of changing a symbol. It returns call
sites, references, nearby tests, and suggested review files.

## Market Position

Existing tools:

- IDEs and LSP servers provide references and symbol navigation.
- Sourcegraph/Cody uses search and code graph context.
- MCP-based coding tools can expose LSP functions to agents.

Market validity: high. The value is proven, but a portable CLI implementation
is hard.

Coverage and impact: very high. It could replace many manual grep/read/test
selection loops.

Build decision: YES, but only after a research milestone.

## Naming

- Binary: `axt-impact`
- Optional alias: `impact`
- Crate: `crates/axt-impact`
- Schema prefix: `axt.impact.v1`

Verify package-name availability again before publish.

## MVP Scope

- Rust-first implementation.
- Use `rust-analyzer` if available and project configuration is valid.
- Fall back to local text/tree-sitter references with lower confidence.
- Return call sites with file, line, symbol kind, confidence, and snippet.
- Suggest test files based on references, naming conventions, and git history
  where available.

## Deferred Scope

- Full multi-language LSP matrix.
- Type-aware call graph construction for every language.
- Build-system execution.
- Network code intelligence.

## CLI Sketch

```bash
axt-impact src/config.rs --symbol parse_config --json
axt-impact --file src/config.rs --line 42 --agent
axt-impact --symbol ConfigLoader::load --max-depth 2 --jsonl
```

## Output Requirements

```json
{
  "target": {"path": "src/config.rs", "symbol": "parse_config"},
  "engine": {"kind": "lsp", "name": "rust-analyzer", "confidence": "high"},
  "call_sites": [{"path": "src/main.rs", "line": 18, "kind": "call", "confidence": "high"}],
  "tests_touching": [{"path": "tests/config.rs", "reason": "references target"}],
  "suggested_review_files": ["src/main.rs", "tests/config.rs"],
  "next": ["axt-test --files tests/config.rs --agent"]
}
```

## Cross-Platform Matrix

| Feature | Linux | macOS | Windows |
|---|---:|---:|---:|
| Text fallback | yes | yes | yes |
| Tree-sitter fallback | parser-dependent | parser-dependent | parser-dependent |
| Rust LSP mode | yes, if installed | yes, if installed | yes, if installed |
| Process lifecycle | yes | yes | yes, with Windows-specific tests |

## Tests

- Rust fixture workspace with known references.
- LSP-unavailable fallback tests.
- Ambiguous symbol tests.
- Confidence scoring tests.
- Timeout and process-failure tests.
- Snapshot tests for all output modes.

## Skill Requirements

Create `docs/skills/axt-impact/SKILL.md` with rules:

- Use before changing shared functions, public types, or exported APIs.
- Treat fallback results as incomplete when confidence is not high.
- Use suggested tests, but do not assume they are exhaustive.

Update the skill installer after spec approval.
