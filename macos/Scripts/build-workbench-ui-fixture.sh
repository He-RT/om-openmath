#!/bin/bash
set -euo pipefail
task_root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$task_root"
fixture_dir="${1:-target/acceptance/pre-alpha.4/development/r42/workbench-ui-fixture}"
fixture_app="$fixture_dir/OpenMath Workbench Fixture.app"
mkdir -p "$fixture_app/Contents/MacOS" "$fixture_app/Contents/Resources" "$fixture_dir/data"
cargo build -p om-apple-ffi --release --locked
source macos/Scripts/native-math-dependencies.sh
prepare_native_math_dependencies
source macos/Scripts/native-markdown-dependencies.sh
prepare_native_markdown_dependencies
sources=()
while IFS= read -r file; do
  if [[ "$file" != macos/OpenMathNative/AppHost.swift ]]; then sources+=("$file"); fi
done < <(python3 -c 'from pathlib import Path; print("\n".join(str(p) for p in sorted(Path("macos/OpenMathNative").rglob("*.swift"))))')
swiftc -O -whole-module-optimization -swift-version 6 -parse-as-library -target arm64-apple-macos27.0 \
  -I macos/Headers -L target/release -lom_apple_ffi \
  "${native_math_flags[@]}" "${native_markdown_flags[@]}" "${sources[@]}" macos/OpenMathNativeTests/WorkbenchUIFixture/main.swift \
  -o "$fixture_app/Contents/MacOS/WorkbenchUIFixture"
python3 - "$fixture_app" "$fixture_dir/data" <<'PY'
from pathlib import Path
import json,plistlib,sys
app=Path(sys.argv[1]);root=Path(sys.argv[2]).resolve()
info={'CFBundleExecutable':'WorkbenchUIFixture','CFBundleIdentifier':'org.openmath.OpenMath.WorkbenchUIFixture','CFBundleName':'OpenMath Workbench Fixture','CFBundlePackageType':'APPL','NSPrincipalClass':'NSApplication','LSMinimumSystemVersion':'27.0'}
(app/'Contents/Info.plist').write_bytes(plistlib.dumps(info))
(app/'Contents/Resources/FixtureRoot.txt').write_text(str(root)+'\n')
(root/'fixture.omnb').write_text(json.dumps({'version':1,'title':'原生工作台验收','cells':[{'id':'a','kind':'Math','source':'let a=2 # 中文🙂','dialect':'Modern'},{'id':'b','kind':'Math','source':'a+1','dialect':'Modern'},{'id':'long','kind':'Math','source':'map(fn(k)=>sin(k),range(1,1000000))','dialect':'Modern'}]},ensure_ascii=False))
PY
install_native_math_resources "$fixture_app/Contents/Resources"
if [[ $# -ge 2 ]]; then cp "$2" "$fixture_app/Contents/Resources/ActionTranscript.json"; fi
codesign --force --sign - "$fixture_app"
printf '%s\n' "$fixture_app"
