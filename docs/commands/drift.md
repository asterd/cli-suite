# axt-drift

`axt-drift` marks filesystem state and later reports what changed.

## CLI

```bash
axt-drift [FLAGS] mark [--name <NAME>] [--hash]
axt-drift [FLAGS] diff [--since <NAME>] [--hash]
axt-drift [FLAGS] run [--name <NAME>] [--hash] -- <CMD> [ARGS]...
axt-drift [FLAGS] list
axt-drift [FLAGS] reset
```

Shared flags are available before the subcommand: `--json`, `--agent`, `--print-schema`, `--list-errors`, `--limit`, `--max-bytes`, and `--strict`.

When no name is provided, `axt-drift` uses `default`. Snapshots are stored as JSONL under `.axt/drift/<NAME>.jsonl`. Snapshot records contain relative path, size, mtime, and an optional BLAKE3 hash when `--hash` is passed.

`axt-drift run` captures a snapshot before running the command, runs the command in the current directory, then reports the created, modified, and deleted files.

## Output

JSON mode emits the `axt.drift.v1` envelope:

```json
{
  "schema": "axt.drift.v1",
  "ok": true,
  "data": {
    "operation": "diff",
    "name": "default",
    "changes": []
  },
  "warnings": [],
  "errors": []
}
```

Agent mode emits summary-first JSONL records:

```jsonl
{"schema":"axt.drift.summary.v1","type":"summary","operation":"diff","name":"default","files":12,"changed":1,"marks":0,"removed":0,"truncated":false,"next":[]}
{"schema":"axt.drift.file.v1","type":"file","path":"dist/app.js","action":"created","size_delta":1204}
```

## Cross-Platform Matrix

| Feature | Linux | macOS | Windows | Notes |
| --- | --- | --- | --- | --- |
| Metadata snapshot and diff | Yes | Yes | Yes | Uses file size and modified time. |
| Hash snapshot and diff | Yes | Yes | Yes | Opt-in with `--hash`; slower than metadata mode. |
| Snapshot storage | Yes | Yes | Yes | Stored below `.axt/drift`; `.axt` is excluded from captured snapshots. |
