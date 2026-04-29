# axt-outline Evolution Brief

Status: implemented in `crates/axt-outline`. This brief is retained as product
rationale; the live contract is `docs/commands/outline.md` plus the approved
spec addendum.

## Purpose

`axt-outline` emits declarations, signatures, doc comments, and symbol ranges
without function bodies. It gives agents a compact view of a file, directory, or
module public surface.

## Market Position

Existing tools:

- Aider uses repository maps with tree-sitter to expose key symbols.
- Repomix can compress code by keeping structure/signatures.
- IDEs and LSPs expose outlines interactively, but not as a portable
  schema-first CLI for agents.

Market validity: high.

Coverage and impact: high. This is one of the best token-reduction candidates
because agents often read large files only to discover available symbols.

Build decision: YES.

## Naming

- Binary: `axt-outline`
- Optional alias: `outline`
- Crate: `crates/axt-outline`
- Schema prefix: `axt.outline.v1`

Verify package-name availability again before publish.

## MVP Scope

- Accept files and directories.
- Emit symbols for Rust, TypeScript/JavaScript, Python, Go, Java, and PHP
  through embedded tree-sitter grammars.
- Include symbol kind, name, visibility, signature, doc comment summary,
  source range, parent symbol, and file path.
- Support `--public-only`, `--private`, `--tests`, `--lang`, and `--max-depth`.
- Preserve source order by default; support `--sort path|name|kind`.

## Deferred Scope

- Cross-file reference ranking.
- Semantic symbol importance.
- Full repository graph computation.
- LSP dependency as a hard requirement.

## CLI Sketch

```bash
axt-outline crates/axt-test/src --public-only --json
axt-outline src/lib.rs --agent
axt-outline . --lang rust --max-depth 4 --agent
```

## Output Requirements

All standard output modes are mandatory: human, `--json`, and `--agent`. Agent
mode is summary-first JSONL and should be compact enough to replace line-range
reads:

```jsonl
{"schema":"axt.outline.summary.v1","type":"summary","ok":true,"root":".","files":2,"symbols":14,"warnings":0,"source_bytes":12000,"signature_bytes":900,"truncated":false,"next":["axt-slice src/lib.rs --symbol parse_config --agent"]}
{"schema":"axt.outline.symbol.v1","type":"symbol","p":"src/lib.rs","l":"rust","k":"fn","vis":"pub","n":"parse_config","sig":"pub fn parse_config(input: &str) -> Result<Config, Error>","docs":"Parse the configuration text.","range":{"start_line":42,"end_line":57},"parent":null}
```

## JSON Data Shape

```json
{
  "root": ".",
  "summary": {"files": 2, "symbols": 14, "truncated": false},
  "symbols": [
    {
      "path": "src/lib.rs",
      "kind": "fn",
      "visibility": "pub",
      "name": "parse_config",
      "signature": "pub fn parse_config(input: &str) -> Result<Config, Error>",
      "docs": "Parse the configuration text.",
      "range": {"start_line": 42, "end_line": 57}
    }
  ],
  "next": ["axt-slice src/lib.rs --symbol parse_config --agent"]
}
```

## Cross-Platform Matrix

| Feature | Linux | macOS | Windows |
|---|---:|---:|---:|
| Directory traversal | yes | yes | yes |
| Rust outlines | yes | yes | yes |
| TS/JS outlines | yes | yes | yes |
| Python outlines | yes | yes | yes |
| Go outlines | yes | yes | yes |
| Java outlines | yes | yes | yes |
| PHP outlines | yes | yes | yes |
| LSP ranking | deferred | deferred | deferred |

## Tests

- Snapshot tests for all output modes.
- Fixtures for each supported language.
- Visibility and doc-comment extraction tests.
- Truncation tests by record count and byte budget.
- Error tests for unsupported language, unreadable path, parse failure, and
  mixed valid/invalid files.

## Skill Requirements

Create `docs/skills/axt-outline/SKILL.md` with rules:

- Use `axt-outline --agent` before reading large source files.
- Use `--public-only` when API surface is enough.
- Use `axt-slice` for full bodies after selecting a symbol.

Update the skill installer after spec approval.
