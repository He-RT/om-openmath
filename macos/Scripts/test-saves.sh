#!/bin/bash
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$root"
mkdir -p target/macos-storage-tests
cargo build -p om-apple-ffi --release --locked
clang -std=c11 -Wall -Wextra -Werror -dynamiclib macos/OpenMathNativeTests/StorageFixtures/SyncProbe.c -lsqlite3 \
  -install_name @rpath/libSaveSyncProbe.dylib -o target/macos-storage-tests/libSaveSyncProbe.dylib
swiftc -whole-module-optimization -swift-version 6 -parse-as-library -target arm64-apple-macos27.0 -I macos/Headers \
  -import-objc-header macos/OpenMathNativeTests/StorageFixtures/SyncProbe.h \
  -L target/release -lom_apple_ffi macos/OpenMathNative/Generated/HostContractSupport.swift \
  macos/OpenMathNative/Generated/Contracts/*.swift macos/OpenMathNative/Storage/*.swift \
  macos/OpenMathNative/Files/*.swift macos/OpenMathNative/NativeDocument.swift \
  macos/OpenMathNative/Editor/*.swift \
  macos/OpenMathNative/NativeHostClient.swift macos/OpenMathNative/AppEventRouter.swift macos/OpenMathNative/DraftStore.swift \
  macos/OpenMathNativeTests/SaveFixtures/*.swift \
  -L target/macos-storage-tests -lSaveSyncProbe -Xlinker -rpath -Xlinker "$root/target/macos-storage-tests" \
  -o target/macos-storage-tests/save-fixtures
attempt_root=$(mktemp -d "$root/target/macos-storage-tests/saves-XXXXXX")
DYLD_INSERT_LIBRARIES="$root/target/macos-storage-tests/libSaveSyncProbe.dylib" \
  target/macos-storage-tests/save-fixtures "$attempt_root"
python3 macos/OpenMathNativeTests/SaveFixtures/interruption.py
