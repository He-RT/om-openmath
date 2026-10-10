#!/bin/bash
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$root"
mkdir -p target/macos-storage-tests
cargo build -p om-apple-ffi --release --locked
swiftc -swift-version 6 -parse-as-library -target arm64-apple-macos27.0 -I macos/Headers \
  -L target/release -lom_apple_ffi macos/OpenMathNative/Generated/HostContractSupport.swift \
  macos/OpenMathNative/Generated/Contracts/*.swift macos/OpenMathNative/Storage/*.swift \
  macos/OpenMathNative/Kernel/*.swift macos/OpenMathNative/Rendering/ResultClient.swift \
  macos/OpenMathNative/NativeHostClient.swift macos/OpenMathNative/AppEventRouter.swift macos/OpenMathNative/DraftStore.swift \
  macos/OpenMathNativeTests/ResultFixtures/main.swift -o target/macos-storage-tests/result-fixtures
attempt_root=$(mktemp -d "$root/target/macos-storage-tests/results-XXXXXX")
target/macos-storage-tests/result-fixtures "$attempt_root"
