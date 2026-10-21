"""Apply the requested SDK 35 and one release-APK check to Tauri's generated Android project."""
from pathlib import Path
import argparse
import re
import shutil
import subprocess
import tempfile

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
app.write_text(text,encoding='utf-8')
# Tauri's library defaults to API 36; finalize every generated library DSL at API 35 too.
build=project/'build.gradle.kts'
text=build.read_text(encoding='utf-8')
text+='''
subprojects {
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
import android.content.res.Configuration
import android.view.View
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat
import androidx.core.view.WindowInsetsControllerCompat

class MainActivity : TauriActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val dark = (resources.configuration.uiMode and Configuration.UI_MODE_NIGHT_MASK) == Configuration.UI_MODE_NIGHT_YES
        WindowInsetsControllerCompat(window, window.decorView).apply {
            isAppearanceLightStatusBars = !dark
            isAppearanceLightNavigationBars = !dark
        }
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
with tempfile.TemporaryDirectory(prefix='openmath-mobile-icons-') as icon_dir:
    subprocess.run([
        'node',str(root/'app/node_modules/@tauri-apps/cli/tauri.js'),
        'icon',str(root/'app/src-tauri/icons/icon.png'),'--output',icon_dir,
    ],cwd=root/'app',check=True)
    shutil.copytree(Path(icon_dir)/'android',project/'app/src/main/res',dirs_exist_ok=True)
print('Generated app compileSdk=35 targetSdk=35; libraries compileSdk=35; system bars and IME insets configured.')
