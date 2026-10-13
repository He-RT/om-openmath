#!/bin/bash
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$root"
mkdir -p target/macos-math-tests
cargo build -p om-apple-ffi --release --locked
source macos/Scripts/native-math-dependencies.sh
prepare_native_math_dependencies
sources=()
while IFS= read -r file; do
  if [[ "$file" != macos/OpenMathNative/AppHost.swift ]]; then sources+=("$file"); fi
done < <(python3 -c 'from pathlib import Path; print("\n".join(str(p) for p in sorted(Path("macos/OpenMathNative").rglob("*.swift"))))')
swiftc -O -whole-module-optimization -swift-version 6 -parse-as-library -target arm64-apple-macos27.0 \
  -I macos/Headers -L target/release -lom_apple_ffi "${native_math_flags[@]}" \
  "${sources[@]}" macos/OpenMathNativeTests/MathRenderFixtures/main.swift \
  -o target/macos-math-tests/math-render-fixtures
install_native_math_resources target/macos-math-tests
attempt_root=$(mktemp -d "$root/target/macos-math-tests/attempt-XXXXXX")
target/macos-math-tests/math-render-fixtures "$attempt_root" ios/OpenMath/Resources/solve-corpus.json
