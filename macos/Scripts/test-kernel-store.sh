#!/bin/bash
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$root"
storage_sources=()
for file in macos/OpenMathNative/Storage/*.swift; do
  if [[ "$file" != */CommitPort.swift ]]; then storage_sources+=("$file"); fi
done
mkdir -p target/macos-storage-tests
cargo build -p om-host-service --example kernel_store_contract --locked
clang -std=c11 -Wall -Wextra -Werror -dynamiclib macos/OpenMathNativeTests/StorageFixtures/SyncProbe.c -lsqlite3 \
  -install_name @rpath/libKernelSyncProbe.dylib -o target/macos-storage-tests/libKernelSyncProbe.dylib
swiftc -swift-version 6 -parse-as-library -target arm64-apple-macos27.0 \
  -import-objc-header macos/OpenMathNativeTests/StorageFixtures/SyncProbe.h \
  macos/OpenMathNative/Generated/HostContractSupport.swift macos/OpenMathNative/Generated/Contracts/*.swift \
  "${storage_sources[@]}" macos/OpenMathNativeTests/KernelFixtures/main.swift \
  -L target/macos-storage-tests -lKernelSyncProbe -Xlinker -rpath -Xlinker "$root/target/macos-storage-tests" \
  -o target/macos-storage-tests/kernel-fixtures
attempt_root=$(mktemp -d "$root/target/macos-storage-tests/kernel-XXXXXX")
fixture="$attempt_root/contract"
store="$attempt_root/physical"
target/debug/examples/kernel_store_contract seed "$fixture"
target/macos-storage-tests/kernel-fixtures initialize "$fixture" "$store"
target/debug/examples/kernel_store_contract prepare "$fixture" bootstrap
target/macos-storage-tests/kernel-fixtures commit "$fixture" "$store" bootstrap
target/debug/examples/kernel_store_contract verify "$fixture" bootstrap bootstrap
previous=bootstrap
for stage in a b error; do
  target/debug/examples/kernel_store_contract prepare "$fixture" "$stage" "$previous"
  target/macos-storage-tests/kernel-fixtures commit "$fixture" "$store" "$stage"
  target/debug/examples/kernel_store_contract verify "$fixture" "$stage" "$stage"
  previous="$stage"
done
sync_root=$(mktemp -d "$root/target/macos-storage-tests/kernel-fullsync-XXXXXX")
sync_fixture="$sync_root/contract"
sync_store="$sync_root/physical"
target/debug/examples/kernel_store_contract seed "$sync_fixture"
target/macos-storage-tests/kernel-fixtures initialize "$sync_fixture" "$sync_store"
target/debug/examples/kernel_store_contract prepare "$sync_fixture" bootstrap
target/macos-storage-tests/kernel-fixtures commit "$sync_fixture" "$sync_store" bootstrap
target/debug/examples/kernel_store_contract prepare "$sync_fixture" a bootstrap
target/macos-storage-tests/kernel-fixtures commit "$sync_fixture" "$sync_store" a
target/debug/examples/kernel_store_contract prepare "$sync_fixture" b a
DYLD_INSERT_LIBRARIES="$root/target/macos-storage-tests/libKernelSyncProbe.dylib" \
  target/macos-storage-tests/kernel-fixtures commit "$sync_fixture" "$sync_store" b fullsync-fault
target/debug/examples/kernel_store_contract verify "$sync_fixture" b b
python3 macos/OpenMathNativeTests/KernelFixtures/interruption.py
