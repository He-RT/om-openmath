#!/bin/bash
# Build the native Mac product and package only the current Mac/Web release scope.
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$root"
version=$(python3 -c 'import tomllib; print(tomllib.load(open("Cargo.toml","rb"))["workspace"]["package"]["version"])')
revision=$(git rev-parse HEAD)
python3 scripts/release.py verify --tag "v$version"
# A separately owned ignore rule does not affect program sources or packaged resources.
git diff --quiet HEAD -- . ':!.gitignore'
output=${1:?Usage: package-native.sh OUTPUT_DIRECTORY}
mkdir -p "$output"
output=$(cd "$output" && pwd)
for name in "OpenMath_${version}_aarch64.dmg" "OpenMath_${version}_macos_arm64.app.zip"; do
  [[ ! -e "$output/$name" ]] || { echo "Existing package: $name" >&2; exit 1; }
done
bash macos/Scripts/verify-env.sh
python3 macos/Scripts/generate-project.py --check
python3 macos/native-package/build-helper.py --output macos/OpenMathNative/Resources/AgentRuntime
cargo build -p om-apple-ffi -p om-cli --release --locked
xcodebuild -project macos/OpenMathNative.xcodeproj -scheme OpenMathNative -configuration Release \
  -destination 'platform=macOS,arch=arm64' -derivedDataPath target/macos-native -jobs 4 \
  CODE_SIGN_IDENTITY=- CODE_SIGN_STYLE=Manual SWIFT_COMPILATION_MODE=incremental \
  OPENMATH_SOURCE_REVISION="$revision" build
stage=$(mktemp -d "$root/target/macos-package-XXXXXX")
trap 'rm -rf "$stage"' EXIT
mkdir -p "$stage/volume"
bundle="$stage/volume/OpenMath Preview.app"
ditto 'target/macos-native/Build/Products/Release/OpenMath Preview.app' "$bundle"
mkdir -p "$bundle/Contents/Resources/Licenses"
cp LICENSE-MIT LICENSE-APACHE THIRD_PARTY_NOTICES.md "$bundle/Contents/Resources/Licenses/"
python3 - "$bundle" "$version" "$revision" <<'PY'
from pathlib import Path
import plistlib,sys
bundle=Path(sys.argv[1]); info=plistlib.loads((bundle/'Contents/Info.plist').read_bytes())
assert info['OpenMathReleaseVersion']==sys.argv[2] and info['OpenMathSourceRevision']==sys.argv[3]
for name in ['AgentRuntime/Node/node','AgentRuntime/Agent/main.mjs','Licenses/Node-LICENSE.txt','Licenses/SwiftMath-LICENSE.txt','Licenses/swift-markdown-LICENSE.txt','Licenses/swift-cmark-LICENSE.txt']:
 assert (bundle/'Contents/Resources'/name).is_file(), name
PY
# Re-sign only the outer resource seal; preserve the bundled Node executable's JIT entitlements.
codesign --force --sign - --timestamp=none --options runtime "$bundle"
codesign --verify --deep --strict --verbose=2 "$bundle"
ln -s /Applications "$stage/volume/Applications"
cp "docs/release-${version}.md" "$stage/volume/使用说明.md"
ditto -c -k --sequesterRsrc --keepParent "$bundle" "$output/OpenMath_${version}_macos_arm64.app.zip"
hdiutil create -volname "OpenMath ${version}" -srcfolder "$stage/volume" -format UDZO "$output/OpenMath_${version}_aarch64.dmg"
hdiutil verify "$output/OpenMath_${version}_aarch64.dmg"
echo "Native Mac packages: $output"
