---
name: axt-ctxpack
description: Use axt-ctxpack for bounded multi-pattern local context search in axt workspaces.
license: MIT
---

# axt-ctxpack

Use `axt-ctxpack --agent` when searching for several related patterns in local files and you need dense hit context with tree-sitter classification.

## Rules

- Prefer named patterns over separate command calls.
- Use `--context 0` first when only locations are needed.
- Add `--include` or `--files` globs to keep scans narrow.
- Use `kind`, `node`, `symbol`, and `src` in agent output to distinguish code, comments, strings, and tests before reading full files.
- Treat `src=ast` as parser-backed classification and `src=heuristic` as a weaker fallback.
- Do not use it for semantic search, embeddings, edit application, or AST queries.
