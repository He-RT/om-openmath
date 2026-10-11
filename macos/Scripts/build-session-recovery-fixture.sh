#!/bin/bash
set -euo pipefail
task_root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$task_root"
fixture_root="$1"
fixture_dir=target/acceptance/pre-alpha.4/development/r41/session-recovery-ui-fixture
fixture_app="$fixture_dir/OpenMath Session Recovery Fixture.app"
mkdir -p "$fixture_app/Contents/MacOS" "$fixture_app/Contents/Resources"
cargo build -p om-apple-ffi --release --locked
sources=()
while IFS= read -r file; do
  if [[ "$file" != macos/OpenMathNative/AppHost.swift ]]; then sources+=("$file"); fi
done < <(rg --files macos/OpenMathNative -g '*.swift' | sort)
swiftc -O -whole-module-optimization -swift-version 6 -parse-as-library -target arm64-apple-macos27.0 \
  -I macos/Headers -L target/release -lom_apple_ffi \
  "${sources[@]}" macos/OpenMathNativeTests/SessionRecoveryUIFixture/main.swift \
  -o "$fixture_app/Contents/MacOS/SessionRecoveryFixture"
python3 - "$fixture_app" "$fixture_root" <<'PY'
from pathlib import Path
import plistlib,sys
app=Path(sys.argv[1]);root=Path(sys.argv[2]).resolve();assert (root/'Library/active.json').exists()
info={'CFBundleExecutable':'SessionRecoveryFixture','CFBundleIdentifier':'org.openmath.OpenMath.SessionRecoveryFixture','CFBundleName':'OpenMath Session Recovery Fixture','CFBundlePackageType':'APPL','NSPrincipalClass':'NSApplication','LSMinimumSystemVersion':'27.0'}
(app/'Contents/Info.plist').write_bytes(plistlib.dumps(info))
(app/'Contents/Resources/FixtureRoot.txt').write_text(str(root)+'\n')
PY
codesign --force --sign - "$fixture_app"
printf '%s\n' "$fixture_app"
