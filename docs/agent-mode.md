# Agent Mode

Agent mode is minified JSONL. The first line is always a summary record with a
`schema`, `type: "summary"`, `ok`, `truncated`, and `next` field. Detail records
follow only when useful for the command result.

Selection:

- TTY stdout defaults to human output.
- Non-TTY stdout defaults to compact text output.
- `--agent`, `--json`, compact auto-selection, and human auto-selection are the output behaviors.
- `AXT_OUTPUT=human|compact|agent|json` overrides the automatic default.
- `--json` emits the canonical envelope `{schema, ok, data, warnings, errors}`.

`--plain`, `--json-data`, and `--jsonl` are retired. Use human output, `jq .data`,
or `--agent` respectively.

Compact text is distinct from agent mode. It is the implicit non-TTY format:
short plain-text records with a dense `key=value` summary first. `--agent` is
only the explicit schema-versioned JSONL mode.

## Shared Fields

Summary records use readable keys because they are low-cardinality control
records. High-cardinality detail records should prefer short keys:

| Key | Meaning |
| --- | --- |
| `p` | path |
| `k` | kind |
| `b` | bytes |
| `l` | language |
| `g` | git status |
| `ms` | duration in milliseconds |
| `ts` | timestamp |

Command-specific records are versioned as `axt.<cmd>.<record>.v1`. Warnings use
`axt.<cmd>.warn.v1` and include `type: "warn"` plus a stable `code`.

## Example

```jsonl
{"schema":"axt.peek.summary.v1","type":"summary","ok":true,"root":".","files":12,"dirs":3,"truncated":false,"next":["axt-outline src --agent"]}
{"schema":"axt.peek.entry.v1","type":"file","p":"src/lib.rs","b":4210,"l":"rust","g":"modified"}
```

Schemas are useful as exact references for validators and tool authors. For an
LLM, a schema file that is merely present on disk but not included in context has
little direct value; the model only benefits once the schema is printed,
referenced in docs, or used by tests/tools that enforce it.
