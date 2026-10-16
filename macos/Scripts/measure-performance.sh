#!/bin/bash
# Reuse optimized production objects; emit only an internal interface for the measurement entry.
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$root"
mkdir -p target/plan-scope2
xcodebuild -project macos/OpenMathNative.xcodeproj -scheme OpenMathNative -configuration Release \
 -destination 'platform=macOS,arch=arm64' -derivedDataPath target/macos-native -jobs 4 \
 CODE_SIGN_IDENTITY=- CODE_SIGN_STYLE=Manual SWIFT_COMPILATION_MODE=incremental build > target/plan-scope2/performance-app-build.log 2>&1
source macos/Scripts/native-math-dependencies.sh
prepare_native_math_dependencies
source macos/Scripts/native-markdown-dependencies.sh
prepare_native_markdown_dependencies
interface_flags=()
for flag in "${native_math_flags[@]}" "${native_markdown_flags[@]}"; do
 [[ "$flag" == *.o ]] || interface_flags+=("$flag")
done
mkdir -p target/macos-performance/module
xcodebuild -project macos/OpenMathNative.xcodeproj -scheme OpenMathNative -configuration Release -showBuildSettings -json > target/macos-performance/build-settings.json
native_performance_module=$(python3 -c 'import json; v=json.load(open("target/macos-performance/build-settings.json")); print(next(x["buildSettings"]["PRODUCT_MODULE_NAME"] for x in v if x["target"]=="OpenMathNative"))')
sources=()
while IFS= read -r file; do
 [[ "$file" == macos/OpenMathNative/AppHost.swift ]] || sources+=("$file")
done < <(python3 -c 'from pathlib import Path; print("\n".join(str(p) for p in sorted(Path("macos/OpenMathNative").rglob("*.swift"))))')
swiftc -emit-module -enable-testing -module-name "$native_performance_module" -emit-module-path "target/macos-performance/module/$native_performance_module.swiftmodule" \
 -swift-version 6 -parse-as-library -target arm64-apple-macos27.0 -I macos/Headers \
 "${interface_flags[@]}" "${sources[@]}"
python3 - "$native_performance_module" <<'PY'
from pathlib import Path
import re,sys
module=sys.argv[1];assert re.fullmatch(r'[A-Za-z_]\w*',module)
source=Path('macos/OpenMathNativeTests/PerformanceFixtures/main.swift').read_text().replace('// @production-module-import','@testable import '+module)
Path('target/macos-performance/performance-main.swift').write_text(source)
p=Path('target/macos-native/Build/Intermediates.noindex/OpenMathNative.build/Release/OpenMathNative.build/Objects-normal/arm64/OpenMath Preview.LinkFileList')
objects=p.read_text().splitlines()
main=[x for x in objects if Path(x).name=='AppHost.o']
assert len(main)==1, 'production main object must be excluded exactly once'
Path('target/macos-performance/production-objects.txt').write_text('\n'.join(x for x in objects if x not in main)+'\n')
PY
swiftc -O -swift-version 6 -parse-as-library -target arm64-apple-macos27.0 -I macos/Headers -I target/macos-performance/module \
 -L target/release -lom_apple_ffi -lsqlite3 "${interface_flags[@]}" \
 -Xlinker -filelist -Xlinker target/macos-performance/production-objects.txt \
 target/macos-performance/performance-main.swift -o target/macos-performance/measure-performance
install_native_math_resources target/macos-performance
if [[ "${1:-}" != --build-only ]]; then
 attempt_root=$(mktemp -d "$root/target/macos-performance/attempt-XXXXXX")
 target/macos-performance/measure-performance "$attempt_root" ios/OpenMath/Resources/solve-corpus.json
fi
