# axt-logdx Evolution Brief

Status: proposed. Requires spec approval before implementation.

## Purpose

`axt-logdx` triages large logs and command outputs. It returns deduplicated
errors, stack traces, severity timelines, and top repeated failure groups within
strict output budgets.

## Market Position

Existing tools:

- Logdy and similar tools are strong for log viewing and streaming.
- Traditional Unix tools can filter logs, but require fragile pipelines and
  return noisy output.
- Observability systems are powerful but often remote, stateful, or too broad
  for local coding-agent debugging.

Market validity: medium-high.

Coverage and impact: medium-high. It is valuable when build, test, service, or
CI logs exceed useful model context.

Build decision: YES.

## Naming

- Binary: `axt-logdx`
- Optional alias: `logdx`
- Crate: `crates/axt-logdx`
- Schema prefix: `axt.logdx.v1`

Verify package-name availability again before publish.

## MVP Scope

- Read files and stdin.
- Detect plain text, JSONL, syslog-like timestamps, and common JavaScript,
  Python, Rust, Go, and JVM stack traces.
- Filter by severity and time range where parseable.
- Deduplicate repeated messages with counts and first/last occurrence.
- Extract top N groups and representative snippets.
- Enforce byte and record limits.

## Deferred Scope

- Live tailing.
- Remote log ingestion.
- OpenTelemetry trace graph reconstruction.
- Full query language.

## CLI Sketch

```bash
axt-logdx app.log --severity error --top 20 --json
cat build.log | axt-logdx --stdin --agent
axt-logdx ci.log --since 2026-04-28T10:00:00Z --agent
```

## Output Requirements

```json
{
  "sources": [{"path": "app.log", "lines": 120000, "bytes": 9000000}],
  "summary": {"lines": 120000, "groups": 12, "errors": 44, "warnings": 0, "bytes_scanned": 9000000, "truncated": false},
  "groups": [
    {
      "fingerprint": "blake3:...",
      "severity": "error",
      "count": 18,
      "first": {"source": "app.log", "line": 120, "timestamp": "2026-04-28T10:00:00Z"},
      "last": {"source": "app.log", "line": 8801, "timestamp": "2026-04-28T10:03:00Z"},
      "message": "connection refused",
      "stack": [],
      "snippets": ["..."]
    }
  ],
  "timeline": [{"bucket": "2026-04-28T10:00:00Z", "trace": 0, "debug": 0, "info": 0, "warn": 0, "error": 4, "fatal": 0}],
  "next": ["axt-logdx app.log --severity error --top 20 --agent"]
}
```

## Cross-Platform Matrix

| Feature | Linux | macOS | Windows |
|---|---:|---:|---:|
| File and stdin input | yes | yes | yes |
| CRLF logs | yes | yes | yes |
| Timezone parsing | yes | yes | yes |
| ANSI stripping | yes | yes | yes |

## Tests

- Fixtures for plain logs, JSONL logs, syslog-like logs, ANSI-colored logs,
  CRLF logs, and stack traces.
- Dedup fingerprint tests.
- Severity and time filter tests.
- Large-file streaming tests.
- Snapshot tests for all output modes.

## Skill Requirements

Create `docs/skills/axt-logdx/SKILL.md` with rules:

- Use for logs larger than a few hundred lines.
- Start with `--top 20 --agent`.
- Use returned fingerprints for focused follow-up context.

Update the skill installer after spec approval.
