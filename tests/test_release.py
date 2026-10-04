"""发布入口必须拒绝错版标签与不完整/不透明的产物集合。"""
import importlib.util
import hashlib
import json
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


class ReleaseTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        spec = importlib.util.spec_from_file_location("release_tools", ROOT / "scripts/release.py")
        cls.release = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(cls.release)

    def test_checked_in_versions_and_windows_installer_intent_agree(self):
        version = self.release.verify_version(ROOT, "v0.1.0-pre-alpha.2")
        self.assertEqual(version, "0.1.0-pre-alpha.2")
        config = json.loads((ROOT / "app/src-tauri/tauri.windows.conf.json").read_text())
        self.assertEqual(set(config["bundle"]["targets"]), {"nsis", "msi"})
        windows = config["bundle"]["windows"]
        self.assertEqual(windows["wix"]["language"], "zh-CN")
        self.assertEqual(windows["wix"]["version"], "0.1.0.2")
        self.assertIn("SimpChinese", windows["nsis"]["languages"])
        self.assertEqual(windows["webviewInstallMode"]["type"], "embedBootstrapper")

    def test_wrong_version_tags_and_partial_asset_sets_cannot_publish(self):
        with self.assertRaises(ValueError):
            self.release.verify_version(ROOT, "v0.2.0")
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            (root / "OpenMath_0.1.0-pre-alpha.2_x64-setup.exe").write_bytes(b"fixture")
            with self.assertRaises(ValueError):
                self.release.finalize(root, "v0.1.0-pre-alpha.2", "a" * 40)

    def test_release_manifest_covers_exact_assets_with_byte_hashes(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            names = [
                "OpenMath_0.1.0-pre-alpha.2_x64-setup.exe",
                "OpenMath_0.1.0-pre-alpha.2_x64_zh-CN.msi",
                "OpenMath_0.1.0-pre-alpha.2_aarch64.dmg",
                "OpenMath_0.1.0-pre-alpha.2_macos_arm64.app.zip",
                "om-cli_0.1.0-pre-alpha.2_windows_x64.zip",
                "om-cli_0.1.0-pre-alpha.2_macos_arm64.zip",
                "OpenMath-web_0.1.0-pre-alpha.2.zip",
                "OpenMath_0.1.0-pre-alpha.2_ios_simulator_arm64.app.zip",
                "OpenMathKernel_0.1.0-pre-alpha.2.xcframework.zip",
            ]
            for name in names:
                (root / name).write_bytes(b"release-fixture")
            self.release.finalize(root, "v0.1.0-pre-alpha.2", "a" * 40)
            manifest = json.loads((root / "release-manifest.json").read_text())
            self.assertEqual({a["name"] for a in manifest["assets"]}, set(names))
            self.assertTrue(all(a["bytes"] == 15 for a in manifest["assets"]))
            expected_hash = hashlib.sha256(b"release-fixture").hexdigest()
            self.assertTrue(all(a["sha256"] == expected_hash for a in manifest["assets"]))
            self.assertEqual(manifest["revision"], "a" * 40)
            with self.assertRaises(ValueError):
                self.release.finalize(root, "v0.1.0-pre-alpha.2", "short-sha")
            (root / names[0]).write_bytes(b"")
            with self.assertRaises(ValueError):
                self.release.finalize(root, "v0.1.0-pre-alpha.2", "a" * 40)
            (root / names[0]).write_bytes(b"release-fixture")
            (root / "config.toml").write_text("should not ship")
            with self.assertRaises(ValueError):
                self.release.finalize(root, "v0.1.0-pre-alpha.2", "a" * 40)

    def test_installed_payload_allows_only_the_exact_tauri_bundle_stamp(self):
        marker = b"__TAURI_BUNDLE_TYPE_VAR_UNK"
        original = b"code-before\0" + marker + b"\0code-after"
        for kind, stamp in (("nsis", b"NSS"), ("msi", b"MSI")):
            installed = original.replace(marker, marker[:-3] + stamp)
            self.release.verify_payload_bytes(original, installed, kind)
            with self.assertRaises(ValueError):
                self.release.verify_payload_bytes(original, installed[:-1] + b"X", kind)
            with self.assertRaises(ValueError):
                self.release.verify_payload_bytes(original, original, kind)
        with self.assertRaises(ValueError):
            self.release.verify_payload_bytes(original + marker, original + marker, "nsis")


if __name__ == "__main__":
    unittest.main()
