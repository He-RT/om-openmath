#!/bin/bash
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$root"
mkdir -p target/macos-storage-tests
clang -std=c11 -Wall -Wextra -Werror -dynamiclib \
  macos/OpenMathNativeTests/StorageFixtures/SyncProbe.c -lsqlite3 \
  -install_name @rpath/libStorageSyncProbe.dylib -o target/macos-storage-tests/libStorageSyncProbe.dylib
swiftc -swift-version 6 -parse-as-library -target arm64-apple-macos27.0 \
  -import-objc-header macos/OpenMathNativeTests/StorageFixtures/SyncProbe.h \
  macos/OpenMathNative/Storage/*.swift macos/OpenMathNativeTests/StorageFixtures/main.swift \
  -L target/macos-storage-tests -lStorageSyncProbe -Xlinker -rpath -Xlinker "$root/target/macos-storage-tests" \
  -o target/macos-storage-tests/storage-fixtures
attempt_root=$(mktemp -d "$root/target/macos-storage-tests/attempt-XXXXXX")
DYLD_INSERT_LIBRARIES="$root/target/macos-storage-tests/libStorageSyncProbe.dylib" \
  target/macos-storage-tests/storage-fixtures "$attempt_root"
python3 macos/OpenMathNativeTests/StorageFixtures/interruption.py
