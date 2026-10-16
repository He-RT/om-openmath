#!/bin/bash
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$root"
python3 -I scripts/build_prompts.py --check
mkdir -p target/macos-prompt-tests
swiftc -swift-version 6 -warnings-as-errors -parse-as-library -target arm64-apple-macos27.0 \
  macos/OpenMathNative/Agent/SystemPromptText.swift macos/OpenMathNative/Agent/SystemPrompt.swift macos/OpenMathNativeTests/PromptFixtures/main.swift \
  -o target/macos-prompt-tests/fixtures
target/macos-prompt-tests/fixtures
