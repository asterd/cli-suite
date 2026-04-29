#!/usr/bin/env python3
"""Install one or more axt commands from a local checkout with Cargo."""

from __future__ import annotations

import argparse
import subprocess
from pathlib import Path


COMMANDS = {
    "peek": "crates/axt-peek",
    "run": "crates/axt-run",
    "doc": "crates/axt-doc",
    "drift": "crates/axt-drift",
    "port": "crates/axt-port",
    "test": "crates/axt-test",
    "outline": "crates/axt-outline",
}


def repo_root() -> Path:
    return Path(__file__).resolve().parents[1]


def selected_commands(value: str) -> list[str]:
    if value == "all":
        return list(COMMANDS)
    return [value]


def install_command(command: str, aliases: bool, locked: bool, dry_run: bool) -> None:
    crate = repo_root() / COMMANDS[command]
    cargo = ["cargo", "install", "--path", str(crate)]
    if locked:
        cargo.append("--locked")
    if aliases:
        cargo.extend(["--features", "aliases"])
    print(" ".join(cargo))
    if not dry_run:
        subprocess.run(cargo, check=True)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--command", choices=("all", *COMMANDS), default="all")
    parser.add_argument("--aliases", action="store_true")
    parser.add_argument("--no-locked", action="store_true")
    parser.add_argument("--dry-run", action="store_true")
    args = parser.parse_args()

    for command in selected_commands(args.command):
        install_command(command, args.aliases, not args.no_locked, args.dry_run)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
