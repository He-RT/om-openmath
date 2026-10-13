#!/bin/bash
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$root"
mkdir -p target/macos-secret-store-tests
swiftc -swift-version 6 -warnings-as-errors -parse-as-library -target arm64-apple-macos27.0 \
  macos/OpenMathNative/Models/SecretStore.swift macos/OpenMathNativeTests/SecretStoreFixtures/main.swift \
  -o target/macos-secret-store-tests/fixtures
target/macos-secret-store-tests/fixtures
