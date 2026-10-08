#!/bin/bash
# Environment metadata only: no simulator, model request, key export, or package signing.
set -eu
python3 - "$@" <<'PY'
import argparse,json,subprocess,sys
parser=argparse.ArgumentParser(description="Read-only native Mac environment preflight")
parser.add_argument("--require-distribution-signing",action="store_true")
args=parser.parse_args()

def result(*cmd):
 p=subprocess.run(cmd,capture_output=True,text=True)
 return p.returncode,p.stdout.strip()

checks={}
for name,cmd in [('architecture',('uname','-m')),('os_version',('sw_vers','-productVersion')),('xcode',('xcodebuild','-version')),('macos_sdk',('xcrun','--sdk','macosx','--show-sdk-version')),('rust',('rustc','--version'))]:
 code,value=result(*cmd);checks[name]={'exit_code':code,'value':value}
code,identities=result('security','find-identity','-v','-p','codesigning')
checks['identity_counts']={'exit_code':code,'developer_id_application':identities.count('Developer ID Application:'),'apple_development':identities.count('Apple Development:')}
errors=[]
if checks['architecture']['value']!='arm64':errors.append('architecture_arm64_required')
try: supported_os=int(checks['os_version']['value'].split('.')[0])>=27
except ValueError: supported_os=False
if not supported_os:errors.append('macos_27_or_newer_required')
if not checks['xcode']['value'].startswith('Xcode 27.0\n'):errors.append('xcode_27_0_required')
if checks['macos_sdk']['value']!='27.0':errors.append('sdk_27_0_required')
if not checks['rust']['value'].startswith('rustc 1.94.0 '):errors.append('rust_1_94_0_required')
if args.require_distribution_signing and checks['identity_counts']['developer_id_application']==0:
 errors.append('developer_id_application_missing')
print(json.dumps({'kind':'development_environment_preflight','checks':checks,'errors':errors,'is_public_signing_evidence':False,'started_simulators':False},ensure_ascii=False,indent=2))
sys.exit(1 if errors else 0)
PY
