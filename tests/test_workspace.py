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
            encoding="utf-8",
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

    def test_kernel_packages_form_expression_independent_polynomial_graph(self):
        result = subprocess.run(
            ["cargo", "metadata", "--format-version", "1", "--no-deps"],
            cwd=ROOT, capture_output=True, text=True, encoding="utf-8", check=True,
        )
        packages = {p["name"]: p for p in json.loads(result.stdout)["packages"]}
        expected = {
            "om-num": set(),
            "om-analysis": {"om-num"},
            "om-core": {"om-num"},
            "om-parse": {"om-core"},
            "om-format": {"om-core"},
            "om-poly": {"om-num"},
            "om-simplify": {"om-core", "om-poly"},
            "om-solve": {"om-simplify"},
            "om-eval": {"om-solve"},
            "om-llm": {"om-num"},
            "om-kernel": {"om-eval", "om-parse", "om-format", "om-llm"},
            "om-cli": {"om-kernel"},
            "om-wasm": {"om-kernel"},
        }
        self.assertTrue(set(expected).issubset(packages), set(expected) - packages.keys())
        graph = {}
        for name, package in packages.items():
            graph[name] = {
                d["name"] for d in package["dependencies"]
                if d["kind"] != "dev" and d["name"].startswith("om-")
            }
        for name, dependencies in expected.items():
            self.assertTrue(dependencies.issubset(graph[name]), (name, graph[name]))
        pending, reachable = ["om-poly"], set()
        while pending:
            node = pending.pop()
            if node not in reachable:
                reachable.add(node)
                pending.extend(graph[node])
        self.assertNotIn("om-core", reachable)
        self.assertEqual(graph["om-analysis"], {"om-num"})
        self.assertIn("native", packages["om-kernel"]["features"])
        self.assertIn("http", packages["om-llm"]["features"])

    def test_desktop_is_a_workspace_binary_with_native_kernel(self):
        result = subprocess.run(
            ["cargo", "metadata", "--format-version", "1", "--no-deps"],
            cwd=ROOT, capture_output=True, text=True, encoding="utf-8", check=True,
        )
        packages = {p["name"]: p for p in json.loads(result.stdout)["packages"]}
        self.assertIn("om-desktop", packages)
        desktop = packages["om-desktop"]
        self.assertTrue(any("bin" in t["kind"] for t in desktop["targets"]))
        kernel = next(d for d in desktop["dependencies"] if d["name"] == "om-kernel")
        self.assertIn("native", kernel["features"])


if __name__ == "__main__":
    unittest.main()
