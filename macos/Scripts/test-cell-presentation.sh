#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/../.."
mkdir -p target/macos-cell-presentation-tests
swiftc -swift-version 6 -warnings-as-errors -parse-as-library macos/OpenMathNative/UI/CellPresentationState.swift macos/OpenMathNativeTests/CellPresentationFixtures/main.swift -o target/macos-cell-presentation-tests/fixtures
target/macos-cell-presentation-tests/fixtures
