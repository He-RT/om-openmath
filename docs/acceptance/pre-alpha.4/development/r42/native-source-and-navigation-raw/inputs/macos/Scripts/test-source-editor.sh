#!/bin/bash
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$root"
mkdir -p target/macos-source-editor-tests
cargo build -p om-apple-ffi --release --locked
sources=()
while IFS= read -r file; do
  if [[ "$file" != macos/OpenMathNative/AppHost.swift ]]; then sources+=("$file"); fi
done < <(python3 -c 'from pathlib import Path; print("\n".join(str(p) for p in sorted(Path("macos/OpenMathNative").rglob("*.swift"))))')
swiftc -O -whole-module-optimization -swift-version 6 -parse-as-library -target arm64-apple-macos27.0 \
  -I macos/Headers -L target/release -lom_apple_ffi \
  "${sources[@]}" macos/OpenMathNativeTests/SourceEditorFixtures/main.swift \
  -o target/macos-source-editor-tests/source-editor-fixtures
attempt_root=$(mktemp -d "$root/target/macos-source-editor-tests/attempt-XXXXXX")
target/macos-source-editor-tests/source-editor-fixtures "$attempt_root"
