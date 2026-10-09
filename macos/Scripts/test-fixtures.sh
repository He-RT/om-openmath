#!/bin/bash
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$root"
python3 macos/OpenMathNativeTests/Fixtures/check.py
node --test agent/test/fixtures.test.mjs
mkdir -p target/macos-native-tests
swiftc -swift-version 6 -parse-as-library -target arm64-apple-macos27.0 \
  macos/OpenMathNativeTests/MediaFixtures/main.swift -o target/macos-native-tests/media-fixtures
target/macos-native-tests/media-fixtures macos/OpenMathNativeTests/Fixtures/Media
