#!/bin/bash
set -euo pipefail
task_root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$task_root"
[[ $# == 1 || ( $# == 2 && "$2" == structured ) ]] || { echo 'Provide the actual result input JSON, optionally structured' >&2; exit 1; }
recorded="$1"
fixture_dir=target/acceptance/pre-alpha.4/development/r42/result-ui-fixture
fixture_name='OpenMath Native Result Fixture'
fixture_id=org.openmath.OpenMath.ResultPresenterUIFixture
if [[ ${2:-} == structured ]]; then
  fixture_dir=target/acceptance/pre-alpha.4/development/r42/value-ui-fixture
  fixture_name='OpenMath Native Value Fixture'
  fixture_id=org.openmath.OpenMath.ValuePresenterUIFixture
fi
fixture_app="$fixture_dir/$fixture_name.app"
mkdir -p "$fixture_app/Contents/MacOS" "$fixture_app/Contents/Resources"
cargo build -p om-apple-ffi --release --locked
source macos/Scripts/native-math-dependencies.sh
prepare_native_math_dependencies
sources=()
while IFS= read -r file; do
  if [[ "$file" != macos/OpenMathNative/AppHost.swift ]]; then sources+=("$file"); fi
done < <(python3 -c 'from pathlib import Path; print("\n".join(str(p) for p in sorted(Path("macos/OpenMathNative").rglob("*.swift"))))')
swiftc -O -whole-module-optimization -swift-version 6 -parse-as-library -target arm64-apple-macos27.0 \
  -I macos/Headers -L target/release -lom_apple_ffi "${native_math_flags[@]}" "${sources[@]}" \
  macos/OpenMathNativeTests/ResultPresenterUIFixture/main.swift -o "$fixture_app/Contents/MacOS/ResultPresenterUIFixture"
python3 - "$fixture_app" "$recorded" "$fixture_id" "$fixture_name" <<'PY'
from pathlib import Path
import json,plistlib,shutil,sys
app=Path(sys.argv[1]);data=Path(sys.argv[2]);records=json.loads(data.read_text())
assert len(records)>0 and all('cell' in row and 'input' in row for row in records)
info={'CFBundleExecutable':'ResultPresenterUIFixture','CFBundleIdentifier':sys.argv[3],'CFBundleName':sys.argv[4],'CFBundlePackageType':'APPL','NSPrincipalClass':'NSApplication','LSMinimumSystemVersion':'27.0'}
(app/'Contents/Info.plist').write_bytes(plistlib.dumps(info));shutil.copyfile(data,app/'Contents/Resources/ActualInputs.json')
PY
install_native_math_resources "$fixture_app/Contents/Resources"
codesign --force --sign - "$fixture_app"
printf '%s\n' "$fixture_app"
