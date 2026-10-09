#!/bin/bash
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$root"
mkdir -p target/macos-storage-tests
cargo test -p om-host-service --test commit_owner --test source_endpoint --lib --locked
cargo run -p om-host-service --example source_store_contract --locked > target/macos-storage-tests/source-plans.json
cargo build -p om-apple-ffi --release --locked
swiftc -swift-version 6 -parse-as-library -target arm64-apple-macos27.0 -I macos/Headers \
  -L target/release -lom_apple_ffi macos/OpenMathNative/Generated/HostContractSupport.swift \
  macos/OpenMathNative/Generated/Contracts/*.swift macos/OpenMathNative/Storage/*.swift \
  macos/OpenMathNative/NativeHostClient.swift macos/OpenMathNative/AppEventRouter.swift \
  macos/OpenMathNative/DraftStore.swift macos/OpenMathNative/Editor/DraftTextViewAdapter.swift \
  macos/OpenMathNativeTests/CommitFixtures/main.swift -o target/macos-storage-tests/commit-fixtures
attempt_root=$(mktemp -d "$root/target/macos-storage-tests/commit-XXXXXX")
target/macos-storage-tests/commit-fixtures target/macos-storage-tests/source-plans.json "$attempt_root" target/macos-storage-tests/commit-report.json
