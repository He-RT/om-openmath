#!/bin/bash
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$root"
export DEVELOPER_DIR="${DEVELOPER_DIR:-/Applications/Xcode.app/Contents/Developer}"
ios/Scripts/verify-environment.sh
export IPHONEOS_DEPLOYMENT_TARGET=27.0
rustup target add --toolchain 1.94.0 aarch64-apple-ios aarch64-apple-ios-sim
cargo build -p om-ios-ffi --release --locked --target aarch64-apple-ios
cargo build -p om-ios-ffi --release --locked --target aarch64-apple-ios-sim
rm -rf ios/Frameworks/OpenMathKernel.xcframework
xcodebuild -create-xcframework -library target/aarch64-apple-ios/release/libom_ios_ffi.a -headers ios/Headers -library target/aarch64-apple-ios-sim/release/libom_ios_ffi.a -headers ios/Headers -output ios/Frameworks/OpenMathKernel.xcframework

python3 ios/Scripts/package-notices.py
