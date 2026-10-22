"""Record build inputs and join existing manifests without rebuilding release assets."""
import argparse
import hashlib
import json
import os
import re
import subprocess
import sys
import tomllib
from pathlib import Path
from urllib.parse import quote

REPOSITORY = 'He-RT/om-openmath'
DEPENDENCIES = ('Cargo.lock', 'app/package-lock.json', 'agent/package-lock.json',
                'rust-toolchain.toml', 'app/.nvmrc',
                'ios/OpenMath.xcodeproj/project.xcworkspace/xcshareddata/swiftpm/Package.resolved',
                'macos/OpenMathNative.xcodeproj/project.xcworkspace/xcshareddata/swiftpm/Package.resolved')
WORKFLOWS = {'app-web': '.github/workflows/release.yml', 'cli': '.github/workflows/release-cli.yml',
             'tauri': '.github/workflows/release-shells.yml'}
PLATFORMS = {'app-web': {'web', 'macos_arm64'}, 'cli': {'macos_arm64', 'windows_x64'},
             'tauri': {'windows', 'android'}}
CHECK_OVERLAYS = {'app/scripts/check-installed-shell.mjs', 'app/scripts/package-smoke.js'}


def sha(value: str) -> str:
    if not isinstance(value, str) or not re.fullmatch(r'[0-9a-f]{40}', value):
        raise ValueError('Source must be a complete commit SHA')
    return value


def git(root: Path, *args: str) -> bytes:
    return subprocess.check_output(['git', '-C', str(root), *args], stderr=subprocess.PIPE)


def commit(root: Path, revision: str) -> None:
    sha(revision)
    if git(root, 'cat-file', '-t', revision).strip() != b'commit':
        raise ValueError('Source must name a retrievable commit')


def digest(path: Path) -> dict:
    if path.is_symlink() or not path.is_file() or path.stat().st_size == 0:
        raise ValueError(f'Expected nonempty regular file: {path.name}')
    with path.open('rb') as stream:
        return {'name': path.name, 'bytes': path.stat().st_size,
                'sha256': hashlib.file_digest(stream, 'sha256').hexdigest()}


def write_new(path: Path, value: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    # Supplement names must be new, never silently replace a frozen source record.
    with path.open('x', encoding='utf-8') as stream:
        stream.write(json.dumps(value, ensure_ascii=False, indent=2) + '\n')


def tools(root: Path) -> dict:
    result = {'python': sys.version, 'runner': {k: os.environ.get(k) for k in
              ('RUNNER_OS', 'RUNNER_ARCH', 'ImageOS', 'ImageVersion')}}
    commands = {'rustc': ['rustc', '-Vv'], 'cargo': ['cargo', '--version'],
                'node': ['node', '--version'], 'npm': ['npm', '--version'],
                'xcode': ['xcodebuild', '-version'], 'java': ['java', '-version'],
                'swift': ['swift', '--version'], 'clang': ['clang', '--version'],
                'wasm-bindgen': ['wasm-bindgen', '--version']}
    for name, command in commands.items():
        try:
            # Windows npm uses a .cmd launcher; no shell interpolation of inputs.
            if os.name == 'nt' and name == 'npm':
                command = ['cmd', '/c', 'npm', '--version']
            run = subprocess.run(command, cwd=root, capture_output=True, encoding='utf-8', errors='replace', timeout=30)
            result[name] = {'exit_code': run.returncode, 'version': (run.stdout + run.stderr).strip()}
        except (OSError, subprocess.TimeoutExpired):
            result[name] = {'unavailable': True}
    return result



def platform_assets(scope: str, platform: str, version: str) -> set[str]:
    from release import asset_names
    names = asset_names(version, scope)
    if scope == 'app-web':
        return {name for name in names if ('-web_' in name) == (platform == 'web')}
    if scope == 'cli':
        return {name for name in names if platform in name}
    return {name for name in names if name.endswith('.apk' if platform == 'android' else '.msi')}


def overlay_inputs(source: Path, packaging: Path, revision: str, scope: str, platform: str) -> list[dict]:
    changed = git(source, 'diff', '--name-only', '--no-renames', '-z', 'HEAD').decode().split('\0')
    overlays = []
    for path in filter(None, changed):
        # The Windows installer test deliberately copies these two external checks.
        # Product code changes are never attributed to an otherwise clean commit.
        if scope != 'tauri' or platform != 'windows' or path not in CHECK_OVERLAYS:
            raise ValueError(f'Unrecorded modified source: {path}')
        data = git(packaging, 'show', f'{revision}:{path}')
        if (source / path).read_bytes() != data:
            raise ValueError(f'Overlay differs from packaging source: {path}')
        overlays.append({'path': path, 'sha256': hashlib.sha256(data).hexdigest()})
    return overlays


def record(source: Path, packaging: Path, scope: str, platform: str, tag: str,
           product_sha: str, packaging_sha: str, run_url: str, parameters: dict,
           validation: str, assets_root: Path, assets: list[str], output: Path) -> dict:
    if scope not in PLATFORMS or platform not in PLATFORMS[scope]:
        raise ValueError('Invalid scope/platform')
    for root, revision in ((source, product_sha), (packaging, packaging_sha)):
        commit(root, revision)
        if git(root, 'rev-parse', 'HEAD').decode().strip() != revision:
            raise ValueError('Build checkout differs from declared source')
    if not re.fullmatch(r'https://github.com/He-RT/om-openmath/actions/runs/[0-9]+(?:/attempts/[0-9]+)?', run_url):
        raise ValueError('A concrete build run URL is required')
    if not isinstance(parameters, dict) or not parameters or not validation.strip():
        raise ValueError('Build parameters and validation scope are required')
    from release import VERSION
    if tag != f'v{VERSION}' or not assets or len(set(assets)) != len(assets) or set(assets) != platform_assets(scope, platform, VERSION):
        raise ValueError('Wrong tag or unexpected/duplicate assets')
    product_version = tomllib.loads(git(source, 'show', f'{product_sha}:Cargo.toml').decode())['workspace']['package']['version']
    if tag != 'v' + product_version:
        raise ValueError('Product source version differs from release tag')
    overlays = overlay_inputs(source, packaging, packaging_sha, scope, platform)
    dependencies = {}
    for path in DEPENDENCIES:
        try:
            data = git(source, 'show', f'{product_sha}:{path}')
        except subprocess.CalledProcessError:
            continue
        # Record the bytes actually consumed, rejecting a modified lock/toolchain file.
        if (source / path).exists() and (source / path).read_bytes() != data:
            raise ValueError(f'Dirty build dependency: {path}')
        dependencies[path] = hashlib.sha256(data).hexdigest()
    if 'Cargo.lock' not in dependencies:
        raise ValueError('Missing product dependency lock')
    workflow = WORKFLOWS[scope]
    workflow_hash = hashlib.sha256(git(packaging, 'show', f'{packaging_sha}:{workflow}')).hexdigest()
    value = {'schema': 1, 'repository': REPOSITORY, 'tag': tag, 'scope': scope, 'platform': platform,
             'product_sha': product_sha, 'packaging_sha': packaging_sha, 'capture': 'build',
             'workflow': {'path': workflow, 'sha': packaging_sha, 'sha256': workflow_hash},
             'run_url': run_url, 'parameters': parameters, 'toolchains': tools(source),
             'overlays': overlays,
             'dependency_sha256': dependencies, 'validation': validation,
             'reproducibility': 'Pinned sources and locks; runner images and external tool downloads may change.',
             'assets': [digest(assets_root / name) for name in sorted(assets)]}
    write_new(output, value)
    return value


def validate_record(value: dict, repo: Path) -> None:
    if value.get('schema') != 1 or value.get('repository') != REPOSITORY:
        raise ValueError('Unsupported source record')
    scope, platform = value.get('scope'), value.get('platform')
    if scope not in PLATFORMS or platform not in PLATFORMS[scope]:
        raise ValueError('Missing source platform')
    from release import VERSION
    if value.get('tag') != f'v{VERSION}' or {a['name'] for a in value.get('assets', [])} != platform_assets(scope, platform, VERSION):
        raise ValueError('Wrong tag or source platform assets')
    if value.get('capture') not in ('build', 'historical-build-log'):
        raise ValueError('Source capture method is required')
    for name in ('product_sha', 'packaging_sha'):
        commit(repo, value.get(name))
    product_version = tomllib.loads(git(repo, 'show', f"{value['product_sha']}:Cargo.toml").decode())['workspace']['package']['version']
    if value['tag'] != 'v' + product_version:
        raise ValueError('Product source version differs from release tag')
    workflow = value.get('workflow', {})
    if workflow.get('sha') != value['packaging_sha'] or workflow.get('path') != WORKFLOWS[scope]:
        raise ValueError('Workflow source mismatch')
    data = git(repo, 'show', f"{workflow['sha']}:{workflow['path']}")
    if hashlib.sha256(data).hexdigest() != workflow.get('sha256'):
        raise ValueError('Workflow digest mismatch')
    seen = set()
    for overlay in value.get('overlays', []):
        path = overlay['path']
        if scope != 'tauri' or platform != 'windows' or path not in CHECK_OVERLAYS or path in seen:
            raise ValueError('Unexpected source overlay')
        seen.add(path)
        if hashlib.sha256(git(repo, 'show', f"{value['packaging_sha']}:{path}")).hexdigest() != overlay.get('sha256'):
            raise ValueError('Overlay digest mismatch')
    dependencies = value.get('dependency_sha256', {})
    if not dependencies or 'Cargo.lock' not in dependencies:
        raise ValueError('Missing dependency sources')
    available = set()
    for path in DEPENDENCIES:
        try:
            git(repo, 'show', f"{value['product_sha']}:{path}")
            available.add(path)
        except subprocess.CalledProcessError:
            continue
    if set(dependencies) != available:
        raise ValueError('Incomplete or unexpected dependency sources')
    for path, expected in dependencies.items():
        if path not in DEPENDENCIES or hashlib.sha256(git(repo, 'show', f"{value['product_sha']}:{path}")).hexdigest() != expected:
            raise ValueError('Dependency digest mismatch')
    if not re.fullmatch(r'https://github.com/He-RT/om-openmath/actions/runs/[0-9]+(?:/attempts/[0-9]+)?', value.get('run_url', '')):
        raise ValueError('Missing concrete run URL')
    if not value.get('parameters') or not value.get('toolchains') or not value.get('validation'):
        raise ValueError('Missing build parameters, toolchains or validation scope')


def index(repo: Path, root: Path, manifests: list[Path], records: list[Path], output: Path) -> dict:
    if not manifests or not records:
        raise ValueError('Manifests and source records are required')
    assets, tags = {}, set()
    manifest_entries = []
    for path in manifests:
        value = json.loads(path.read_text(encoding='utf-8'))
        commit(repo, value.get('revision'))
        if value.get('tag') != 'v' + value.get('version', ''):
            raise ValueError('Manifest version/tag mismatch')
        tags.add(value['tag'])
        if not value.get('assets'):
            raise ValueError('Empty release manifest')
        for asset in value['assets']:
            name = asset['name']
            if Path(name).name != name or name in assets or digest(root / name) != asset:
                raise ValueError(f'Duplicate or mismatched asset: {name}')
            assets[name] = {'asset': asset, 'product_sha': value['revision'],
                            'packaging_revisions': value.get('packaging_revisions', {})}
        manifest_entries.append(digest(path))
    if len(tags) != 1:
        raise ValueError('Mixed release tags')
    tag = tags.pop()
    coverage, source_entries = {}, []
    for path in records:
        value = json.loads(path.read_text(encoding='utf-8'))
        validate_record(value, repo)
        if value['tag'] != tag or not value.get('assets'):
            raise ValueError('Source record tag mismatch or missing assets')
        for asset in value['assets']:
            name = asset['name']
            expected = assets.get(name)
            if not expected or name in coverage or asset != expected['asset'] or value['product_sha'] != expected['product_sha']:
                raise ValueError('Asset has conflicting or incorrect source')
            declared = expected['packaging_revisions'].get(value['platform'])
            if declared and declared != value['packaging_sha']:
                raise ValueError('Packaging source conflicts with manifest')
            coverage[name] = path.name
        source_entries.append({'file': digest(path), 'record': value})
    if set(coverage) != set(assets):
        raise ValueError('Missing asset provenance')
    if len({x['name'] for x in manifest_entries}) != len(manifest_entries) or len({x['file']['name'] for x in source_entries}) != len(source_entries):
        raise ValueError('Source filenames must be unique')
    result = {'schema': 1, 'repository': REPOSITORY, 'tag': tag,
              'manifests': manifest_entries, 'sources': source_entries,
              'assets': [{**row['asset'], 'source_record': coverage[name],
                          'url': f'https://github.com/{REPOSITORY}/releases/download/{quote(tag)}/{quote(name)}'}
                         for name, row in sorted(assets.items())]}
    write_new(output, result)
    return result


def verify_index(path: Path, repo: Path, assets_root: Path) -> None:
    value = json.loads(path.read_text(encoding='utf-8'))
    if value.get('schema') != 1 or value.get('repository') != REPOSITORY:
        raise ValueError('Unsupported index')
    # Verify original sidecars and rebuild the complete relation without overwriting it.
    import tempfile
    with tempfile.TemporaryDirectory() as folder:
        base = Path(folder)
        manifests, records = [], []
        for row in value['manifests']:
            original = assets_root / row['name']
            if digest(original) != row:
                raise ValueError('Manifest digest mismatch')
            manifests.append(original)
        for row in value['sources']:
            original = assets_root / row['file']['name']
            if digest(original) != row['file'] or json.loads(original.read_text(encoding='utf-8')) != row['record']:
                raise ValueError('Source sidecar mismatch')
            records.append(original)
        rebuilt = index(repo, assets_root, manifests, records, base / 'index.json')
        if rebuilt != value:
            raise ValueError('Source index contents mismatch')
    print(f"Verified {len(value['assets'])} assets and all recorded Git inputs")


def main(argv=None) -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest='command', required=True)
    build = sub.add_parser('record')
    for name in ('source', 'packaging', 'assets-root', 'output'):
        build.add_argument('--' + name, type=Path, required=True)
    for name in ('scope', 'platform', 'tag', 'product-sha', 'packaging-sha', 'run-url', 'validation'):
        build.add_argument('--' + name, required=True)
    build.add_argument('--parameters', type=json.loads, required=True)
    build.add_argument('--asset', action='append', required=True)
    join = sub.add_parser('index')
    join.add_argument('--repo', type=Path, default=Path('.'))
    join.add_argument('--assets-root', type=Path, required=True)
    join.add_argument('--manifest', type=Path, action='append', required=True)
    join.add_argument('--record', type=Path, action='append', required=True)
    join.add_argument('--output', type=Path, required=True)
    verify = sub.add_parser('verify')
    verify.add_argument('--repo', type=Path, default=Path('.'))
    verify.add_argument('--assets-root', type=Path, required=True)
    verify.add_argument('--index', type=Path, required=True)
    args = parser.parse_args(argv)
    if args.command == 'record':
        record(args.source, args.packaging, args.scope, args.platform, args.tag,
               args.product_sha, args.packaging_sha, args.run_url, args.parameters,
               args.validation, args.assets_root, args.asset, args.output)
    elif args.command == 'index':
        index(args.repo, args.assets_root, args.manifest, args.record, args.output)
    else:
        verify_index(args.index, args.repo, args.assets_root)


if __name__ == '__main__':
    main()
