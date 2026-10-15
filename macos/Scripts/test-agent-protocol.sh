#!/bin/bash
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$root"
export CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0
mkdir -p target/macos-agent-protocol-tests
cargo build -p om-apple-ffi --release --locked
swiftc -whole-module-optimization -swift-version 6 -parse-as-library -target arm64-apple-macos27.0 -I macos/Headers \
  -L target/release -lom_apple_ffi macos/OpenMathNative/Generated/HostContractSupport.swift \
  macos/OpenMathNative/Generated/Contracts/*.swift macos/OpenMathNative/Storage/*.swift \
  macos/OpenMathNative/Kernel/KernelCommitPort.swift \
  macos/OpenMathNative/NativeHostClient.swift macos/OpenMathNative/AppEventRouter.swift macos/OpenMathNative/DraftStore.swift macos/OpenMathNative/Agent/*.swift \
  macos/OpenMathNativeTests/AgentProtocolFixtures/main.swift -o target/macos-agent-protocol-tests/fixtures
attempt_root=$(mktemp -d "$root/target/macos-agent-protocol-tests/attempt-XXXXXX")
target/macos-agent-protocol-tests/fixtures "$root" "$attempt_root" "$(command -v node)"
