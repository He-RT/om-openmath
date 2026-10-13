#!/bin/bash
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$root"
mkdir -p target/macos-workbench-tests
cargo build -p om-apple-ffi --release --locked
source macos/Scripts/native-math-dependencies.sh
prepare_native_math_dependencies
sources=()
while IFS= read -r file; do
  if [[ "$file" != macos/OpenMathNative/AppHost.swift ]]; then sources+=("$file"); fi
done < <(python3 -c 'from pathlib import Path; print("\n".join(str(p) for p in sorted(Path("macos/OpenMathNative").rglob("*.swift"))))')
swiftc -O -whole-module-optimization -swift-version 6 -parse-as-library -target arm64-apple-macos27.0 \
  -I macos/Headers -L target/release -lom_apple_ffi \
  "${native_math_flags[@]}" "${sources[@]}" macos/OpenMathNativeTests/WorkbenchFixtures/main.swift \
  -o target/macos-workbench-tests/workbench-fixtures
install_native_math_resources "target/macos-workbench-tests"
attempt_root=$(mktemp -d "$root/target/macos-workbench-tests/attempt-XXXXXX")
target/macos-workbench-tests/workbench-fixtures "$attempt_root"
