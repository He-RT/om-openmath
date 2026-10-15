#!/bin/bash
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$root"
mkdir -p target/macos-geometry-tests
CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 cargo build -p om-apple-ffi --release --locked
swiftc -whole-module-optimization -swift-version 6 -parse-as-library -target arm64-apple-macos27.0 -I macos/Headers \
 -L target/release -lom_apple_ffi macos/OpenMathNative/Generated/HostContractSupport.swift \
 macos/OpenMathNative/Generated/Contracts/*.swift macos/OpenMathNative/Storage/*.swift macos/OpenMathNative/Kernel/*.swift \
 macos/OpenMathNative/Rendering/ResultClient.swift macos/OpenMathNative/Rendering/ResultPresenter.swift \
 macos/OpenMathNative/Rendering/NativeGeometryData.swift macos/OpenMathNative/Rendering/NativeGeometryModel.swift macos/OpenMathNative/Rendering/NativeStepsModel.swift \
 macos/OpenMathNative/Rendering/NativePlotView.swift macos/OpenMathNative/Rendering/NativeMetalSceneView.swift \
 macos/OpenMathNative/NativeHostClient.swift macos/OpenMathNative/AppEventRouter.swift macos/OpenMathNative/DraftStore.swift \
 macos/OpenMathNativeTests/GeometryFixtures/main.swift -o target/macos-geometry-tests/geometry-fixtures
attempt_root=$(mktemp -d "$root/target/macos-geometry-tests/attempt-XXXXXX")
target/macos-geometry-tests/geometry-fixtures "$attempt_root"
