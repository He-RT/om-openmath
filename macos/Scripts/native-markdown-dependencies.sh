#!/bin/bash
# Reuse the exact Markdown/cmark products from the native app build, without a duplicate cache.
prepare_native_markdown_dependencies() {
  local markdown_root markdown_products markdown_package
  markdown_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
  markdown_products="$markdown_root/target/macos-native/Build/Products/Release"
  if [[ ! -f "$markdown_products/Markdown.o" ]]; then
    CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 bash "$markdown_root/macos/Scripts/build-native.sh" >&2
  fi
  python3 - "$markdown_root/macos/OpenMathNative.xcodeproj/project.xcworkspace/xcshareddata/swiftpm/Package.resolved" <<'PY'
import json,sys
pins={p['identity']:p['state']['revision'] for p in json.load(open(sys.argv[1]))['pins']}
assert pins['swift-markdown']=='25cb61d3482054b09ae76ca4f281b1bfe7fe5a43'
assert pins['swift-cmark']=='08ddb528923cc1a6527e02b7a1aee9e516ca749a'
PY
  native_markdown_flags=(-I "$markdown_products"
    -I "$markdown_root/target/macos-native/SourcePackages/checkouts/swift-markdown/Sources/CAtomic/include"
    -I "$markdown_root/target/macos-native/SourcePackages/checkouts/swift-cmark/src/include"
    -I "$markdown_root/target/macos-native/SourcePackages/checkouts/swift-cmark/extensions/include")
  for markdown_package in Markdown CAtomic cmark-gfm cmark-gfm-extensions; do
    [[ -f "$markdown_products/$markdown_package.o" ]] || { echo "Missing native Markdown product: $markdown_package" >&2; return 1; }
    native_markdown_flags+=("$markdown_products/$markdown_package.o")
  done
}
