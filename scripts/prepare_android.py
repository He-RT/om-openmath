"""Apply the requested SDK 35 and one release-APK check to Tauri's generated Android project."""
from pathlib import Path
import argparse
import re
import shutil

tooling=Path(__file__).resolve().parents[1]
parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('--root',type=Path,default=tooling)
root=parser.parse_args().root.resolve()
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
# API 35 enforces edge-to-edge. Keep system bars and the IME outside the Web workbench.
activities=list((project/'app/src/main').rglob('MainActivity.kt'))
assert len(activities)==1,'Expected one generated MainActivity'
activities[0].write_text('''package org.openmath.openmath

import android.os.Bundle
import android.view.View
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat

class MainActivity : TauriActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val content = findViewById<View>(android.R.id.content)
        ViewCompat.setOnApplyWindowInsetsListener(content) { view, insets ->
            val bars = insets.getInsets(WindowInsetsCompat.Type.systemBars() or WindowInsetsCompat.Type.displayCutout())
            val keyboard = insets.getInsets(WindowInsetsCompat.Type.ime())
            view.setPadding(bars.left, bars.top, bars.right, maxOf(bars.bottom, keyboard.bottom))
            WindowInsetsCompat.CONSUMED
        }
        ViewCompat.requestApplyInsets(content)
    }
}
''',encoding='utf-8')
tests=project/'app/src/androidTest/java/org/openmath/openmath'
tests.mkdir(parents=True,exist_ok=True)
shutil.copy2(tooling/'scripts/android/PackageSmokeTest.java',tests/'PackageSmokeTest.java')
assets=project/'app/src/androidTest/assets'
assets.mkdir(parents=True,exist_ok=True)
shutil.copy2(tooling/'app/scripts/package-smoke.js',assets/'package-smoke.js')
print('Generated app compileSdk=35 targetSdk=35; library compileSdk=35; release APK check configured.')
