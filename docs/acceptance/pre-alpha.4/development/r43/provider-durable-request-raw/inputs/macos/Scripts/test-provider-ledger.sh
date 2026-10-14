#!/bin/bash
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$root"
export CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0
mkdir -p target/macos-provider-ledger-tests
cargo build -p om-apple-ffi --release --locked
swiftc -whole-module-optimization -swift-version 6 -warnings-as-errors -parse-as-library -target arm64-apple-macos27.0 -I macos/Headers \
  -L target/release -lom_apple_ffi macos/OpenMathNative/Generated/HostContractSupport.swift \
  macos/OpenMathNative/Generated/Contracts/*.swift macos/OpenMathNative/Storage/*.swift \
  macos/OpenMathNative/NativeHostClient.swift macos/OpenMathNative/AppEventRouter.swift macos/OpenMathNative/DraftStore.swift \
  macos/OpenMathNative/Agent/AgentNativeModelPort.swift macos/OpenMathNative/Models/*.swift \
  macos/OpenMathNativeTests/ProviderTransportFixtures/LoopbackServer.swift macos/OpenMathNativeTests/ProviderLedgerFixtures/main.swift \
  -o target/macos-provider-ledger-tests/fixtures
attempt=$(mktemp -d "$root/target/macos-provider-ledger-tests/attempt-XXXXXX")
mkdir "$attempt/complete"
target/macos-provider-ledger-tests/fixtures "$attempt/complete" run
target/macos-provider-ledger-tests/fixtures "$attempt/complete" inspect finished
mkdir "$attempt/echo" "$attempt/timeout"
target/macos-provider-ledger-tests/fixtures "$attempt/echo" echo
target/macos-provider-ledger-tests/fixtures "$attempt/timeout" timeout
python3 macos/OpenMathNativeTests/ProviderLedgerFixtures/interruption.py "$root/target/macos-provider-ledger-tests/fixtures" "$attempt"
printf 'Evidence: %s\n' "$attempt"
