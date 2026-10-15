#!/bin/bash
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$root"
export CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0
mkdir -p target/macos-agent-package-tests
if [[ $# -eq 0 ]]; then
 python3 macos/native-package/build-helper.py --output target/macos-agent-package-tests/AgentRuntime
 package="$root/target/macos-agent-package-tests/AgentRuntime"
else
 package="$1"
fi
cargo build -p om-apple-ffi --release --locked
swiftc -whole-module-optimization -swift-version 6 -parse-as-library -target arm64-apple-macos27.0 -I macos/Headers \
 -L target/release -lom_apple_ffi macos/OpenMathNative/Generated/HostContractSupport.swift \
 macos/OpenMathNative/Generated/Contracts/*.swift macos/OpenMathNative/Storage/*.swift macos/OpenMathNative/Kernel/*.swift \
 macos/OpenMathNative/Rendering/ResultClient.swift macos/OpenMathNative/Rendering/ResultPresenter.swift macos/OpenMathNative/Rendering/NativeGeometryData.swift macos/OpenMathNative/Rendering/NativeGeometryModel.swift macos/OpenMathNative/Rendering/NativeStepsModel.swift \
 macos/OpenMathNative/NativeHostClient.swift macos/OpenMathNative/AppEventRouter.swift macos/OpenMathNative/DraftStore.swift \
 macos/OpenMathNative/Agent/*.swift macos/OpenMathNative/Models/*.swift macos/OpenMathNativeTests/ProviderTransportFixtures/LoopbackServer.swift macos/OpenMathNativeTests/AgentPackageFixtures/main.swift -o target/macos-agent-package-tests/fixtures
attempt=$(mktemp -d "$root/target/macos-agent-package-tests/attempt-XXXXXX")
env NODE_OPTIONS='--invalid-old-profile' NODE_PATH='/untrusted/old-modules' OPENAI_API_KEY='fixture-sentinel' \
 target/macos-agent-package-tests/fixtures "$package" "$attempt" "${2:-}"
