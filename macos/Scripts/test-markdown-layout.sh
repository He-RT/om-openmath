#!/bin/bash
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$root"
source macos/Scripts/native-math-dependencies.sh
prepare_native_math_dependencies
source macos/Scripts/native-markdown-dependencies.sh
prepare_native_markdown_dependencies
mkdir -p target/macos-markdown-tests
swiftc -whole-module-optimization -swift-version 6 -parse-as-library -target arm64-apple-macos27.0 \
 "${native_math_flags[@]}" "${native_markdown_flags[@]}" \
 macos/OpenMathNative/Generated/Contracts/MarkdownSpanMapping.swift \
 macos/OpenMathNative/UI/Theme.swift \
 macos/OpenMathNative/Rendering/MathRenderAdapter.swift macos/OpenMathNative/Rendering/NativeMathView.swift \
 macos/OpenMathNative/Rendering/MarkdownRenderDocument.swift macos/OpenMathNative/Rendering/NativeMarkdownView.swift macos/OpenMathNative/Rendering/NativeMarkdownTableView.swift \
 macos/OpenMathNative/Rendering/NativeCodeBlockView.swift \
 macos/OpenMathNativeTests/MarkdownLayoutFixtures/main.swift -o target/macos-markdown-tests/layout-fixtures
install_native_math_resources target/macos-markdown-tests
target/macos-markdown-tests/layout-fixtures "${1:-macos/OpenMathNativeTests/MarkdownLayoutFixtures/notebook.omnb}"
