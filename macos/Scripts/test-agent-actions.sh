#!/bin/bash
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$root"
mkdir -p target/macos-agent-action-tests
swiftc -swift-version 6 -warnings-as-errors -parse-as-library -target arm64-apple-macos27.0 \
  macos/OpenMathNative/Agent/TurnOutcome.swift macos/OpenMathNative/Agent/AssistantMessage.swift \
  macos/OpenMathNative/UI/AssistantTranscriptScrollState.swift \
  macos/OpenMathNativeTests/AgentActionFixtures/main.swift -o target/macos-agent-action-tests/fixtures
target/macos-agent-action-tests/fixtures
