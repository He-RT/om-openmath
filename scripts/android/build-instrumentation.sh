#!/usr/bin/env bash
# A CI-only Android system Instrumentation APK; never bundled with the application.
set -euo pipefail
openmath_tools_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
openmath_test_root="$(mktemp -d "${RUNNER_TEMP}/openmath-instrumentation.XXXXXX")"
openmath_android_tools="${ANDROID_HOME}/build-tools/35.0.0"
umask 077
mkdir -p "$openmath_test_root/classes" "$openmath_test_root/dex" "$openmath_test_root/assets"
cp "$openmath_tools_root/app/scripts/package-smoke.js" "$openmath_test_root/assets/"
cat > "$openmath_test_root/AndroidManifest.xml" <<'XML'
<manifest xmlns:android="http://schemas.android.com/apk/res/android" package="org.openmath.openmath.test">
  <uses-sdk android:minSdkVersion="24" android:targetSdkVersion="35" />
  <application android:label="OpenMath package check" android:testOnly="true" />
  <instrumentation android:name="org.openmath.openmath.test.PackageSmokeInstrumentation" android:targetPackage="org.openmath.openmath" />
</manifest>
XML
javac -source 8 -target 8 -bootclasspath "$ANDROID_HOME/platforms/android-35/android.jar" -d "$openmath_test_root/classes" "$openmath_tools_root/scripts/android/PackageSmokeInstrumentation.java"
jar --create --file "$openmath_test_root/classes.jar" -C "$openmath_test_root/classes" .
"$openmath_android_tools/d8" --release --min-api 24 --lib "$ANDROID_HOME/platforms/android-35/android.jar" --output "$openmath_test_root/dex" "$openmath_test_root/classes.jar"
"$openmath_android_tools/aapt" package -f -M "$openmath_test_root/AndroidManifest.xml" -I "$ANDROID_HOME/platforms/android-35/android.jar" -A "$openmath_test_root/assets" -F "$openmath_test_root/unsigned.apk"
python3 - "$openmath_test_root" <<'PY'
import base64,os,sys,zipfile
from pathlib import Path
root=Path(sys.argv[1])
(root/'openmath-release.p12').write_bytes(base64.b64decode(os.environ['OPENMATH_ANDROID_KEYSTORE_BASE64']))
with zipfile.ZipFile(root/'unsigned.apk','a') as archive:
    archive.write(root/'dex/classes.dex','classes.dex')
PY
"$openmath_android_tools/zipalign" -f 4 "$openmath_test_root/unsigned.apk" "$openmath_test_root/aligned.apk"
"$openmath_android_tools/apksigner" sign --ks "$openmath_test_root/openmath-release.p12" --ks-key-alias openmath --ks-pass env:OPENMATH_ANDROID_KEY_PASSWORD --out "$RUNNER_TEMP/openmath-test.apk" "$openmath_test_root/aligned.apk"
