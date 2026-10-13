#!/bin/bash
set -euo pipefail
task_root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$task_root"
mkdir -p target/macos-result-presenter-tests
cargo build -p om-apple-ffi --release --locked
source macos/Scripts/native-math-dependencies.sh
prepare_native_math_dependencies
sources=()
while IFS= read -r file; do
  if [[ "$file" != macos/OpenMathNative/AppHost.swift ]]; then sources+=("$file"); fi
done < <(python3 -c 'from pathlib import Path; print("\n".join(str(p) for p in sorted(Path("macos/OpenMathNative").rglob("*.swift"))))')
swiftc -O -whole-module-optimization -swift-version 6 -parse-as-library -target arm64-apple-macos27.0 \
  -I macos/Headers -L target/release -lom_apple_ffi "${native_math_flags[@]}" \
  "${sources[@]}" macos/OpenMathNativeTests/ResultPresenterFixtures/main.swift \
  -o target/macos-result-presenter-tests/result-presenter-fixtures
install_native_math_resources target/macos-result-presenter-tests
attempt_root=$(mktemp -d "$task_root/target/macos-result-presenter-tests/attempt-XXXXXX")
target/macos-result-presenter-tests/result-presenter-fixtures "$attempt_root"
