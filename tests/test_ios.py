"""原生发行版本、数学语料和生产依赖必须保持一致。"""
import json
import plistlib
import subprocess
import tomllib
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

class IOSTests(unittest.TestCase):
    def test_mobile_versions_packages_and_original_corpus_are_pinned(self):
        version = tomllib.loads((ROOT / "Cargo.toml").read_text())["workspace"]["package"]["version"]
        info = plistlib.loads((ROOT / "ios/OpenMath/Info.plist").read_bytes())
        self.assertEqual(info["OpenMathReleaseVersion"], version)
        self.assertEqual(info["CFBundleVersion"], "2")
        pins = json.loads((ROOT / "ios/OpenMath.xcodeproj/project.xcworkspace/xcshareddata/swiftpm/Package.resolved").read_text())["pins"]
        self.assertEqual({p["identity"]: p["state"]["version"] for p in pins}, {"swiftmath": "1.7.3", "swift-markdown": "0.9.0", "swift-cmark": "0.9.0"})
        original = tomllib.loads((ROOT / "tests/corpus/solve.toml").read_text())["case"]
        self.assertEqual(json.loads((ROOT / "ios/OpenMath/Resources/solve-corpus.json").read_text()), original)
        self.assertEqual(len(original), 53)

    def test_ios_production_graph_excludes_desktop_io_and_http_runtime(self):
        result = subprocess.run(["cargo", "tree", "-p", "om-ios-ffi", "--edges", "normal", "--target", "aarch64-apple-ios", "--locked", "--prefix", "none"], cwd=ROOT, check=True, capture_output=True, text=True)
        names = {line.split()[0] for line in result.stdout.splitlines()}
        self.assertTrue({"om-kernel", "om-solve", "om-eval"}.issubset(names))
        self.assertFalse({"keyring", "directories", "reqwest", "tokio"} & names)
