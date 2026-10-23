#!/bin/bash
set -euo pipefail
package_repo="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$package_repo"
: "${1:?candidate build root}"
: "${2:?new empty asset directory}"
package_build="$1"
package_assets="$2"
package_sha="$(git rev-parse HEAD)"
python3 scripts/release.py verify --tag v0.1.0-pre-alpha.5
[ -z "$(git status --porcelain)" ] || { echo 'Commit reviewed build inputs before packaging'; exit 1; }
[ ! -e "$package_assets" ] || { echo 'Output must be a new directory'; exit 1; }
for package_path in device/Build/Products/Release-iphoneos simulator/Build/Products/Release-iphonesimulator; do
  package_app="$package_build/$package_path/OpenMath.app"
  [ "$(/usr/libexec/PlistBuddy -c 'Print :OpenMathSourceRevision' "$package_app/Info.plist")" = "$package_sha" ] || { echo 'Candidate source revision mismatch'; exit 1; }
  [ "$(/usr/libexec/PlistBuddy -c 'Print :OpenMathReleaseVersion' "$package_app/Info.plist")" = '0.1.0-pre-alpha.5' ]
done
mkdir -p "$package_assets"
ditto -c -k --keepParent "$package_build/device/Build/Products/Release-iphoneos/OpenMath.app" "$package_assets/OpenMath_0.1.0-pre-alpha.5_ios_device_arm64_unsigned.app.zip"
ditto -c -k --keepParent "$package_build/simulator/Build/Products/Release-iphonesimulator/OpenMath.app" "$package_assets/OpenMath_0.1.0-pre-alpha.5_ios_simulator_arm64.app.zip"
ditto -c -k --keepParent ios/Frameworks/OpenMathKernel.xcframework "$package_assets/OpenMathKernel_0.1.0-pre-alpha.5.xcframework.zip"
python3 scripts/release.py finalize --scope ios --tag v0.1.0-pre-alpha.5 --sha "$package_sha" --root "$package_assets"
