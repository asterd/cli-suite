---
name: axt-port
description: Use axt-port to inspect local TCP/UDP port holders, wait for ports, and carefully free local ports with dry-run and confirmation controls. Trigger when an agent needs safe local port diagnostics.
license: MIT OR Apache-2.0
---

# axt-port Skill

Use `axt-port` for local port inspection and controlled local process cleanup.

## Rules

- Prefer `who <PORT> --agent` to identify a holder.
- Use `list --json` for full local socket inventory.
- Use `watch <PORT>` when waiting for a port to become free.
- Treat `free` as mutating. Run `free <PORT> --dry-run --agent` before killing anything.
- Never use it for remote hosts; scope is local ports only.

## Examples

```bash
axt-port who 3000 --agent
axt-port list --proto both --json
axt-port watch 3000 --timeout 10s --agent
axt-port free 3000 --dry-run --agent
```

Inspect contracts with `axt-port --print-schema agent` or `axt-port --print-schema json`.
