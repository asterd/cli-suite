# Agent Mode

Agent mode emits ACF: Agent Compact Format. It is the LLM-first output format for `--agent`.

ACF is line-oriented text:

```text
schema=axt.peek.agent.v1 ok=true mode=table root=. cols=path,kind,bytes,lang,git rows=3 total=42 truncated=false
Cargo.toml,file,2102,toml,clean
README.md,file,8902,markdown,modified
src,dir,0,,mixed
```

## Contract

- The first line is always the summary/schema line.
- The first line includes `schema`, `ok`, `mode`, and `truncated`.
- `mode=table` declares `cols` once; following rows are comma-separated values in that order.
- `mode=records` uses uppercase record prefixes such as `X`, `W`, `F`, and `S`.
- Values are raw unless they contain whitespace, commas, quotes, or control characters; those values are JSON-string quoted.
- There is no ANSI color, no decorative prose, and no human-only unit formatting.
- Paths are relative when possible.
- Truncation is explicit: `W code=truncated reason=max_records truncated=true`.

## Shared Keys

```text
schema     schema identifier, usually axt.<command>.agent.v<N>
ok         bool, top-level success
mode       records|table
cols       comma-separated table columns
rows       emitted row count
total      total row count before truncation/filtering
truncated  bool, output was truncated
root       repo-relative root when relevant
path       repo-relative path when possible
kind       file|dir|symlink|other or command-specific entity kind
bytes      raw byte count
ms         milliseconds
ts         RFC 3339 UTC timestamp
lang       language (rust, python, ts, ...)
git        git status (clean|modified|untracked|added|deleted|renamed|mixed)
mime       mime type
enc        encoding (utf-8, utf-16le, ...)
nl         newline style (lf|crlf|mixed|none)
generated  bool, "looks generated"
code       error or warning code
hint       human-or-agent next-step suggestion
```

## Shared Prefixes

```text
X  fatal or command-level error
W  warning
E  stderr/stdout excerpt or error line
F  file record
S  suggested next command
```

Command-specific keys and prefixes are documented in `docs/commands/<cmd>.md`.
`axt-outline` uses `Y` for symbol records.

`--jsonl` is the NDJSON/JSONL mode for streaming and pipelines. It is separate from agent mode.
