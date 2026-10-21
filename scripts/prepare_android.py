"""Apply the requested SDK 35 and one release-APK check to Tauri's generated Android project."""
from pathlib import Path
import re
import shutil

root=Path(__file__).resolve().parents[1]
project=root/'app/src-tauri/gen/android'
app=project/'app/build.gradle.kts'
text=app.read_text(encoding='utf-8')
text,count=re.subn(r'compileSdk\s*=\s*\d+','compileSdk = 35',text)
assert count==1,'Expected one generated compileSdk'
text,count=re.subn(r'targetSdk\s*=\s*\d+','targetSdk = 35',text)
assert count==1,'Expected one generated targetSdk'
text=text.replace('android {','android {\n    testBuildType = "release"',1)
text=text.replace('defaultConfig {','defaultConfig {\n        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"',1)
text+='\ndependencies {\n    androidTestImplementation("androidx.test:runner:1.6.2")\n    androidTestImplementation("androidx.test.ext:junit:1.2.1")\n}\n'
app.write_text(text,encoding='utf-8')
# Tauri's library defaults to API 36; finalize every generated library DSL at API 35 too.
build=project/'build.gradle.kts'
text=build.read_text(encoding='utf-8')
text+='''
subprojects {
    tasks.configureEach {
        if (System.getenv("OPENMATH_ANDROID_TEST_ONLY") == "1" && name.startsWith("rustBuild")) enabled = false
    }
    pluginManager.withPlugin("com.android.library") {
        extensions.configure<com.android.build.api.variant.LibraryAndroidComponentsExtension>("androidComponents") {
            finalizeDsl { it.compileSdk = 35 }
        }
    }
}
'''
build.write_text(text,encoding='utf-8')
tests=project/'app/src/androidTest/java/org/openmath/openmath'
tests.mkdir(parents=True,exist_ok=True)
shutil.copy2(root/'scripts/android/PackageSmokeTest.java',tests/'PackageSmokeTest.java')
assets=project/'app/src/androidTest/assets'
assets.mkdir(parents=True,exist_ok=True)
shutil.copy2(root/'app/scripts/package-smoke.js',assets/'package-smoke.js')
print('Generated app compileSdk=35 targetSdk=35; library compileSdk=35; release APK check configured.')
