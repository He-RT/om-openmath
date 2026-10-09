#!/bin/bash
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$root"
storage_sources=()
for file in macos/OpenMathNative/Storage/*.swift; do
  if [[ "$file" != */CommitPort.swift ]]; then storage_sources+=("$file"); fi
done
mkdir -p target/macos-storage-tests
swiftc -swift-version 6 -parse-as-library -target arm64-apple-macos27.0 \
  macos/OpenMathNative/Generated/HostContractSupport.swift macos/OpenMathNative/Generated/Contracts/*.swift \
  "${storage_sources[@]}" macos/OpenMathNativeTests/BlobFixtures/main.swift \
  -o target/macos-storage-tests/blob-fixtures
attempt_root=$(mktemp -d "$root/target/macos-storage-tests/blobs-XXXXXX")
target/macos-storage-tests/blob-fixtures "$attempt_root" "$root/macos/OpenMathNativeTests/Fixtures"

python3 macos/OpenMathNativeTests/BlobFixtures/interruption.py
