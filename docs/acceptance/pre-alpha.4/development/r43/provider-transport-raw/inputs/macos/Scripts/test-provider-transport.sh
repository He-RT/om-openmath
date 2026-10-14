#!/bin/bash
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$root"
export CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0
mkdir -p target/macos-provider-transport-tests
cargo build -p om-apple-ffi --release --locked
swiftc -whole-module-optimization -swift-version 6 -warnings-as-errors -parse-as-library -target arm64-apple-macos27.0 -I macos/Headers \
  -L target/release -lom_apple_ffi macos/OpenMathNative/Generated/HostContractSupport.swift \
  macos/OpenMathNative/Generated/Contracts/*.swift macos/OpenMathNative/NativeHostClient.swift \
  macos/OpenMathNative/AppEventRouter.swift macos/OpenMathNative/Models/*.swift \
  macos/OpenMathNative/Agent/AgentNativeModelPort.swift \
  macos/OpenMathNativeTests/ProviderTransportFixtures/main.swift -o target/macos-provider-transport-tests/fixtures
attempt=$(mktemp -d "$root/target/macos-provider-transport-tests/attempt-XXXXXX")
target/macos-provider-transport-tests/fixtures "$attempt"
printf 'Evidence: %s\n' "$attempt"
