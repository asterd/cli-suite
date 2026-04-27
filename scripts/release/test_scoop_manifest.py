import hashlib
import importlib.util
import json
import tempfile
import unittest
from pathlib import Path


SCRIPT_PATH = Path(__file__).with_name("scoop-manifest.py")
SPEC = importlib.util.spec_from_file_location("scoop_manifest", SCRIPT_PATH)
scoop_manifest = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(scoop_manifest)


class ScoopManifestTests(unittest.TestCase):
    def test_builds_manifest_from_dist_metadata_and_hash_file(self) -> None:
        manifest = {
            "releases": [
                {
                    "app_name": "axt-peek",
                    "app_version": "0.1.0-rc1",
                    "artifacts": ["axt-peek-x86_64-pc-windows-msvc.zip"],
                    "hosting": {
                        "github": {
                            "artifact_base_url": "https://github.com",
                            "artifact_download_path": "/ddurzo/axt/releases/download/v0.1.0-rc1",
                        }
                    },
                }
            ]
        }
        hash_value = hashlib.sha256(b"archive").hexdigest()

        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            dist_manifest = root / "dist-manifest.json"
            sha256_file = root / "axt-peek-x86_64-pc-windows-msvc.zip.sha256"
            output = root / "bucket" / "axt-peek.json"
            dist_manifest.write_text(json.dumps(manifest), encoding="utf-8")
            sha256_file.write_text(f"{hash_value}  axt-peek-x86_64-pc-windows-msvc.zip\n", encoding="utf-8")

            exit_code = scoop_manifest.main_with_args_for_test(
                dist_manifest=dist_manifest,
                sha256_file=sha256_file,
                output=output,
            )

            self.assertEqual(exit_code, 0)
            generated = json.loads(output.read_text(encoding="utf-8"))
            self.assertEqual(generated["version"], "0.1.0-rc1")
            self.assertEqual(generated["architecture"]["64bit"]["hash"], hash_value)
            self.assertEqual(
                generated["architecture"]["64bit"]["url"],
                "https://github.com/ddurzo/axt/releases/download/v0.1.0-rc1/axt-peek-x86_64-pc-windows-msvc.zip",
            )


if __name__ == "__main__":
    unittest.main()
