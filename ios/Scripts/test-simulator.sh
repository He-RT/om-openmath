#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/../.."
: "${1:?phone 或 pad}"
case "$1" in phone|pad) ;; *) echo '仅支持 phone 或 pad' >&2; exit 64 ;; esac
bash ios/Scripts/verify-environment.sh
sim_existing=$(xcrun simctl list devices -j | python3 -c 'import json,sys; print(sum(len(rows) for rows in json.load(sys.stdin)["devices"].values()))')
[ "$sim_existing" = 0 ] || { echo 'A simulator already exists, including shutdown devices; finish and delete it before creating another.' >&2; exit 1; }
sim_name="OpenMath-$1-acceptance"
if [ "$1" = phone ]; then model_name='iPhone 18 Pro'; else model_name='iPad Pro 11-inch (M5)'; fi
sim_runtime=$(xcrun simctl list runtimes -j | python3 -c 'import json,sys; print(next(r["identifier"] for r in json.load(sys.stdin)["runtimes"] if r["version"]=="27.0" and r["isAvailable"] and "iOS" in r["name"]))')
sim_type=$(xcrun simctl list devicetypes -j | python3 -c 'import json,sys; print(next(r["identifier"] for r in json.load(sys.stdin)["devicetypes"] if r["name"]==sys.argv[1]))' "$model_name")
sim_id=$(xcrun simctl create "$sim_name" "$sim_type" "$sim_runtime")
trap 'xcrun simctl shutdown "$sim_id" >/dev/null 2>&1 || true; xcrun simctl delete "$sim_id"' EXIT
xcrun simctl boot "$sim_id"
xcrun simctl bootstatus "$sim_id" -b
result_path="target/ios-$1-$(date -u +%Y%m%dT%H%M%SZ)-$$.xcresult"
printf '验收结果：%s\n' "$result_path"
sim_build="${OPENMATH_IOS_SIMULATOR_BUILD:-target/ios-derived}"
sim_action=test
if [ "${OPENMATH_IOS_TEST_WITHOUT_BUILDING:-0}" = 1 ]; then sim_action=test-without-building; fi
xcodebuild -project ios/OpenMath.xcodeproj -scheme OpenMath -configuration Release \
  -destination "platform=iOS Simulator,id=$sim_id" -derivedDataPath "$sim_build" \
  -clonedSourcePackagesDirPath target/ios-device/SourcePackages \
  -resultBundlePath "$result_path" -parallel-testing-enabled NO -collect-test-diagnostics never \
  -maximum-concurrent-test-simulator-destinations 1 \
  -disableAutomaticPackageResolution ARCHS=arm64 ONLY_ACTIVE_ARCH=YES CODE_SIGNING_ALLOWED=YES CODE_SIGN_IDENTITY=- "$sim_action"
