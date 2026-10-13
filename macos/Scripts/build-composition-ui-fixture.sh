#!/bin/bash
set -euo pipefail
task_root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$task_root"
fixture_dir=target/acceptance/pre-alpha.4/development/r42/composition-ui-fixture
fixture_app="$fixture_dir/OpenMath Native IME Fixture.app"
mkdir -p "$fixture_app/Contents/MacOS" "$fixture_app/Contents/Resources" "$fixture_dir/data"
cargo build -p om-apple-ffi --release --locked
source macos/Scripts/native-math-dependencies.sh
prepare_native_math_dependencies
sources=()
while IFS= read -r file; do
  if [[ "$file" != macos/OpenMathNative/AppHost.swift ]]; then sources+=("$file"); fi
done < <(python3 -c 'from pathlib import Path; print("\n".join(str(p) for p in sorted(Path("macos/OpenMathNative").rglob("*.swift"))))')
swiftc -O -whole-module-optimization -swift-version 6 -parse-as-library -target arm64-apple-macos27.0 \
  -I macos/Headers -L target/release -lom_apple_ffi \
  "${native_math_flags[@]}" "${sources[@]}" macos/OpenMathNativeTests/CompositionUIFixture/main.swift \
  -o "$fixture_app/Contents/MacOS/CompositionUIFixture"
python3 - "$fixture_app" "$fixture_dir/data" <<'PY'
from pathlib import Path
import json,plistlib,sys
app=Path(sys.argv[1]);root=Path(sys.argv[2]).resolve()
info={'CFBundleExecutable':'CompositionUIFixture','CFBundleIdentifier':'org.openmath.OpenMath.CompositionUIFixture','CFBundleName':'OpenMath Native IME Fixture','CFBundlePackageType':'APPL','NSPrincipalClass':'NSApplication','LSMinimumSystemVersion':'27.0'}
(app/'Contents/Info.plist').write_bytes(plistlib.dumps(info))
(app/'Contents/Resources/FixtureRoot.txt').write_text(str(root)+'\n')
path=root/'composition.omnb'
if not path.exists():
 path.write_text(json.dumps({'version':1,'title':'系统输入法验收','cells':[{'id':'Chinese','kind':'Math','source':'# 中文候选验收：','dialect':'Modern'},{'id':'Japanese','kind':'Math','source':'# 日本語候補：','dialect':'Modern'}]},ensure_ascii=False))
PY
install_native_math_resources "$fixture_app/Contents/Resources"
codesign --force --sign - "$fixture_app"
printf '%s\n' "$fixture_app"
