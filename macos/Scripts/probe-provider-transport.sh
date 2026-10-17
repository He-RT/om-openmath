#!/bin/bash
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$root"
test $# -eq 1
export CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0
mkdir -p target/macos-provider-live-tests
cargo build -p om-apple-ffi --release --locked
swiftc -whole-module-optimization -swift-version 6 -warnings-as-errors -parse-as-library -target arm64-apple-macos27.0 -I macos/Headers \
  -L target/release -lom_apple_ffi macos/OpenMathNative/Generated/HostContractSupport.swift \
  macos/OpenMathNative/Generated/Contracts/*.swift macos/OpenMathNative/NativeHostClient.swift macos/OpenMathNative/Kernel/NotebookAnalysis.swift macos/OpenMathNative/Kernel/NotebookDefinitions.swift \
  macos/OpenMathNative/AppEventRouter.swift macos/OpenMathNative/Models/SecretStore.swift \
  macos/OpenMathNative/Models/ProviderTransport.swift macos/OpenMathNative/Models/ProviderAgentModelAdapter.swift \
  macos/OpenMathNative/Agent/AgentNativeModelPort.swift macos/OpenMathNativeTests/ProviderLiveFixtures/main.swift \
  -o target/macos-provider-live-tests/fixtures
target/macos-provider-live-tests/fixtures "$1"
