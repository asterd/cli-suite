# ax-peek

`ax-peek` is currently the Milestone 1 proof binary for the shared `ax-core`
and `ax-output` scaffolding. Real directory, filesystem, and git snapshot
behavior starts in Milestone 3.

## Purpose

For Milestone 1, `ax-peek` proves that a command can emit the same stub payload
through the shared human, JSON, JSON data, JSONL, and agent renderers.

## Examples

```bash
ax-peek
ax-peek --json
ax-peek --json-data
ax-peek --jsonl
ax-peek --agent
ax-peek --list-errors
ax-peek --print-schema
```

## Output Samples

Human mode:

```text
ax-peek stub: Milestone 0 scaffolding only
```

JSON mode:

```json
{"schema":"ax.peek.v1","ok":true,"data":{"status":"stub"},"warnings":[],"errors":[]}
```

JSONL mode:

```json
{"schema":"ax.peek.summary.v1","type":"summary","ok":true,"stub":true}
```

Agent mode:

```text
schema=ax.peek.agent.v1 ok=true mode=records stub=true truncated=false
```

## Flags

- `--json`: emit the standard JSON envelope.
- `--json-data`: emit only the JSON `data` payload.
- `--jsonl`: emit newline-delimited JSON for streaming and pipelines.
- `--agent`: emit agent-oriented ACF.
- `--plain`: emit plain human-readable output.
- `--limit <N>`: cap normal agent records before truncation metadata.
- `--max-bytes <BYTES>`: cap normal agent record bytes before truncation metadata.
- `--strict`: exit non-zero when truncation is required.
- `--print-schema`: print the current `ax.peek.v1` schema.
- `--list-errors`: print the standard error catalog as JSONL.

The mode flags `--json`, `--json-data`, `--jsonl`, `--agent`, and `--plain` are
mutually exclusive.

## Error Codes

`--list-errors` prints the full standard catalog from `ax-core`. The M1 stub
does not perform filesystem or git work, so it should only fail for CLI usage,
I/O, serialization, or strict truncation errors.

## Performance

The M1 stub writes a constant-size payload. Real performance targets apply when
`ax-peek` traversal is implemented in Milestone 3.

## Cross-Platform Notes

The M1 stub is platform-neutral. It performs no platform-specific filesystem or
git operations.

## Agent Usage

Agents should read the first ACF line as the summary/schema line. If output is
truncated, `ax-peek` still emits the summary first and appends a compact
`W code=truncated` warning record. Use `--jsonl` when a pipeline or harness needs
NDJSON.
