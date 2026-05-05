#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"

cargo build --workspace --all-features

bin="target/debug"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

mkdir -p "$tmp/project/src"
cat >"$tmp/project/Cargo.toml" <<'EOF'
[package]
name = "axt-smoke"
version = "0.0.0"
edition = "2021"
EOF
cat >"$tmp/project/src/lib.rs" <<'EOF'
pub fn answer() -> u8 { 42 }

#[test]
fn answer_is_stable() {
    assert_eq!(answer(), 42);
}
EOF
printf 'ignored.txt\n' >"$tmp/project/.gitignore"
printf 'ignored\n' >"$tmp/project/ignored.txt"

"$bin/axt-peek" --json "$tmp/project" >/dev/null
"$bin/axt-run" --json --no-save --timeout 5s -- sh -c 'printf smoke' >/dev/null
(
  cd "$tmp/project"
  "$repo_root/$bin/axt-test" --json --framework cargo >/dev/null
)

(
  cd "$tmp/project"
  "$repo_root/$bin/axt-drift" mark --name smoke --hash >/dev/null
  printf 'changed\n' >src/changed.txt
  "$repo_root/$bin/axt-drift" --json diff --since smoke --hash >/dev/null
)

python3 -c 'import socket, sys, time
try:
    s = socket.socket()
    s.bind(("127.0.0.1", 0))
    s.listen()
except OSError:
    print("SKIP", flush=True)
    sys.exit(0)
print(s.getsockname()[1], flush=True)
time.sleep(20)' >"$tmp/port" &
listener_pid="$!"
for _ in $(seq 1 50); do
  test -s "$tmp/port" && break
  sleep 0.1
done
port="$(cat "$tmp/port")"
if [ "$port" != "SKIP" ]; then
  "$bin/axt-port" --json free --dry-run "$port" >/dev/null
fi
kill "$listener_pid" 2>/dev/null || true

"$bin/axt-outline" --json "$tmp/project/src/lib.rs" >/dev/null
"$bin/axt-slice" --json "$tmp/project/src/lib.rs" --symbol answer >/dev/null
"$bin/axt-ctxpack" --json --pattern smoke=answer "$tmp/project/src/lib.rs" >/dev/null
"$bin/axt-bundle" --json "$tmp/project" >/dev/null
"$bin/axt-logdx" --stdin --json --severity error <<<"2026-04-28T10:00:00Z error smoke failure" >/dev/null
