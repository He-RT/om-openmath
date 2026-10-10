#!/bin/bash
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$root"
mkdir -p target/macos-storage-tests
cargo run -p om-host-service --example kernel_store_contract --locked -- seed target/macos-storage-tests/kernel-runtime-source
cargo build -p om-apple-ffi --release --locked
clang -std=c11 -Wall -Wextra -Werror -dynamiclib macos/OpenMathNativeTests/StorageFixtures/SyncProbe.c -lsqlite3 \
  -install_name @rpath/libKernelRuntimeSyncProbe.dylib -o target/macos-storage-tests/libKernelRuntimeSyncProbe.dylib
swiftc -swift-version 6 -parse-as-library -target arm64-apple-macos27.0 -I macos/Headers \
  -import-objc-header macos/OpenMathNativeTests/StorageFixtures/SyncProbe.h \
  -L target/release -lom_apple_ffi macos/OpenMathNative/Generated/HostContractSupport.swift \
  macos/OpenMathNative/Generated/Contracts/*.swift macos/OpenMathNative/Storage/*.swift \
  macos/OpenMathNative/Kernel/*.swift macos/OpenMathNative/NativeHostClient.swift \
  macos/OpenMathNative/AppEventRouter.swift macos/OpenMathNative/DraftStore.swift \
  macos/OpenMathNativeTests/KernelRuntimeFixtures/main.swift \
  -L target/macos-storage-tests -lKernelRuntimeSyncProbe -Xlinker -rpath -Xlinker "$root/target/macos-storage-tests" \
  -o target/macos-storage-tests/kernel-runtime-fixtures
attempt_root=$(mktemp -d "$root/target/macos-storage-tests/kernel-runtime-XXXXXX")
DYLD_INSERT_LIBRARIES="$root/target/macos-storage-tests/libKernelRuntimeSyncProbe.dylib" \
  target/macos-storage-tests/kernel-runtime-fixtures "$attempt_root" target/macos-storage-tests/kernel-runtime-source/source.json
