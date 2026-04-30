#!/usr/bin/env python3
"""Generate synthetic logs and benchmark axt-logdx release throughput."""

from __future__ import annotations

import argparse
import os
import subprocess
import tempfile
import time
from pathlib import Path

try:
    import resource
except ImportError:  # pragma: no cover - Windows fallback.
    resource = None


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--lines", type=int, default=1_000_000)
    parser.add_argument("--cardinality", type=int, default=10_000)
    parser.add_argument(
        "--format",
        choices=("plain", "jsonl", "logfmt"),
        default="plain",
        dest="log_format",
    )
    parser.add_argument("--severity", default="error")
    parser.add_argument("--top", type=int, default=20)
    parser.add_argument("--keep", action="store_true", help="keep generated log file")
    parser.add_argument("--bin", default=os.environ.get("AXT_LOGDX_BIN"))
    return parser.parse_args()


def ensure_binary(path: str | None) -> Path:
    if path:
        candidate = Path(path)
        if candidate.exists():
            return candidate
    subprocess.run(
        ["cargo", "build", "--release", "-p", "axt-logdx", "--bin", "axt-logdx"],
        check=True,
    )
    return Path("target/release/axt-logdx")


def line_for(index: int, cardinality: int, log_format: str) -> str:
    code = index % max(cardinality, 1)
    second = index % 60
    timestamp = f"2026-04-28T10:00:{second:02d}Z"
    if log_format == "jsonl":
        return (
            f'{{"timestamp":"{timestamp}","level":"error",'
            f'"message":"synthetic failure code-{code}"}}\n'
        )
    if log_format == "logfmt":
        return f'ts={timestamp} level=error msg="synthetic failure code-{code}"\n'
    return f"{timestamp} ERROR synthetic failure code-{code}\n"


def generate_log(path: Path, lines: int, cardinality: int, log_format: str) -> int:
    bytes_written = 0
    with path.open("w", encoding="utf-8") as handle:
        for index in range(lines):
            line = line_for(index, cardinality, log_format)
            bytes_written += len(line.encode("utf-8"))
            handle.write(line)
    return bytes_written


def max_rss_mb() -> float:
    if resource is None:
        return 0.0
    usage = resource.getrusage(resource.RUSAGE_CHILDREN).ru_maxrss
    if hasattr(os, "uname") and os.uname().sysname == "Darwin":
        return usage / (1024 * 1024)
    return usage / 1024


def main() -> None:
    args = parse_args()
    binary = ensure_binary(args.bin)
    with tempfile.NamedTemporaryFile(prefix="axt-logdx-bench-", suffix=".log", delete=False) as tmp:
        path = Path(tmp.name)
    try:
        bytes_written = generate_log(path, args.lines, args.cardinality, args.log_format)
        command = [
            str(binary),
            str(path),
            "--agent",
            "--severity",
            args.severity,
            "--top",
            str(args.top),
        ]
        start = time.perf_counter()
        completed = subprocess.run(command, check=True, capture_output=True)
        elapsed = time.perf_counter() - start
        mib = bytes_written / (1024 * 1024)
        print(f"format={args.log_format}")
        print(f"lines={args.lines}")
        print(f"bytes={bytes_written}")
        print(f"elapsed_seconds={elapsed:.3f}")
        print(f"throughput_mib_s={mib / elapsed:.2f}")
        print(f"stdout_bytes={len(completed.stdout)}")
        print(f"stderr_bytes={len(completed.stderr)}")
        rss = max_rss_mb()
        print(f"max_child_rss_mb={rss:.1f}" if rss else "max_child_rss_mb=unavailable")
        print(f"log_path={path if args.keep else '<deleted>'}")
    finally:
        if not args.keep:
            path.unlink(missing_ok=True)


if __name__ == "__main__":
    main()
