"""Generate the native Mac Xcode project deterministically; --check does not mutate files."""
from pathlib import Path
import argparse
import hashlib
import json
import sys

ROOT = Path(__file__).resolve().parents[1]
objects = {}
def oid(name):
    return hashlib.sha256(('native-mac:' + name).encode()).hexdigest()[:24].upper()
def add(name, body):
    key = oid(name)
    objects[key] = body
    return key

def q(value):
    return json.dumps(value)

def array(values):
    return '(' + ','.join(values) + (',' if values else '') + ')'

sources = []
for path in sorted((ROOT / 'OpenMathNative').rglob('*.swift')):
    relative = path.relative_to(ROOT).as_posix()
    ref = add(relative, f'isa = PBXFileReference; lastKnownFileType = sourcecode.swift; path = {q(relative)}; sourceTree = "<group>";')
    sources.append((ref, add('build:' + relative, f'isa = PBXBuildFile; fileRef = {ref};')))
resources = add('licenses', 'isa = PBXFileReference; lastKnownFileType = folder; path = OpenMathNative/Resources/Licenses; sourceTree = "<group>";')
resource_build = add('licenses-build', f'isa = PBXBuildFile; fileRef = {resources};')
product = add('app-product', 'isa = PBXFileReference; explicitFileType = wrapper.application; path = "OpenMath Preview.app"; sourceTree = BUILT_PRODUCTS_DIR;')
products = add('products', f'isa = PBXGroup; children = {array([product])}; name = Products; sourceTree = "<group>";')
main_group = add('main-group', f'isa = PBXGroup; children = {array([ref for ref, _ in sources] + [resources, products])}; sourceTree = "<group>";')
source_phase = add('source-phase', f'isa = PBXSourcesBuildPhase; buildActionMask = 2147483647; files = {array([build for _, build in sources])}; runOnlyForDeploymentPostprocessing = 0;')
resource_phase = add('resource-phase', f'isa = PBXResourcesBuildPhase; buildActionMask = 2147483647; files = {array([resource_build])}; runOnlyForDeploymentPostprocessing = 0;')
framework_phase = add('framework-phase', 'isa = PBXFrameworksBuildPhase; buildActionMask = 2147483647; files = (); runOnlyForDeploymentPostprocessing = 0;')
settings = {
    'SDKROOT': 'macosx', 'SUPPORTED_PLATFORMS': 'macosx', 'ARCHS': 'arm64',
    'MACOSX_DEPLOYMENT_TARGET': '27.0', 'SWIFT_VERSION': '6.0',
    'SWIFT_STRICT_CONCURRENCY': 'complete', 'CLANG_ENABLE_MODULES': 'YES',
    'ENABLE_HARDENED_RUNTIME': 'YES', 'CODE_SIGN_STYLE': 'Manual', 'CODE_SIGN_IDENTITY': '-',
    'PRODUCT_NAME': 'OpenMath Preview', 'EXECUTABLE_NAME': 'OpenMathNative',
    'PRODUCT_BUNDLE_IDENTIFIER': 'org.openmath.OpenMath.NativeMacPreview',
    'GENERATE_INFOPLIST_FILE': 'NO', 'INFOPLIST_FILE': 'OpenMathNative/Info.plist',
    'SWIFT_INCLUDE_PATHS': '$(inherited) $(PROJECT_DIR)/Headers',
    'LIBRARY_SEARCH_PATHS': '$(inherited) $(PROJECT_DIR)/../target/release',
    'OTHER_LDFLAGS': '$(inherited) -lom_apple_ffi -lsqlite3', 'ENABLE_TESTABILITY': 'YES',
}
config_ids = []
for name in ['Debug', 'Release']:
    values = dict(settings)
    values['SWIFT_OPTIMIZATION_LEVEL'] = '-Onone' if name == 'Debug' else '-O'
    values['DEBUG_INFORMATION_FORMAT'] = 'dwarf' if name == 'Debug' else 'dwarf-with-dsym'
    if name == 'Release':
        values['SWIFT_COMPILATION_MODE'] = 'wholemodule'
    config_ids.append(add('config:' + name, 'isa = XCBuildConfiguration; buildSettings = {' + ''.join(f'{k} = {q(v)};' for k, v in values.items()) + f'}}; name = {name};'))
configs = add('config-list', f'isa = XCConfigurationList; buildConfigurations = {array(config_ids)}; defaultConfigurationIsVisible = 0; defaultConfigurationName = Release;')
target = add('target', f'isa = PBXNativeTarget; buildConfigurationList = {configs}; buildPhases = {array([source_phase, framework_phase, resource_phase])}; buildRules = (); dependencies = (); name = OpenMathNative; productName = "OpenMath Preview"; productReference = {product}; productType = "com.apple.product-type.application";')
project = add('project', f'isa = PBXProject; attributes = {{LastUpgradeCheck = 2700; LastSwiftUpdateCheck = 2700;}}; buildConfigurationList = {configs}; compatibilityVersion = "Xcode 14.0"; developmentRegion = "zh-Hans"; knownRegions = ("zh-Hans",en,Base,); mainGroup = {main_group}; productRefGroup = {products}; projectDirPath = ""; projectRoot = ""; targets = {array([target])};')
pbx = '// !$*UTF8*$!\n{archiveVersion = 1; classes = {}; objectVersion = 56; objects = {\n' + ''.join(f'{key} = {{{body}}};\n' for key, body in sorted(objects.items())) + f'}}; rootObject = {project};}}\n'
reference = f'<BuildableReference BuildableIdentifier="primary" BlueprintIdentifier="{target}" BuildableName="OpenMath Preview.app" BlueprintName="OpenMathNative" ReferencedContainer="container:OpenMathNative.xcodeproj"/>'
scheme = '<?xml version="1.0" encoding="UTF-8"?><Scheme LastUpgradeVersion="2700" version="1.3"><BuildAction parallelizeBuildables="YES" buildImplicitDependencies="YES"><BuildActionEntries><BuildActionEntry buildForTesting="YES" buildForRunning="YES" buildForProfiling="YES" buildForArchiving="YES" buildForAnalyzing="YES">' + reference + '</BuildActionEntry></BuildActionEntries></BuildAction><LaunchAction buildConfiguration="Debug" selectedDebuggerIdentifier="Xcode.DebuggerFoundation.Debugger.LLDB" selectedLauncherIdentifier="Xcode.IDEFoundation.Launcher.LLDB" launchStyle="0" useCustomWorkingDirectory="NO" ignoresPersistentStateOnLaunch="NO" debugDocumentVersioning="YES"><BuildableProductRunnable runnableDebuggingMode="0">' + reference + '</BuildableProductRunnable></LaunchAction><ProfileAction buildConfiguration="Release" shouldUseLaunchSchemeArgsEnv="YES" savedToolIdentifier="" useCustomWorkingDirectory="NO" debugDocumentVersioning="YES"><BuildableProductRunnable runnableDebuggingMode="0">' + reference + '</BuildableProductRunnable></ProfileAction><AnalyzeAction buildConfiguration="Debug"/><ArchiveAction buildConfiguration="Release" revealArchiveInOrganizer="YES"/></Scheme>\n'
outputs = {'OpenMathNative.xcodeproj/project.pbxproj': pbx, 'OpenMathNative.xcodeproj/xcshareddata/xcschemes/OpenMathNative.xcscheme': scheme}
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--check', action='store_true')
args = parser.parse_args()
for relative, text in outputs.items():
    path = ROOT / relative
    if args.check:
        if not path.exists() or path.read_text(encoding='utf-8') != text:
            sys.exit(f'Generated project drift: {relative}')
    else:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding='utf-8')
print('Native Mac project verified' if args.check else 'Native Mac project generated')
