# axt-logsift Evolution Brief

Status: proposed. Requires spec approval before implementation.

## Purpose

`axt-logsift` triages large logs and command outputs. It returns deduplicated
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

- Binary: `axt-logsift`
- Optional alias: `logsift`
- Crate: `crates/axt-logsift`
- Schema prefix: `axt.logsift.v1`

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
axt-logsift app.log --severity error --top 20 --json
cat build.log | axt-logsift --stdin --agent
axt-logsift ci.log --since 1h --dedup --jsonl
```

## Output Requirements

```json
{
  "source": "app.log",
  "summary": {"lines": 120000, "groups": 12, "errors": 44, "truncated": false},
  "groups": [
    {
      "fingerprint": "sha256:...",
      "severity": "error",
      "count": 18,
      "first_line": 120,
      "last_line": 8801,
      "message": "connection refused",
      "sample": "..."
    }
  ],
  "timeline": [{"bucket": "2026-04-28T10:00:00Z", "error": 4}],
  "next": ["axt-logsift app.log --fingerprint sha256:... --context 20 --agent"]
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

Create `docs/skills/axt-logsift/SKILL.md` with rules:

- Use for logs larger than a few hundred lines.
- Start with `--top 20 --agent`.
- Use returned fingerprints for focused follow-up context.

Update the skill installer after spec approval.
