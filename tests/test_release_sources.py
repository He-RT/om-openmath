"""Reject missing sources, unknown commits and altered assets or source sidecars."""
import copy
import hashlib
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / 'scripts'))
import release
import release_sources as sources


class SourceTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.revision = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT).decode().strip()
        self.tag = 'v' + release.VERSION
        for name in release.asset_names(release.VERSION, 'cli'):
            (self.root / name).write_bytes(b'original-artifact')
        release.finalize(self.root, self.tag, self.revision, 'cli')
        self.manifest = self.root / 'cli-release-manifest.json'
        self.records = []
        for platform in ('macos_arm64', 'windows_x64'):
            output = self.root / (platform + '-sources.json')
            with patch.object(sources, 'tools', return_value={'python': sys.version, 'rustc': 'test fixture'}):
                sources.record(ROOT, ROOT, 'cli', platform, self.tag, self.revision, self.revision,
                               'https://github.com/He-RT/om-openmath/actions/runs/1',
                               {'target': platform}, 'fixture version and roots', self.root,
                               list(sources.platform_assets('cli', platform, release.VERSION)), output)
            self.records.append(output)

    def join(self):
        return sources.index(ROOT, self.root, [self.manifest], self.records, self.root / 'index.json')

    def test_roundtrip_preserves_manifests_and_all_source_relations(self):
        before = self.manifest.read_bytes()
        self.join()
        sources.verify_index(self.root / 'index.json', ROOT, self.root)
        self.assertEqual(before, self.manifest.read_bytes())
        with self.assertRaises(FileExistsError):
            self.join()

    def test_missing_sources_and_forged_commit_are_rejected(self):
        for key, value in [('product_sha', 'abc'), ('packaging_sha', 'f' * 40),
                           ('dependency_sha256', {}), ('run_url', ''), ('parameters', {}),
                           ('workflow', {}), ('toolchains', {})]:
            record = json.loads(self.records[0].read_text(encoding='utf-8'))
            record[key] = value
            with self.subTest(key=key), self.assertRaises((ValueError, subprocess.CalledProcessError)):
                sources.validate_record(record, ROOT)
        with self.assertRaises(ValueError):
            sources.index(ROOT, self.root, [self.manifest], self.records[:1], self.root / 'bad.json')

    def test_wrong_asset_hash_or_changed_sidecar_is_rejected(self):
        asset = next(iter(release.asset_names(release.VERSION, 'cli')))
        original = (self.root / asset).read_bytes()
        (self.root / asset).write_bytes(b'wrong artifact')
        with self.assertRaises(ValueError):
            self.join()
        (self.root / asset).write_bytes(original)
        self.join()
        self.records[0].write_text('{}', encoding='utf-8')
        with self.assertRaises(ValueError):
            sources.verify_index(self.root / 'index.json', ROOT, self.root)

    def test_wrong_platform_dependencies_or_workflow_hash_is_rejected(self):
        record = json.loads(self.records[0].read_text(encoding='utf-8'))
        for mutate in (lambda r: r.update(platform='windows_x64'),
                       lambda r: r['dependency_sha256'].update({'Cargo.lock': '0' * 64}),
                       lambda r: r['workflow'].update(sha256='0' * 64)):
            changed = copy.deepcopy(record)
            mutate(changed)
            with self.assertRaises(ValueError):
                sources.validate_record(changed, ROOT)


if __name__ == '__main__':
    unittest.main()
