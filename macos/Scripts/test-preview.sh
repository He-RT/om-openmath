#!/bin/bash
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$root"
storage_sources=()
for file in macos/OpenMathNative/Storage/*.swift; do
  if [[ "$file" != */CommitPort.swift ]]; then storage_sources+=("$file"); fi
done
mkdir -p target/macos-storage-tests
cargo test -p om-host-service --test preview --test references --locked
cargo run -p om-host-service --example preview_contract --locked > target/macos-storage-tests/frozen-preview.json
swiftc -swift-version 6 -parse-as-library -target arm64-apple-macos27.0 \
  macos/OpenMathNative/Generated/HostContractSupport.swift macos/OpenMathNative/Generated/Contracts/*.swift \
  "${storage_sources[@]}" macos/OpenMathNativeTests/PreviewFixtures/main.swift \
  -o target/macos-storage-tests/preview-fixtures
attempt_root=$(mktemp -d "$root/target/macos-storage-tests/preview-XXXXXX")
target/macos-storage-tests/preview-fixtures target/macos-storage-tests/frozen-preview.json "$attempt_root" \
  target/macos-storage-tests/frozen-preview-receipt.json
cargo run -p om-host-service --example preview_contract --locked -- verify target/macos-storage-tests/frozen-preview-receipt.json
