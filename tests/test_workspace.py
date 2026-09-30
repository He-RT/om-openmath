"""Repository boundary checks, using Cargo's resolved package graph."""

import json
import subprocess
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


class WorkspaceTests(unittest.TestCase):
    def test_cargo_resolves_workspace_with_shared_package_policy(self):
        result = subprocess.run(
            ["cargo", "metadata", "--format-version", "1", "--no-deps"],
            cwd=ROOT,
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        metadata = json.loads(result.stdout)
        members = set(metadata["workspace_members"])
        packages = [p for p in metadata["packages"] if p["id"] in members]
        self.assertGreater(len(packages), 0)
        for package in packages:
            with self.subTest(package=package["name"]):
                self.assertEqual(package["license"], "MIT OR Apache-2.0")
                self.assertEqual(package["edition"], "2024")
                self.assertEqual(package["rust_version"], "1.94")
                self.assertTrue(package["name"].startswith("om-"))


if __name__ == "__main__":
    unittest.main()
