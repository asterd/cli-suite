---
name: axt-slice
description: Use axt-slice to extract local source by symbol or enclosing line with stable parser-derived ranges.
license: MIT OR Apache-2.0
---

# axt-slice

Use `axt-slice` when you need source for a known function, method, class, impl,
or nearby line. Prefer it over `sed -n`, `head`, or `tail` because symbol ranges
remain stable after edits.

## Rules

- Prefer `axt-slice <file> --symbol <name>` over manual line ranges.
- If the symbol is ambiguous, retry with a qualified query such as
  `Parser::parse` or inspect candidates with `axt-outline <file> --agent`.
- If only a line is known, use `axt-slice <file> --line <N>` and let the command
  expand to the enclosing symbol.
- Use `--include-imports=all` when the returned code must compile or when all
  top-of-file imports clarify type names.
- Use `--include-imports=matched` when you want a smaller local slice and
  syntactic identifier matching is enough.
- Use `--before-symbol` or `--after-symbol` only when adjacent context is needed.
- Use `--include-tests` when validating behavior around the selected symbol.

## Examples

```bash
axt-slice crates/axt-slice/src/command.rs --symbol run --agent
axt-slice crates/axt-slice/src/tree.rs --symbol fn::parse_source --json
axt-slice crates/axt-slice/src/tree.rs --line 150 --include-imports=matched
```
