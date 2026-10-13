#!/bin/bash
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$root"
bash macos/Scripts/verify-env.sh
python3 macos/Scripts/generate-project.py --check
python3 macos/native-package/build-helper.py --output macos/OpenMathNative/Resources/AgentRuntime
cargo build -p om-apple-ffi --release --locked
xcodebuild -project macos/OpenMathNative.xcodeproj -scheme OpenMathNative \
  -configuration Release -destination 'platform=macOS,arch=arm64' \
  -derivedDataPath target/macos-native CODE_SIGN_IDENTITY=- CODE_SIGN_STYLE=Manual build
