#!/usr/bin/env python3
"""Install local axt agent skills into Codex and Claude Code directories."""

from __future__ import annotations

import argparse
import shutil
from pathlib import Path


SKILLS = (
    "axt-suite",
    "axt-peek",
    "axt-run",
    "axt-doc",
    "axt-drift",
    "axt-port",
    "axt-test",
    "axt-outline",
    "axt-slice",
    "axt-ctxpack",
    "axt-bundle",
    "axt-gitctx",
)


def repo_root() -> Path:
    return Path(__file__).resolve().parents[2]


def source_dir(skill: str) -> Path:
    path = repo_root() / "docs" / "skills" / skill
    if not path.is_dir():
        raise FileNotFoundError(f"skill not found: {path}")
    return path


def target_base(agent: str, scope: str, project: Path) -> Path:
    if agent == "codex":
        return (project / ".codex" / "skills") if scope == "project" else Path.home() / ".codex" / "skills"
    if agent == "claude":
        return (project / ".claude" / "skills") if scope == "project" else Path.home() / ".claude" / "skills"
    raise ValueError(f"unsupported agent: {agent}")


def selected_skills(value: str) -> tuple[str, ...]:
    if value == "all":
        return SKILLS
    if value not in SKILLS:
        raise ValueError(f"unsupported skill: {value}")
    return (value,)


def selected_agents(value: str) -> tuple[str, ...]:
    if value == "both":
        return ("codex", "claude")
    return (value,)


def install_skill(source: Path, target: Path, force: bool, dry_run: bool) -> None:
    if dry_run:
        print(f"would install {source} -> {target}")
        return
    if target.exists():
        if not force:
            raise FileExistsError(f"{target} already exists; pass --force to replace it")
        shutil.rmtree(target)
    target.parent.mkdir(parents=True, exist_ok=True)
    shutil.copytree(source, target)
    print(f"installed {target}")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--agent", choices=("codex", "claude", "both"), default="both")
    parser.add_argument("--scope", choices=("project", "user"), default="project")
    parser.add_argument("--skill", choices=("all", *SKILLS), default="axt-suite")
    parser.add_argument("--project", type=Path, default=Path.cwd())
    parser.add_argument("--force", action="store_true")
    parser.add_argument("--dry-run", action="store_true")
    args = parser.parse_args()

    project = args.project.resolve()
    for agent in selected_agents(args.agent):
        base = target_base(agent, args.scope, project)
        for skill in selected_skills(args.skill):
            install_skill(source_dir(skill), base / skill, args.force, args.dry_run)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
