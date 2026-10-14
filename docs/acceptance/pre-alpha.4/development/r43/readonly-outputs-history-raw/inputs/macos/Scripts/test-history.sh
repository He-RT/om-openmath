#!/bin/bash
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$root"
mkdir -p target/macos-storage-tests
cargo run -p om-host-service --example history_store_contract --locked > target/macos-storage-tests/history-plans.json
storage_sources=()
for file in macos/OpenMathNative/Storage/*.swift; do
  if [[ "$file" != */CommitPort.swift ]]; then storage_sources+=("$file"); fi
done
swiftc -swift-version 6 -parse-as-library -target arm64-apple-macos27.0 \
  macos/OpenMathNative/Generated/HostContractSupport.swift macos/OpenMathNative/Generated/Contracts/*.swift \
  "${storage_sources[@]}" macos/OpenMathNativeTests/HistoryFixtures/main.swift -o target/macos-storage-tests/history-fixtures
attempt_root=$(mktemp -d "$root/target/macos-storage-tests/history-XXXXXX")
target/macos-storage-tests/history-fixtures target/macos-storage-tests/history-plans.json "$attempt_root" target/macos-storage-tests/history-report.json
