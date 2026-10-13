#!/bin/bash
set -euo pipefail
task_root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$task_root"
[[ $# == 1 ]] || { echo 'Provide the actual math fixture corpus JSON' >&2; exit 1; }
recorded="$1"
fixture_dir=target/acceptance/pre-alpha.4/development/r42/math-ui-fixture
fixture_app="$fixture_dir/OpenMath Native Formula Fixture.app"
mkdir -p "$fixture_app/Contents/MacOS" "$fixture_app/Contents/Resources"
source macos/Scripts/native-math-dependencies.sh
prepare_native_math_dependencies
swiftc -O -whole-module-optimization -swift-version 6 -parse-as-library -target arm64-apple-macos27.0 \
  "${native_math_flags[@]}" macos/OpenMathNative/Rendering/MathRenderAdapter.swift \
  macos/OpenMathNative/Rendering/NativeMathView.swift macos/OpenMathNativeTests/MathRenderUIFixture/main.swift \
  -o "$fixture_app/Contents/MacOS/MathRenderUIFixture"
python3 - "$fixture_app" "$recorded" <<'PY'
from pathlib import Path
import json,plistlib,shutil,sys
app=Path(sys.argv[1]);data=Path(sys.argv[2]);records=json.loads(data.read_text())
assert len(records)==57 and all('original_latex' in out for record in records for out in record['outputs'])
info={'CFBundleExecutable':'MathRenderUIFixture','CFBundleIdentifier':'org.openmath.OpenMath.MathRenderUIFixture','CFBundleName':'OpenMath Native Formula Fixture','CFBundlePackageType':'APPL','NSPrincipalClass':'NSApplication','LSMinimumSystemVersion':'27.0'}
(app/'Contents/Info.plist').write_bytes(plistlib.dumps(info))
shutil.copyfile(data,app/'Contents/Resources/RecordedLatex.json')
PY
install_native_math_resources "$fixture_app/Contents/Resources"
codesign --force --sign - "$fixture_app"
printf '%s\n' "$fixture_app"
