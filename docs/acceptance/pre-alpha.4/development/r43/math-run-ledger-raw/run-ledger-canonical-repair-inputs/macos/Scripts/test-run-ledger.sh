#!/bin/bash
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$root"
export CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0
mkdir -p target/macos-storage-tests
cargo build -p om-host-service --example kernel_store_contract --locked
clang -std=c11 -Wall -Wextra -Werror -dynamiclib macos/OpenMathNativeTests/StorageFixtures/SyncProbe.c -lsqlite3 \
  -install_name @rpath/libRunLedgerSyncProbe.dylib -o target/macos-storage-tests/libRunLedgerSyncProbe.dylib
storage_sources=()
for file in macos/OpenMathNative/Storage/*.swift; do
  if [[ "$file" != */CommitPort.swift ]]; then storage_sources+=("$file"); fi
done
swiftc -whole-module-optimization -swift-version 6 -parse-as-library -target arm64-apple-macos27.0 \
  -import-objc-header macos/OpenMathNativeTests/StorageFixtures/SyncProbe.h \
  macos/OpenMathNative/Generated/HostContractSupport.swift macos/OpenMathNative/Generated/Contracts/*.swift \
  "${storage_sources[@]}" macos/OpenMathNativeTests/RunLedgerFixtures/main.swift \
  -L target/macos-storage-tests -lRunLedgerSyncProbe -Xlinker -rpath -Xlinker "$root/target/macos-storage-tests" \
  -o target/macos-storage-tests/run-ledger-fixtures
python3 macos/OpenMathNativeTests/RunLedgerFixtures/interruption.py
