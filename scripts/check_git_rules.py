"""Cheap repository hygiene checks; never delete or normalize evidence."""
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
IGNORED = (
    '.vscode/settings.json', 'app/.vscode/settings.json', 'agent/.idea/workspace.xml',
    'app/local.code-workspace', 'app/npm-debug.log.1', 'npm-debug.log',
    'target/a.xcresult/Info.plist', 'macos/smoke.xcresult/Info.plist',
    'ios/smoke.dSYM/Contents/Info.plist', 'app/node_modules/package/index.js',
    'app/dist/index.html', 'app/src-tauri/gen/android/local.properties',
    'app/.env.local', 'app/signing.p12', 'agent/coverage/index.html',
)
VISIBLE = (
    'Cargo.lock', 'app/package-lock.json', 'agent/package-lock.json',
    'ios/Package.resolved', 'tests/fixtures/mesh.obj', 'tests/fixtures/result.log',
    'tests/fixtures/store.sqlite', 'tests/fixtures/store.sqlite-wal',
    'tests/fixtures/store.sqlite-shm', 'tests/fixtures/store.lock',
    'tests/fixtures/Blobs/sha256/abc', 'app/.env.example', 'agent/.env.test.example',
    'crates/om-llm/tests/fixtures/reply.sse',
    'docs/acceptance/pre-alpha.4/development/r43/case-raw/result.log',
    'docs/acceptance/pre-alpha.4/development/r43/case-raw/run.xcresult/Info.plist',
    'docs/acceptance/pre-alpha.4/development/r43/case-raw/run.dSYM/Contents/Info.plist',
)


def check() -> None:
    for ignored, paths in ((True, IGNORED), (False, VISIBLE)):
        for path in paths:
            result = subprocess.run(['git', 'check-ignore', '--no-index', '-q', path], cwd=ROOT)
            if result.returncode != (0 if ignored else 1):
                raise ValueError(f'ignore boundary mismatch: {path} (expected ignored={ignored})')
    tracked = subprocess.check_output(['git', 'ls-files', '-ci', '--exclude-standard'], cwd=ROOT)
    if tracked.strip():
        raise ValueError('Tracked files unexpectedly ignored:\n' + tracked.decode())
    print(f'Git rules: {len(IGNORED)} ignored / {len(VISIBLE)} visible samples; no ignored tracked files')


if __name__ == '__main__':
    check()
