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
- Use `--since` and `--until` with RFC3339 filter arguments. Input logs may
  contain RFC3339 or Unix epoch seconds/milliseconds.
- Treat `input_truncated`, `time_unparseable`, and `invalid_utf8` warnings as
  signals that the result is still useful but approximate.
- Read full files only after using returned fingerprints, snippets, and line
  numbers to narrow the follow-up.
- Do not use it for live tailing, remote log ingestion, dashboards, or general
  observability queries.
