#!/bin/bash
set -euo pipefail
preview_source_dir="$(cd "$(dirname "$0")" && pwd)"
preview_repo_root="$(cd "$preview_source_dir/../../../.." && pwd)"
preview_output="$preview_repo_root/target/ios27-glass-study"
preview_bundle="$preview_output/OpenMath Glass Study.app"
preview_cache="$preview_output/ModuleCache"
mkdir -p "$preview_bundle/Contents/MacOS" "$preview_cache"
trap 'rm -rf "$preview_cache"' EXIT
xcrun swiftc -parse-as-library -O \
  -target arm64-apple-macosx27.0 \
  -sdk "$(xcrun --sdk macosx --show-sdk-path)" \
  -module-cache-path "$preview_cache" \
  "$preview_source_dir/GlassWorkbench.swift" \
  -o "$preview_bundle/Contents/MacOS/GlassWorkbench"
cat > "$preview_bundle/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleIdentifier</key><string>org.openmath.design.ios27glass</string>
<key>CFBundleName</key><string>OpenMath Glass Study</string>
<key>CFBundleDisplayName</key><string>OpenMath Glass Study</string>
<key>CFBundleExecutable</key><string>GlassWorkbench</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>CFBundleShortVersionString</key><string>0.2</string>
<key>LSMinimumSystemVersion</key><string>27.0</string>
<key>NSHighResolutionCapable</key><true/>
</dict></plist>
PLIST
codesign --force --sign - "$preview_bundle"
printf '%s\n' "$preview_bundle"
