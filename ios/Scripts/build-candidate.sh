#!/bin/bash
# Build only. No simulator creation, boot, installation, launch or tests.
set -euo pipefail
candidate_root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$candidate_root"
[ -z "$(git status --porcelain)" ] || { echo 'Commit reviewed build inputs before building a revision-stamped candidate'; exit 1; }
candidate_output="${OPENMATH_IOS_OUTPUT:-$candidate_root/target/ios-pre-alpha.5}"
mkdir -p "$candidate_output"
bash ios/Scripts/verify-environment.sh
node ios/Scripts/build-agent.mjs
python3 ios/Scripts/generate-project.py
bash ios/Scripts/build-kernel.sh
[ -z "$(git status --porcelain)" ] || { echo 'Generated inputs differ from the committed sources'; exit 1; }
candidate_revision="$(git rev-parse HEAD)"
for candidate_platform in device simulator; do
  if [ "$candidate_platform" = device ]; then
    candidate_destination='generic/platform=iOS'; candidate_action=build
  else
    candidate_destination='generic/platform=iOS Simulator'; candidate_action=build-for-testing
  fi
  xcodebuild -project ios/OpenMath.xcodeproj -scheme OpenMath -configuration Release \
    -destination "$candidate_destination" -derivedDataPath "$candidate_output/$candidate_platform" \
    -clonedSourcePackagesDirPath "$candidate_root/target/ios-device/SourcePackages" \
    -disableAutomaticPackageResolution ARCHS=arm64 ONLY_ACTIVE_ARCH=YES CODE_SIGNING_ALLOWED=NO OPENMATH_SOURCE_REVISION="$candidate_revision" "$candidate_action"
  if [ "$candidate_platform" = device ]; then candidate_product='Release-iphoneos'; else candidate_product='Release-iphonesimulator'; fi
  [ "$(/usr/libexec/PlistBuddy -c 'Print :OpenMathSourceRevision' "$candidate_output/$candidate_platform/Build/Products/$candidate_product/OpenMath.app/Info.plist")" = "$candidate_revision" ] || { echo 'Built source revision mismatch'; exit 1; }
done
printf 'Build-only candidates ready: %s\n' "$candidate_output"
