#!/bin/bash
# Host-native logic checks only; never starts an iOS simulator.
set -euo pipefail
cd "$(dirname "$0")/../.."
check_output=target/ios-pre-alpha.5
mkdir -p "$check_output"
node ios/Scripts/build-agent.mjs --check
node ios/Scripts/test-agent.mjs
xcrun swiftc -parse-as-library ios/OpenMath/MobileJSHost.swift ios/Scripts/test-agent-native.swift -o "$check_output/agent-native-fixture"
"$check_output/agent-native-fixture" ios/OpenMath/Resources/mobile-agent.js
xcrun swiftc -parse-as-library ios/OpenMath/JSONValue.swift ios/OpenMath/MobileAgentStore.swift ios/OpenMath/MobileNotebookPatch.swift ios/OpenMath/MobileSceneGeometry.swift ios/OpenMath/MobileRunPlanner.swift ios/Scripts/test-contracts-native.swift -o "$check_output/contracts-native-fixture"
"$check_output/contracts-native-fixture"
