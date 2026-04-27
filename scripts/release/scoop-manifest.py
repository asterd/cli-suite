#!/usr/bin/env python3
"""Generate the Scoop manifest for axt-peek from dist release metadata."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


APP_NAME = "axt-peek"
WINDOWS_ARCHIVE = "axt-peek-x86_64-pc-windows-msvc.zip"


def load_json(path: Path) -> dict[str, Any]:
    with path.open(encoding="utf-8") as handle:
        data = json.load(handle)
    if not isinstance(data, dict):
        raise ValueError("dist manifest must be a JSON object")
    return data


def read_sha256(path: Path) -> str:
    text = path.read_text(encoding="utf-8").strip()
    if not text:
        raise ValueError(f"{path} is empty")
    hash_value = text.split()[0]
    if len(hash_value) != 64 or any(char not in "0123456789abcdefABCDEF" for char in hash_value):
        raise ValueError(f"{path} does not start with a SHA-256 hash")
    return hash_value.lower()


def axt_peek_release(manifest: dict[str, Any]) -> dict[str, Any]:
    for release in manifest.get("releases", []):
        if release.get("app_name") == APP_NAME:
            return release
    raise ValueError(f"dist manifest does not contain an {APP_NAME} release")


def github_release_url(release: dict[str, Any], artifact: str) -> str:
    github = release.get("hosting", {}).get("github", {})
    base_url = github.get("artifact_base_url")
    download_path = github.get("artifact_download_path")
    if not isinstance(base_url, str) or not isinstance(download_path, str):
        raise ValueError("dist manifest release is missing GitHub hosting metadata")
    return f"{base_url}{download_path}/{artifact}"


def build_manifest(dist_manifest: dict[str, Any], sha256: str) -> dict[str, Any]:
    release = axt_peek_release(dist_manifest)
    artifacts = release.get("artifacts", [])
    if WINDOWS_ARCHIVE not in artifacts:
        raise ValueError(f"dist manifest release is missing {WINDOWS_ARCHIVE}")

    version = release.get("app_version")
    if not isinstance(version, str):
        raise ValueError("dist manifest release is missing app_version")

    url = github_release_url(release, WINDOWS_ARCHIVE)
    return {
        "version": version,
        "description": "Directory and repository snapshot command for the axt Foundation CLI Suite.",
        "homepage": "https://github.com/ddurzo/axt",
        "license": "MIT OR Apache-2.0",
        "architecture": {
            "64bit": {
                "url": url,
                "hash": sha256,
            }
        },
        "bin": "axt-peek.exe",
        "checkver": {
            "github": "https://github.com/ddurzo/axt",
        },
        "autoupdate": {
            "architecture": {
                "64bit": {
                    "url": "https://github.com/ddurzo/axt/releases/download/v$version/axt-peek-x86_64-pc-windows-msvc.zip",
                }
            }
        },
    }


def write_manifest(dist_manifest: Path, sha256_file: Path, output: Path) -> None:
    manifest = build_manifest(load_json(dist_manifest), read_sha256(sha256_file))
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def main_with_args_for_test(dist_manifest: Path, sha256_file: Path, output: Path) -> int:
    write_manifest(dist_manifest, sha256_file, output)
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--dist-manifest", type=Path, required=True)
    parser.add_argument("--sha256-file", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()

    write_manifest(args.dist_manifest, args.sha256_file, args.output)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
