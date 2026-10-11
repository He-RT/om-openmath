#!/bin/bash
set -euo pipefail
task_root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$task_root"
fixture_dir=target/acceptance/pre-alpha.4/development/r41/file-panel-fixture
fixture_app="$fixture_dir/OpenMath File Panel Fixture.app"
mkdir -p "$fixture_app/Contents/MacOS" "$fixture_dir/user-files"
swiftc -swift-version 6 -parse-as-library -target arm64-apple-macos27.0 \
  macos/OpenMathNativeTests/FilePanelFixtures/main.swift -o "$fixture_app/Contents/MacOS/FilePanelFixture"
python3 - "$fixture_app" <<'PY'
from pathlib import Path
import plistlib,sys
app=Path(sys.argv[1])
info={'CFBundleExecutable':'FilePanelFixture','CFBundleIdentifier':'org.openmath.OpenMath.FilePanelFixture','CFBundleName':'OpenMath File Panel Fixture','CFBundlePackageType':'APPL','NSPrincipalClass':'NSApplication','LSMinimumSystemVersion':'27.0','UTExportedTypeDeclarations':[{'UTTypeIdentifier':'org.openmath.notebook','UTTypeConformsTo':['public.json'],'UTTypeTagSpecification':{'public.filename-extension':['omnb']}}]}
(app/'Contents/Info.plist').write_bytes(plistlib.dumps(info))
PY
codesign --force --sign - "$fixture_app"
printf '%s\n' "$fixture_app"
