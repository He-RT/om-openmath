"""Generate the deterministic, checked-in Xcode project without external project tools."""
from pathlib import Path
import hashlib,json
root=Path(__file__).resolve().parents[1]
objects={}
def oid(name): return hashlib.sha256(name.encode()).hexdigest()[:24].upper()
def add(name,body): key=oid(name);objects[key]=body;return key
def q(v):return json.dumps(v)
def array(values):return '('+','.join(values)+(',' if values else '')+')'
appFiles=[];testFiles=[];uiFiles=[]
for folder,output in [('OpenMath',appFiles),('OpenMathTests',testFiles),('OpenMathUITests',uiFiles)]:
 for p in sorted((root/folder).glob('*.swift')):
  ref=add(str(p.relative_to(root)),f'isa = PBXFileReference; lastKnownFileType = sourcecode.swift; path = {q(str(p.relative_to(root)))}; sourceTree = "<group>";')
  output.append((ref,add('build-'+str(p.relative_to(root)),f'isa = PBXBuildFile; fileRef = {ref};')))
resourceRefs=[];resourceBuilds=[]
for path,kind in [(p,'text.json' if p.suffix=='.json' else 'text') for p in sorted((root/'OpenMath/Resources').iterdir()) if p.is_file()]+[(root/'OpenMath/Assets.xcassets','folder.assetcatalog'),(root/'OpenMath/Resources/RustNotices','folder')]:
 ref=add(str(path.relative_to(root)),f'isa = PBXFileReference; lastKnownFileType = {kind}; path = {q(str(path.relative_to(root)))}; sourceTree = "<group>";')
 resourceRefs.append(ref);resourceBuilds.append(add('build-'+str(path.relative_to(root)),f'isa = PBXBuildFile; fileRef = {ref};'))
appProduct=add('app-product','isa = PBXFileReference; explicitFileType = wrapper.application; path = OpenMath.app; sourceTree = BUILT_PRODUCTS_DIR;')
unitProduct=add('unit-product','isa = PBXFileReference; explicitFileType = wrapper.cfbundle; path = OpenMathTests.xctest; sourceTree = BUILT_PRODUCTS_DIR;')
uiProduct=add('ui-product','isa = PBXFileReference; explicitFileType = wrapper.cfbundle; path = OpenMathUITests.xctest; sourceTree = BUILT_PRODUCTS_DIR;')
fw=add('ffi','isa = PBXFileReference; lastKnownFileType = wrapper.xcframework; path = Frameworks/OpenMathKernel.xcframework; sourceTree = "<group>";')
fwBuild=add('ffi-build',f'isa = PBXBuildFile; fileRef = {fw};')
packages=[];pkgProducts=[];pkgBuilds=[]
for name,url,version in [('SwiftMath','https://github.com/mgriebling/SwiftMath.git','1.7.3'),('Markdown','https://github.com/swiftlang/swift-markdown.git','0.9.0')]:
 ref=add('pkg-'+name,f'isa = XCRemoteSwiftPackageReference; repositoryURL = {q(url)}; requirement = {{kind = exactVersion; version = {version};}};');packages.append(ref)
 product=add('product-'+name,f'isa = XCSwiftPackageProductDependency; package = {ref}; productName = {name};');pkgProducts.append(product)
 pkgBuilds.append(add('buildpkg-'+name,f'isa = PBXBuildFile; productRef = {product};'))
productsGroup=add('products','isa = PBXGroup; children = '+array([appProduct,unitProduct,uiProduct])+'; name = Products; sourceTree = "<group>";')
mainGroup=add('main','isa = PBXGroup; children = '+array([ref for ref,_ in appFiles+testFiles+uiFiles]+resourceRefs+[fw,productsGroup])+'; sourceTree = "<group>";')
appTarget=oid('app');unitTarget=oid('unit');uiTarget=oid('ui');project=oid('project')
proxy=add('proxy',f'isa = PBXContainerItemProxy; containerPortal = {project}; proxyType = 1; remoteGlobalIDString = {appTarget}; remoteInfo = OpenMath;')
dep=add('dependency',f'isa = PBXTargetDependency; target = {appTarget}; targetProxy = {proxy};')
signing=add('signing-config','isa = PBXFileReference; lastKnownFileType = text.xcconfig; path = Config/Signing.xcconfig; sourceTree = "<group>";')
def configs(name,settings):
 ids=[]
 for config in ['Debug','Release']:
  vals=dict(settings)
  vals['SWIFT_OPTIMIZATION_LEVEL']='-Onone' if config=='Debug' else '-O'
  vals['DEBUG_INFORMATION_FORMAT']='dwarf' if config=='Debug' else 'dwarf-with-dsym'
  if config=='Debug': vals['SWIFT_ACTIVE_COMPILATION_CONDITIONS']='DEBUG $(inherited)'
  if config=='Release': vals['SWIFT_COMPILATION_MODE']='wholemodule'
  body='isa = XCBuildConfiguration; baseConfigurationReference = '+signing+'; buildSettings = {'+''.join(f'{k} = {q(v)};' for k,v in vals.items())+'}; name = '+config+';'
  ids.append(add(name+'-'+config,body))
 return add(name+'-configlist','isa = XCConfigurationList; buildConfigurations = '+array(ids)+'; defaultConfigurationIsVisible = 0; defaultConfigurationName = Release;')
base={'SUPPORTS_MAC_DESIGNED_FOR_IPHONE_IPAD':'NO','ARCHS':'arm64','SUPPORTED_PLATFORMS':'iphoneos iphonesimulator','SDKROOT':'iphoneos','IPHONEOS_DEPLOYMENT_TARGET':'27.0','SWIFT_VERSION':'6.0','SWIFT_STRICT_CONCURRENCY':'complete','TARGETED_DEVICE_FAMILY':'1,2','CLANG_ENABLE_MODULES':'YES','CODE_SIGN_STYLE':'Automatic','CURRENT_PROJECT_VERSION':'2','MARKETING_VERSION':'0.1.0','ENABLE_BITCODE':'NO','ENABLE_TESTABILITY':'YES','PRODUCT_NAME':'$(TARGET_NAME)','SWIFT_EMIT_LOC_STRINGS':'YES'}
projectConfigs=configs('project',{'CLANG_ENABLE_MODULES':'YES','IPHONEOS_DEPLOYMENT_TARGET':'27.0','SWIFT_VERSION':'6.0'})
for name,key,files,product,kind in [('OpenMath',appTarget,appFiles,appProduct,'com.apple.product-type.application'),('OpenMathTests',unitTarget,testFiles,unitProduct,'com.apple.product-type.bundle.unit-test'),('OpenMathUITests',uiTarget,uiFiles,uiProduct,'com.apple.product-type.bundle.ui-testing')]:
 src=add(name+'-sources','isa = PBXSourcesBuildPhase; buildActionMask = 2147483647; files = '+array([b for _,b in files])+'; runOnlyForDeploymentPostprocessing = 0;')
 framework=add(name+'-frameworks','isa = PBXFrameworksBuildPhase; buildActionMask = 2147483647; files = '+array([fwBuild]+pkgBuilds if name=='OpenMath' else [])+'; runOnlyForDeploymentPostprocessing = 0;')
 resources=add(name+'-resources','isa = PBXResourcesBuildPhase; buildActionMask = 2147483647; files = '+array(resourceBuilds if name=='OpenMath' else [])+'; runOnlyForDeploymentPostprocessing = 0;')
 settings=dict(base);settings['PRODUCT_BUNDLE_IDENTIFIER']='org.openmath.'+name
 settings['GENERATE_INFOPLIST_FILE']='YES'
 if name=='OpenMath':
  settings.update({'ASSETCATALOG_COMPILER_APPICON_NAME':'AppIcon','INFOPLIST_FILE':'OpenMath/Info.plist','GENERATE_INFOPLIST_FILE':'NO','INFOPLIST_KEY_UILaunchScreen_Generation':'YES'})
 elif name=='OpenMathTests':settings.update({'TEST_HOST':'$(BUILT_PRODUCTS_DIR)/OpenMath.app/$(BUNDLE_EXECUTABLE_FOLDER_PATH)/OpenMath','BUNDLE_LOADER':'$(TEST_HOST)'})
 else:settings['TEST_TARGET_NAME']='OpenMath'
 cfg=configs(name,settings)
 objects[key]=f'isa = PBXNativeTarget; buildConfigurationList = {cfg}; buildPhases = '+array([src,framework,resources])+'; buildRules = (); dependencies = '+array([] if name=='OpenMath' else [dep])+f'; name = {name}; productName = {name}; productReference = {product}; productType = {q(kind)}; packageProductDependencies = '+array(pkgProducts if name=='OpenMath' else [])+';'
objects[project]='isa = PBXProject; attributes = {BuildIndependentTargetsInParallel = YES; LastSwiftUpdateCheck = 2700; LastUpgradeCheck = 2700;}; buildConfigurationList = '+projectConfigs+'; compatibilityVersion = "Xcode 14.0"; developmentRegion = zh-Hans; hasScannedForEncodings = 0; knownRegions = (en,"zh-Hans",Base,); mainGroup = '+mainGroup+'; productRefGroup = '+productsGroup+'; projectDirPath = ""; projectRoot = ""; packageReferences = '+array(packages)+'; targets = '+array([appTarget,unitTarget,uiTarget])+';'
p=root/'OpenMath.xcodeproj';p.mkdir(exist_ok=True)
(p/'project.pbxproj').write_text('// !$*UTF8*$!\n{archiveVersion = 1; classes = {}; objectVersion = 56; objects = {\n'+''.join(f'{key} = {{{body}}};\n' for key,body in objects.items())+'}; rootObject = '+project+';}\n')
scheme=p/'xcshareddata/xcschemes';scheme.mkdir(parents=True,exist_ok=True)
def reference(key,name):return f'<BuildableReference BuildableIdentifier="primary" BlueprintIdentifier="{key}" BuildableName="{name}" BlueprintName="{name.split(".")[0]}" ReferencedContainer="container:OpenMath.xcodeproj"/>'
(scheme/'OpenMath.xcscheme').write_text('<?xml version="1.0" encoding="UTF-8"?><Scheme LastUpgradeVersion="2700" version="1.3"><BuildAction parallelizeBuildables="YES" buildImplicitDependencies="YES"><BuildActionEntries><BuildActionEntry buildForTesting="YES" buildForRunning="YES" buildForProfiling="YES" buildForArchiving="YES" buildForAnalyzing="YES">'+reference(appTarget,'OpenMath.app')+'</BuildActionEntry></BuildActionEntries></BuildAction><TestAction buildConfiguration="Debug" selectedDebuggerIdentifier="Xcode.DebuggerFoundation.Debugger.LLDB" selectedLauncherIdentifier="Xcode.IDEFoundation.Launcher.LLDB" shouldUseLaunchSchemeArgsEnv="YES"><Testables>'+''.join('<TestableReference skipped="NO">'+reference(k,n)+'</TestableReference>' for k,n in [(unitTarget,'OpenMathTests.xctest'),(uiTarget,'OpenMathUITests.xctest')])+'</Testables></TestAction><LaunchAction buildConfiguration="Debug" selectedDebuggerIdentifier="Xcode.DebuggerFoundation.Debugger.LLDB" selectedLauncherIdentifier="Xcode.IDEFoundation.Launcher.LLDB" launchStyle="0" useCustomWorkingDirectory="NO" ignoresPersistentStateOnLaunch="NO" debugDocumentVersioning="YES" debugServiceExtension="internal" allowLocationSimulation="YES"><BuildableProductRunnable runnableDebuggingMode="0">'+reference(appTarget,'OpenMath.app')+'</BuildableProductRunnable></LaunchAction><ProfileAction buildConfiguration="Release" shouldUseLaunchSchemeArgsEnv="YES" savedToolIdentifier="" useCustomWorkingDirectory="NO" debugDocumentVersioning="YES"><BuildableProductRunnable runnableDebuggingMode="0">'+reference(appTarget,'OpenMath.app')+'</BuildableProductRunnable></ProfileAction><AnalyzeAction buildConfiguration="Debug"/><ArchiveAction buildConfiguration="Release" revealArchiveInOrganizer="YES"/></Scheme>')
