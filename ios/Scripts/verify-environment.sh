#!/bin/bash
set -euo pipefail
# Read the complete response before checking it: head can close the pipe while
# xcodebuild is still writing its build number and cause an NSFileHandle abort.
xcode_version=$(xcodebuild -version)
test "${xcode_version%%$'\n'*}" = 'Xcode 27.0'
printf '%s\n' "$xcode_version"
test "$(xcrun --sdk iphoneos --show-sdk-version)" = 27.0
test "$(xcrun --sdk iphonesimulator --show-sdk-version)" = 27.0
