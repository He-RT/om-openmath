#!/usr/bin/env python3
"""Build an immutable Node/ESM production closure. Developer tools never ship with it."""
from pathlib import Path
import argparse,hashlib,json,os,shutil,struct,subprocess,tarfile,tempfile,urllib.request
ROOT=Path(__file__).resolve().parents[2]
PIN=json.loads((ROOT/'macos/native-package/dependencies.json').read_text())
SIGNER='5BE8A3F6C8A5C01D106C0AD820B1A390B168D356'
def sha(path):
 h=hashlib.sha256()
 with path.open('rb') as f:
  while data:=f.read(1024*1024):h.update(data)
 return h.hexdigest()
def fetch(url,path):
 if path.exists():return
 path.parent.mkdir(parents=True,exist_ok=True)
 temporary=path.with_suffix(path.suffix+'.incoming')
 with urllib.request.urlopen(url,timeout=60) as incoming,temporary.open('wb') as out:shutil.copyfileobj(incoming,out)
 temporary.replace(path)
def run(argv,**kwargs):return subprocess.run(list(map(str,argv)),check=True,text=True,**kwargs)
def node_runtime(cache):
 node=PIN['node'];archive=cache/node['archive'];fetch(node['url'],archive)
 if sha(archive)!=node['archive_sha256']:raise ValueError('Node archive hash mismatch')
 signed=cache/'SHASUMS256.txt.asc';fetch(node['url'].rsplit('/',1)[0]+'/SHASUMS256.txt.asc',signed)
 keys=cache/'signature-verify';keys.mkdir(exist_ok=True,mode=0o700);os.chmod(keys,0o700)
 common=['gpg','--homedir',keys,'--no-options','--no-default-keyring','--keyring','verified-public.gpg','--batch']
 run(common+['--import',ROOT/'macos/native-package/keys/node-26.11.1-release.asc'],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
 proof=run(common+['--status-fd','1','--verify',signed],capture_output=True)
 if '[GNUPG:] VALIDSIG '+SIGNER+' ' not in proof.stdout:raise ValueError('Node release signer mismatch')
 expected=node['archive_sha256']+'  '+node['archive']
 if expected not in signed.read_text().splitlines():raise ValueError('Signed Node archive digest mismatch')
 extracted=cache/node['archive'].removesuffix('.tar.xz')
 if not extracted.exists():
  with tarfile.open(archive) as tar:tar.extractall(cache,filter='data')
 binary=extracted/'bin/node'
 if sha(binary)!=node['binary_sha256_before_resigning']:raise ValueError('Node executable digest mismatch')
 magic,cpu=struct.unpack('<II',binary.read_bytes()[:8])
 if magic!=0xfeedfacf or cpu!=0x100000c:raise ValueError('Node is not thin arm64 Mach-O')
 if run([binary,'--version'],capture_output=True).stdout.strip()!='v'+node['version']:raise ValueError('Node executable version mismatch')
 return extracted,proof.stdout

def main():
 parser=argparse.ArgumentParser();parser.add_argument('--output',type=Path,required=True);args=parser.parse_args()
 output=args.output.resolve();output.parent.mkdir(parents=True,exist_ok=True)
 if not output.is_relative_to(ROOT/'target') and output!=ROOT/'macos/OpenMathNative/Resources/AgentRuntime':raise ValueError('Unsupported generated output')
 source,signature=node_runtime(ROOT/'target/native-runtime')
 lock=json.loads((ROOT/'agent/package-lock.json').read_text())
 if hashlib.sha256((ROOT/'agent/package-lock.json').read_bytes()).hexdigest()!=PIN['pi']['production_lock_sha256']:raise ValueError('Production lock changed without audit')
 audit=run([source/'bin/node',ROOT/'agent/scripts/audit-dependencies.mjs'],capture_output=True)
 with tempfile.TemporaryDirectory(prefix='helper-build-',dir=ROOT/'target') as temporary:
  stage=Path(temporary)/'AgentRuntime';stage.mkdir();(stage/'Node').mkdir();(stage/'Agent/agent/src').mkdir(parents=True)
  shutil.copy2(source/'bin/node',stage/'Node/node');shutil.copy2(source/'LICENSE',stage/'Node/LICENSE');shutil.copy2(source/'README.md',stage/'Node/README.md')
  run([source/'bin/node',ROOT/'app/node_modules/typescript/bin/tsc','--strict','--skipLibCheck','--module','nodenext','--target','es2024','--rewriteRelativeImportExtensions','--types','node','--typeRoots',ROOT/'app/node_modules/@types','--rootDir',ROOT/'agent/src','--outDir',stage/'Agent/agent/src',ROOT/'agent/src/main.ts'])
  (stage/'Agent/agent/package.json').write_text(json.dumps({'type':'module','private':True})+'\n')
  (stage/'Agent/main.mjs').write_text("import './agent/src/main.js';\n")
  for path,metadata in lock['packages'].items():
   if not path or metadata.get('dev'):continue
   src=ROOT/'agent'/path;dest=stage/'Agent/agent'/path
   if not src.is_dir():raise ValueError('Missing installed production dependency')
   def ignore(folder,names):return [name for name in names if name.endswith(('.map','.ts','.tsx','.cts','.mts')) or name=='.bin' or name=='node_modules']
   shutil.copytree(src,dest,ignore=ignore,dirs_exist_ok=True)
  resources=stage/'Agent/docs/design';resources.mkdir(parents=True)
  for name in ['native-agent.schema.json','agent-tools.schema.json']:shutil.copy2(ROOT/'docs/design'/name,resources/name)
  notices=stage/'Licenses';shutil.copytree(ROOT/'macos/OpenMathNative/Resources/Licenses/Agent',notices)
  (stage/'production-dependencies.json').write_text(audit.stdout)
  (stage/'node-release-signature.txt').write_text(signature)
  entries=[]
  for file in sorted(stage.rglob('*')):
   if not file.is_file():continue
   if file.is_symlink():raise ValueError('Production symlink rejected')
   if file!=stage/'Node/node' and (file.suffix=='.node' or file.read_bytes()[:4] in [b'\xcf\xfa\xed\xfe',b'\xfe\xed\xfa\xcf',b'\xca\xfe\xba\xbe']):raise ValueError('Unreviewed production native component')
   entries.append({'path':file.relative_to(stage).as_posix(),'sha256':sha(file),'bytes':file.stat().st_size})
  manifest={'schema_version':1,'kind':'helper_production_closure','architecture':'arm64','node_version':PIN['node']['version'],'core_version':'1.0.4','ai_version':'1.0.4','adapter_version':'openmath-adapter-1','protocol_version':1,'schema_hash':sha(ROOT/'docs/design/native-agent.schema.json'),'node':'Node/node','entry':'Agent/main.mjs','production_packages':json.loads(audit.stdout)['production_packages'],'system_node_fallback':False,'node_release_signer':SIGNER,'files':entries}
  (stage/'HelperManifest.json').write_text(json.dumps(manifest,indent=2)+'\n')
  if output.exists():shutil.rmtree(output)
  shutil.copytree(stage,output)
 print(json.dumps({'output':str(output),'files':len(entries),'bytes':sum(x['bytes'] for x in entries),'production_packages':manifest['production_packages']}))
if __name__=='__main__':main()
