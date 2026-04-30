---
name: axt-logdx
description: Use axt-logdx for bounded offline diagnosis of large local logs and command outputs.
license: MIT
---

# axt-logdx

Use `axt-logdx --agent` when a local log or command output is too large to read
directly and you need grouped failures, stack traces, timelines, and snippets.

## Rules

- Start with `axt-logdx <path> --severity error --top 20 --agent`.
- Use `--stdin` for piped command output.
- Use `--since` and `--until` only with RFC3339 timestamps.
- Read full files only after using returned fingerprints, snippets, and line
  numbers to narrow the follow-up.
- Do not use it for live tailing, remote log ingestion, dashboards, or general
  observability queries.
