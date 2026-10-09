#!/bin/bash
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$root"
mkdir -p target/macos-storage-tests
cargo run -p om-host-service --example source_store_contract --locked \
  > target/macos-storage-tests/source-plans.json
clang -std=c11 -Wall -Wextra -Werror -dynamiclib macos/OpenMathNativeTests/StorageFixtures/SyncProbe.c -lsqlite3 \
  -install_name @rpath/libStorageSyncProbe.dylib -o target/macos-storage-tests/libStorageSyncProbe.dylib
swiftc -swift-version 6 -parse-as-library -target arm64-apple-macos27.0 \
  -import-objc-header macos/OpenMathNativeTests/StorageFixtures/SyncProbe.h \
  macos/OpenMathNative/Generated/HostContractSupport.swift macos/OpenMathNative/Generated/Contracts/*.swift \
  macos/OpenMathNative/Storage/*.swift macos/OpenMathNativeTests/DocumentFixtures/main.swift \
  -L target/macos-storage-tests -lStorageSyncProbe -Xlinker -rpath -Xlinker "$root/target/macos-storage-tests" \
  -o target/macos-storage-tests/document-fixtures
attempt_root=$(mktemp -d "$root/target/macos-storage-tests/document-XXXXXX")
DYLD_INSERT_LIBRARIES="$root/target/macos-storage-tests/libStorageSyncProbe.dylib" \
  target/macos-storage-tests/document-fixtures target/macos-storage-tests/source-plans.json "$attempt_root" \
  target/macos-storage-tests/actual-source-receipt.json
cargo run -p om-host-service --example source_store_contract --locked -- \
  verify target/macos-storage-tests/actual-source-receipt.json

fullsync_root=$(mktemp -d "$root/target/macos-storage-tests/source-fullsync-XXXXXX")
DYLD_INSERT_LIBRARIES="$root/target/macos-storage-tests/libStorageSyncProbe.dylib" \
  target/macos-storage-tests/document-fixtures target/macos-storage-tests/source-plans.json "$fullsync_root" \
  target/macos-storage-tests/actual-source-fullsync-receipt.json fullsync-fault
cargo run -p om-host-service --example source_store_contract --locked -- \
  verify target/macos-storage-tests/actual-source-fullsync-receipt.json
