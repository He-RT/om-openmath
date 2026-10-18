#!/bin/bash
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$root"
mkdir -p target/macos-workbench-tests
cargo build -p om-apple-ffi --release --locked
source macos/Scripts/native-math-dependencies.sh
prepare_native_math_dependencies
source macos/Scripts/native-markdown-dependencies.sh
prepare_native_markdown_dependencies
native_math_flags+=("${native_markdown_flags[@]}")
fixture_source=macos/OpenMathNativeTests/WorkbenchFixtures/main.swift
fixture_binary=target/macos-workbench-tests/workbench-fixtures
fixture_extra=()
if [[ "${1:-}" == --settings ]]; then
  fixture_source=macos/OpenMathNativeTests/SettingsFixtures/main.swift
  fixture_binary=target/macos-workbench-tests/settings-fixtures
  fixture_extra+=(macos/OpenMathNativeTests/ProviderTransportFixtures/LoopbackServer.swift)
fi
sources=()
while IFS= read -r file; do
  if [[ "$file" != macos/OpenMathNative/AppHost.swift ]]; then sources+=("$file"); fi
done < <(python3 -c 'from pathlib import Path; print("\n".join(str(p) for p in sorted(Path("macos/OpenMathNative").rglob("*.swift"))))')
swiftc -O -whole-module-optimization -swift-version 6 -parse-as-library -target arm64-apple-macos27.0 \
  -I macos/Headers -L target/release -lom_apple_ffi \
  "${native_math_flags[@]}" "${sources[@]}" "$fixture_source" "${fixture_extra[@]}" \
  -o "$fixture_binary"
install_native_math_resources "target/macos-workbench-tests"
xcrun actool macos/OpenMathNative/Assets.xcassets --compile target/macos-workbench-tests \
  --platform macosx --minimum-deployment-target 27.0 --accent-color AccentColor \
  --output-format human-readable-text
attempt_root=$(mktemp -d "$root/target/macos-workbench-tests/attempt-XXXXXX")
if [[ "${1:-}" == --settings ]]; then "$fixture_binary" "--root=$attempt_root" "$@"
else "$fixture_binary" "$attempt_root" "$@";fi
