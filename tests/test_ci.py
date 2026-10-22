"""Integration gates must report docs and reject failures, cancellation and lost jobs."""
import importlib.util
import unittest
from pathlib import Path

spec = importlib.util.spec_from_file_location('ci', Path(__file__).resolve().parents[1] / 'scripts/ci.py')
ci = importlib.util.module_from_spec(spec)
spec.loader.exec_module(ci)


class CITests(unittest.TestCase):
    def needs(self, paths):
        flags = ci.classify(paths)
        return {'changes': {'result': 'success', 'outputs': {k: str(v).lower() for k, v in flags.items()}},
                'lightweight': {'result': 'success'},
                **{k: {'result': 'success' if v else 'skipped'} for k, v in flags.items()}}

    def test_docs_have_lightweight_gate_and_no_product_builds(self):
        self.assertFalse(any(ci.classify(['docs/git-management.md', 'README.md']).values()))
        ci.verify_results(self.needs(['docs/git-management.md']))

    def test_production_docs_and_unknown_paths_cannot_evade_checks(self):
        for path in ('docs/design/notebook-agent.md', 'docs/reference/functions.toml', 'new-build-input', '.github/workflows/ci.yml'):
            self.assertTrue(all(v for k, v in ci.classify([path]).items() if k != 'ios'))

    def test_changes_on_either_side_of_rename_and_shared_core_require_checks(self):
        flags = ci.classify(['docs/old.md', 'crates/om-core/src/new.rs'])
        self.assertTrue(flags['rust'] and flags['frontend'] and flags['native_macos'] and flags['dependencies'])
        self.assertFalse(flags['ios'])
        self.assertTrue(ci.classify(['scripts/release.py'])['contracts'])
        self.assertTrue(ci.classify(['ios/OpenMath/App.swift'])['contracts'])

    def test_failure_cancel_missing_or_unexpected_skip_blocks_gate(self):
        for state in ('failure', 'cancelled', 'skipped', None):
            needs = self.needs(['crates/om-core/src/lib.rs'])
            needs['rust']['result'] = state
            with self.assertRaises(ValueError):
                ci.verify_results(needs)
        for name in ('changes', 'lightweight'):
            needs = self.needs(['README.md'])
            needs[name]['result'] = 'failure'
            with self.assertRaises(ValueError):
                ci.verify_results(needs)
        needs = self.needs(['README.md'])
        del needs['changes']['outputs']['rust']
        with self.assertRaises(ValueError):
            ci.verify_results(needs)

    def test_manual_ios_is_explicit(self):
        self.assertFalse(ci.classify([], manual=True)['ios'])
        self.assertTrue(all(ci.classify([], manual=True, ios=True).values()))


if __name__ == '__main__':
    unittest.main()
