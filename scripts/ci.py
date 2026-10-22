"""Select applicable checks from Git paths and fail closed on their conclusions."""
import argparse
import json
import os
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CHECKS = ('rust', 'frontend', 'native_macos', 'dependencies', 'contracts', 'ios')
TOOLING = {'scripts/ci.py', 'scripts/check_git_rules.py', 'scripts/release.py',
           'scripts/release_sources.py', 'tests/test_ci.py', 'tests/test_release.py',
           'tests/test_release_sources.py'}


def classify(paths: list[str], *, manual: bool = False, ios: bool = False) -> dict[str, bool]:
    selected = {name: manual for name in CHECKS}
    selected['ios'] = ios
    for path in paths:
        if path in TOOLING or path.startswith('.github/workflows/release'):
            selected['contracts'] = True
        elif path in ('.gitignore', '.gitattributes', 'CODEX_HANDOFF.md'):
            pass
        elif path.startswith('docs/design/prototypes/ios27-glass/') or path in (
            'docs/design/ios-ipados-workbench.md', 'docs/design/ios-ipados-workbench-02.md'
        ):
            # Standalone material study; no production client consumes these files.
            # Compile the SwiftUI preview locally with its documented macOS SDK.
            selected['contracts'] = True
        elif path.startswith(('docs/design/', 'docs/reference/', 'docs/examples/', 'docs/acceptance/pre-alpha.4/fixtures/')) or path == 'docs/plan/PLAN.md':
            # These contain production prompts, fixtures and generated function contracts.
            for name in CHECKS[:-1]:
                selected[name] = True
        elif path.startswith('docs/') or path in ('README.md', 'README.en.md', 'LICENSE', 'LICENSE-MIT', 'LICENSE-APACHE'):
            pass
        elif path.startswith(('crates/', 'agent/')) or path in ('Cargo.toml', 'Cargo.lock', 'rust-toolchain.toml', 'deny.toml'):
            for name in CHECKS[:-1]:
                selected[name] = True
        elif path.startswith('macos/'):
            selected['native_macos'] = selected['contracts'] = True
        elif path.startswith('ios/'):
            # Ordinary integration checks static iOS contracts, never starts a simulator.
            selected['contracts'] = True
        elif path.startswith('app/'):
            selected['frontend'] = selected['contracts'] = True
            if path.startswith('app/src-tauri/'):
                selected['rust'] = selected['dependencies'] = True
        elif path.startswith('tests/') and not path.endswith('.rs') and '/corpus/' not in path:
            selected['contracts'] = True
        else:
            # New/unclassified build inputs and changes to CI itself get all regular checks.
            for name in CHECKS[:-1]:
                selected[name] = True
    return selected


def verify_results(needs: dict) -> None:
    for name in ('changes', 'lightweight'):
        if needs.get(name, {}).get('result') != 'success':
            raise ValueError(f'{name} must succeed')
    outputs = needs['changes'].get('outputs', {})
    for name in CHECKS:
        flag = outputs.get(name)
        if flag not in ('true', 'false'):
            raise ValueError(f'missing/invalid applicability: {name}')
        expected = 'success' if flag == 'true' else 'skipped'
        actual = needs.get(name, {}).get('result')
        if actual != expected:
            raise ValueError(f'{name}: expected {expected}, got {actual}')


def changed_paths(event: dict, event_name: str) -> list[str]:
    if event_name == 'workflow_dispatch':
        return []
    if event_name == 'pull_request':
        base = event['pull_request']['base']['sha']
        head = event['pull_request']['head']['sha']
        revision = f'{base}...{head}'
    elif event_name == 'push':
        base, head = event['before'], event['after']
        if base == '0' * 40:
            return subprocess.check_output(['git', 'ls-tree', '-r', '--name-only', '-z', head], cwd=ROOT).decode().strip('\0').split('\0')
        revision = f'{base}..{head}'
    else:
        raise ValueError(f'unsupported event: {event_name}')
    data = subprocess.check_output(['git', 'diff', '--name-only', '--no-renames', '-z', revision], cwd=ROOT)
    return [p for p in data.decode().split('\0') if p]


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('command', choices=('classify', 'gate', 'lightweight'))
    args = parser.parse_args()
    if args.command == 'gate':
        verify_results(json.loads(os.environ['NEEDS_JSON']))
        print('All applicable checks succeeded; all skips are intentional')
        return
    event_name = os.environ['GITHUB_EVENT_NAME']
    event = json.loads(Path(os.environ['GITHUB_EVENT_PATH']).read_text(encoding='utf-8'))
    paths = changed_paths(event, event_name)
    if args.command == 'classify':
        flags = classify(paths, manual=event_name == 'workflow_dispatch', ios=event.get('inputs', {}).get('run_ios') in (True, 'true'))
        with open(os.environ['GITHUB_OUTPUT'], 'a', encoding='utf-8') as output:
            for name, flag in flags.items():
                output.write(f'{name}={str(flag).lower()}\n')
        print(json.dumps({'paths': paths, 'checks': flags}, indent=2))
    else:
        import check_git_rules
        check_git_rules.check()
        # Validate only the relevant diff, respecting byte-preserving attributes.
        if event_name == 'pull_request':
            revision = f"{event['pull_request']['base']['sha']}...{event['pull_request']['head']['sha']}"
        elif event_name == 'push' and event['before'] != '0' * 40:
            revision = f"{event['before']}..{event['after']}"
        else:
            revision = 'HEAD'
        subprocess.run(['git', 'diff', '--check', revision], cwd=ROOT, check=True)


if __name__ == '__main__':
    main()
