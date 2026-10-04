#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/../.."
: "${1:?传入 xcrun devicectl list devices 显示的真机 UDID}"
test -f ios/Local.xcconfig || { echo '请先在 ios/Local.xcconfig 设置 DEVELOPMENT_TEAM'; exit 1; }
bash ios/Scripts/verify-environment.sh
xcodebuild -project ios/OpenMath.xcodeproj -scheme OpenMath -configuration Release -destination "platform=iOS,id=$1" -derivedDataPath target/ios-device -allowProvisioningUpdates -allowProvisioningDeviceRegistration -disableAutomaticPackageResolution build
xcrun devicectl device install app --device "$1" target/ios-device/Build/Products/Release-iphoneos/OpenMath.app
xcrun devicectl device process launch --device "$1" org.openmath.OpenMath
