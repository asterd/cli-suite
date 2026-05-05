# Error Catalog

The standard error catalog is exported by `axt-core` as
`STANDARD_ERROR_CATALOG`. Every command also exposes the same catalog with:

```bash
axt-peek --list-errors
axt-run --list-errors
```

`--list-errors` prints one JSONL record per code. JSON envelopes place fatal
diagnostics in `errors`; non-fatal diagnostics go in `warnings`. Agent JSONL
warning records use the command-specific `axt.<cmd>.warn.v1` schema and the
same stable `code` values.

| Code | Exit | Meaning | Retryable |
|---|---:|---|---|
| `ok` | 0 | Success | n/a |
| `runtime_error` | 1 | Generic runtime failure | maybe |
| `usage_error` | 2 | CLI argument or flag invalid | no |
| `path_not_found` | 3 | A required path does not exist | no |
| `permission_denied` | 4 | Insufficient permissions | no |
| `timeout` | 5 | Operation exceeded `--timeout` | yes |
| `output_truncated_strict` | 6 | `--strict` and output had to be truncated | no |
| `interrupted` | 7 | SIGINT / Ctrl-C received | no |
| `io_error` | 8 | Filesystem or stream IO failure | maybe |
| `feature_unsupported` | 9 | Feature unavailable on this platform | no |
| `schema_violation` | 10 | Internal schema validation failure | no |
| `command_failed` | 11 | Wrapped command exited non-zero | depends |
| `git_unavailable` | 12 | Git repo expected but not found or readable | no |
| `config_error` | 13 | User config file malformed | no |
| `network_disabled` | 14 | Offline command attempted network | no |

Example JSONL:

```jsonl
{"code":"usage_error","exit":2,"meaning":"CLI argument or flag invalid","retryable":"no"}
{"code":"path_not_found","exit":3,"meaning":"A required path does not exist","retryable":"no"}
```

The source of truth for the contract is `docs/spec.md`; the implementation
constant in `axt-core` must stay byte-for-byte compatible with these public
names and exit codes.
