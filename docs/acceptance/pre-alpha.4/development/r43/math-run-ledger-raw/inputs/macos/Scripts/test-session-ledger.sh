#!/bin/bash
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$root"
export CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0
mkdir -p target/macos-storage-tests
cargo run -p om-host-service --example source_store_contract --locked > target/macos-storage-tests/session-ledger-plans.json
clang -std=c11 -Wall -Wextra -Werror -dynamiclib macos/OpenMathNativeTests/StorageFixtures/SyncProbe.c -lsqlite3 \
  -install_name @rpath/libSessionLedgerSyncProbe.dylib -o target/macos-storage-tests/libSessionLedgerSyncProbe.dylib
storage_sources=()
for file in macos/OpenMathNative/Storage/*.swift; do
  if [[ "$file" != */CommitPort.swift ]]; then storage_sources+=("$file"); fi
done
swiftc -whole-module-optimization -swift-version 6 -parse-as-library -target arm64-apple-macos27.0 \
  -import-objc-header macos/OpenMathNativeTests/StorageFixtures/SyncProbe.h \
  macos/OpenMathNative/Generated/HostContractSupport.swift macos/OpenMathNative/Generated/Contracts/*.swift \
  "${storage_sources[@]}" macos/OpenMathNativeTests/SessionLedgerFixtures/main.swift \
  -L target/macos-storage-tests -lSessionLedgerSyncProbe -Xlinker -rpath -Xlinker "$root/target/macos-storage-tests" \
  -o target/macos-storage-tests/session-ledger-fixtures
python3 macos/OpenMathNativeTests/SessionLedgerFixtures/interruption.py
cargo run -p om-host-service --example validate_session_events --locked -- target/acceptance/pre-alpha.4/development/r41/session-ledger-canonical-events.json
