#!/bin/bash
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$root"
mkdir -p target/macos-turn-outcome-tests
swiftc -swift-version 6 -warnings-as-errors -parse-as-library -target arm64-apple-macos27.0 \
  macos/OpenMathNative/Agent/TurnOutcome.swift macos/OpenMathNative/Agent/UIStrings.swift \
  macos/OpenMathNativeTests/TurnOutcomeFixtures/main.swift -o target/macos-turn-outcome-tests/fixtures
target/macos-turn-outcome-tests/fixtures
