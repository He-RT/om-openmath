#!/usr/bin/env python3
"""Build the real offline Rust codec adapter and run the pinned Pi loop tests."""
import argparse
import os
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--node', default='node')
parser.add_argument('--evidence', type=Path)
args = parser.parse_args()
env = dict(os.environ, CARGO_PROFILE_DEV_DEBUG='0', CARGO_PROFILE_TEST_DEBUG='0')
subprocess.run(['cargo', 'build', '-p', 'om-llm', '--example', 'single_turn_fixture', '--locked'], cwd=ROOT, env=env, check=True)
target = Path(env.get('CARGO_TARGET_DIR', ROOT / 'target'))
if not target.is_absolute():
    target = ROOT / target
env['OPENMATH_SINGLE_TURN_FIXTURE'] = str(target / 'debug/examples/single_turn_fixture')
if args.evidence:
    env['OPENMATH_SINGLE_TURN_EVIDENCE'] = str(args.evidence.resolve())
subprocess.run([args.node, '--test', 'agent/test/single-turn-codec.test.mjs'], cwd=ROOT, env=env, check=True)
