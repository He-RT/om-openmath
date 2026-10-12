#!/bin/bash
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$root"
mkdir -p target/macos-storage-tests
cargo run -p om-host-service --example source_store_contract --locked > target/macos-storage-tests/maintenance-plans.json
clang -std=c11 -Wall -Wextra -Werror -dynamiclib macos/OpenMathNativeTests/StorageFixtures/SyncProbe.c -lsqlite3 \
  -install_name @rpath/libMaintenanceSyncProbe.dylib -o target/macos-storage-tests/libMaintenanceSyncProbe.dylib
storage_sources=()
for file in macos/OpenMathNative/Storage/*.swift; do
  if [[ "$file" != */CommitPort.swift ]]; then storage_sources+=("$file"); fi
done
swiftc -whole-module-optimization -swift-version 6 -parse-as-library -target arm64-apple-macos27.0 \
  -import-objc-header macos/OpenMathNativeTests/StorageFixtures/SyncProbe.h \
  macos/OpenMathNative/Generated/HostContractSupport.swift macos/OpenMathNative/Generated/Contracts/*.swift \
  "${storage_sources[@]}" macos/OpenMathNativeTests/MaintenanceFixtures/main.swift \
  -L target/macos-storage-tests -lMaintenanceSyncProbe -Xlinker -rpath -Xlinker "$root/target/macos-storage-tests" \
  -o target/macos-storage-tests/maintenance-fixtures
attempt_root=$(mktemp -d "$root/target/macos-storage-tests/maintenance-XXXXXX")
target/macos-storage-tests/maintenance-fixtures "$attempt_root" target/macos-storage-tests/maintenance-plans.json
python3 macos/OpenMathNativeTests/MaintenanceFixtures/interruption.py
cargo run -p om-host-service --example validate_backup_manifest --locked -- target/acceptance/pre-alpha.4/development/r41/backup-canonical.json
