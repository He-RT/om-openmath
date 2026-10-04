#!/bin/bash
set -euo pipefail
xcodebuild -version | head -1 | grep -x 'Xcode 27.0'
test "$(xcrun --sdk iphoneos --show-sdk-version)" = 27.0
test "$(xcrun --sdk iphonesimulator --show-sdk-version)" = 27.0
