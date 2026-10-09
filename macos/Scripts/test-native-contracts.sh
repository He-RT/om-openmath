#!/bin/bash
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$root"
bash macos/Scripts/verify-env.sh
python3 scripts/native_contracts.py --check
cargo build -p om-apple-ffi --release --locked
mkdir -p target/macos-native-tests
sources=(macos/OpenMathNative/Generated/HostContractSupport.swift)
for file in macos/OpenMathNative/Generated/Contracts/*.swift; do sources+=("$file"); done
swiftc -swift-version 6 -target arm64-apple-macos27.0 -I macos/Headers \
  -L target/release -lom_apple_ffi "${sources[@]}" \
  macos/OpenMathNativeTests/ABIFixtures/main.swift -o target/macos-native-tests/abi-fixtures
target/macos-native-tests/abi-fixtures
swiftc -swift-version 6 -target arm64-apple-macos27.0 "${sources[@]}" \
  macos/OpenMathNativeTests/ContractFixtures/main.swift -o target/macos-native-tests/contract-fixtures
target/macos-native-tests/contract-fixtures

swiftc -swift-version 6 -parse-as-library -target arm64-apple-macos27.0 -I macos/Headers \
  -L target/release -lom_apple_ffi "${sources[@]}" macos/OpenMathNative/NativeHostClient.swift \
  macos/OpenMathNativeTests/ClientFixtures/main.swift -o target/macos-native-tests/client-fixtures
target/macos-native-tests/client-fixtures
