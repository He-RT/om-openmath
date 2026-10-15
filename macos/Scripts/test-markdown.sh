#!/bin/bash
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$root"
source macos/Scripts/native-markdown-dependencies.sh
prepare_native_markdown_dependencies
mkdir -p target/macos-markdown-tests
swiftc -swift-version 6 -parse-as-library -target arm64-apple-macos27.0 "${native_markdown_flags[@]}" \
 macos/OpenMathNative/Generated/Contracts/MarkdownSpanMapping.swift macos/OpenMathNative/Rendering/MarkdownRenderDocument.swift \
 macos/OpenMathNativeTests/MarkdownFixtures/main.swift -o target/macos-markdown-tests/parser-fixtures
target/macos-markdown-tests/parser-fixtures
