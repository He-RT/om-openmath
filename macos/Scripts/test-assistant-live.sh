#!/bin/bash
# Explicit opt-in: consumes the supplied API. Never called by default CI.
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$root"
if [[ $# -ne 2 ]]; then
  echo 'Usage: test-assistant-live.sh PRIVATE_API_JSON MODEL' >&2
  exit 2
fi
mkdir -p target/macos-live-assistant
swiftc -whole-module-optimization -swift-version 6 -parse-as-library -target arm64-apple-macos27.0 -I macos/Headers \
 -L target/release -lom_apple_ffi macos/OpenMathNative/Generated/HostContractSupport.swift \
 macos/OpenMathNative/Generated/Contracts/*.swift macos/OpenMathNative/Storage/*.swift macos/OpenMathNative/Kernel/*.swift \
 macos/OpenMathNative/Rendering/ResultClient.swift macos/OpenMathNative/Rendering/ResultPresenter.swift macos/OpenMathNative/Rendering/NativeGeometryData.swift macos/OpenMathNative/Rendering/NativeGeometryModel.swift macos/OpenMathNative/Rendering/NativeStepsModel.swift \
 macos/OpenMathNative/NativeHostClient.swift macos/OpenMathNative/AppEventRouter.swift macos/OpenMathNative/DraftStore.swift \
 macos/OpenMathNative/Agent/*.swift macos/OpenMathNative/Models/*.swift \
 macos/OpenMathNativeTests/LiveAssistantFixtures/main.swift -o target/macos-live-assistant/fixtures
attempt=$(mktemp -d "$root/target/macos-live-assistant/attempt-XXXXXX")
exec target/macos-live-assistant/fixtures "$1" "$root/target/macos-native/Build/Products/Release/OpenMath Preview.app" "$attempt" "$2"
