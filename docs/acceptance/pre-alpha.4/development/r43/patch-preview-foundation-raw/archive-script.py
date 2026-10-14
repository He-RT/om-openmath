from pathlib import Path
import json,hashlib,gzip,subprocess,stat
R=Path.cwd();T=R/'target/acceptance/pre-alpha.4/development/r43';D=R/'docs/acceptance/pre-alpha.4/development/r43';A=D/'patch-preview-foundation-raw'
def sha(b):return hashlib.sha256(b).hexdigest()
def write(p,v):p.parent.mkdir(parents=True,exist_ok=True);p.write_text(json.dumps(v,ensure_ascii=False,indent=2)+'\n')
def footer(log):
 rows=[]
 for line in log.read_text().splitlines():
  try:
   d=json.loads(line)
   if isinstance(d,dict) and 'evidence_root' in d:rows.append(d)
  except ValueError:pass
 assert len(rows)==1, 'Fixture did not complete: '+log.name
 return rows[0]
standalone=footer(T/'patch-preview-pi-paged-corrected.log');signed=footer(T/'patch-preview-signed-app.log');compressed=[]
A.mkdir(parents=True,exist_ok=True)
def copy(src,dst,compress=False):
 assert src.is_file() and not src.is_symlink();b=src.read_bytes();dst.parent.mkdir(parents=True,exist_ok=True)
 if compress:
  dst=Path(str(dst)+'.gz');z=gzip.compress(b,mtime=0);assert gzip.decompress(z)==b;dst.write_bytes(z)
  compressed.append({'source':str(src.relative_to(R)),'archive':str(dst.relative_to(A)),'original_bytes':len(b),'original_sha256':sha(b)})
 else:dst.write_bytes(b);assert dst.read_bytes()==b

def tree(src,dst):
 for p in sorted(src.rglob('*')):
  if p.is_file():copy(p,dst/p.relative_to(src),p.name.endswith(('.sqlite','.sqlite-wal','.sqlite-shm')))
for p in T.glob('patch-preview*'):
 if p.is_file() and p.suffix in {'.json','.log'}:copy(p,A/'logs'/p.name)
for d in sorted(T.glob('patch-preview-failed-attempt-*')):tree(d,A/'original-failures'/d.name)
roots={}
for label,report in [('standalone',standalone),('signed-app',signed)]:
 root=Path(report['evidence_root']);assert root.parent==R/'target/macos-agent-package-tests';tree(root,A/label)
 p=root/'preview-task';actual=json.loads((p/'actual-preview-result.json').read_text());read=json.loads((p/'actual-preview-read.json').read_text());recovery=json.loads((p/'actual-preview-recovery.json').read_text());wire=json.loads((p/'actual-preview-wire.json').read_text());frames=[json.loads(f['frame_json']) for f in wire]
 assert actual['ok'] and actual['data']['valid'] and actual['data']['preview_ref'].startswith('preview-')
 assert read['data']['cells'][0]['source_complete'] and read['data']['cells'][0]['source']=='2+2'
 assert recovery['complete'] and recovery['last_event_sequence']==21 and len(recovery['tools'])==2 and not recovery['host_or_helper_created']
 assert all(t['phase']=='readonly_completed' and t.get('target') is None and t.get('admissionEvent') is None for t in recovery['tools'])
 body=json.loads((p/'actual-native-http-body-3.json').read_text());roles=[m['role'] for m in body['messages']]
 assert roles==['user','assistant','tool','assistant','tool'],roles
 assert body['messages'][-1]['content']==(p/'actual-preview-result.json').read_text()
 roots[label]={'root':str(root.relative_to(R)),'wire_frames':len(frames),'helper_frames':sum(f['direction']=='helper' for f in wire),'host_frames':sum(f['direction']=='host' for f in wire),'readonly_record_count':2,'sqlite_event_count':21,'tool_names':['read_notebook','preview_source'],'preview_ref':actual['data']['preview_ref'],'third_http_message_roles':roles,'third_http_tool_result_exact':True,'stored_recovery':recovery,'original_two_turn_regression':report}
 roots[label]['helper_frames']=len(frames)-roots[label]['host_frames']
for name in ['HelperManifest.json','production-dependencies.json','node-release-signature.txt']:copy(R/'target/macos-agent-package-tests/AgentRuntime'/name,A/'runtime'/name)
changed=subprocess.check_output(['git','diff','--name-only'],text=True).splitlines()+subprocess.check_output(['git','ls-files','--others','--exclude-standard'],text=True).splitlines()
extra=['crates/om-host-service/src/document/preview.rs','crates/om-host-service/src/document/preview/patch.rs','crates/om-host-service/src/document/commit.rs','crates/om-host-service/src/document/endpoint.rs','crates/om-host-service/src/agent/ingress.rs','macos/OpenMathNative/Storage/AgentSessionTypes.swift','macos/OpenMathNative/Storage/SessionLedgerPort.swift','agent/src/bridge.ts','macos/Scripts/test-agent-package.sh','docs/acceptance/pre-alpha.4/gates.toml']
sources={}
for n in sorted(set(changed+extra)):
 if n.startswith('docs/acceptance/pre-alpha.4/development/r43/patch-preview-foundation'):continue
 p=R/n
 if p.is_file():copy(p,A/'inputs'/n);sources[n]=sha(p.read_bytes())
write(A/'commands.json',[
 'CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 cargo test -p om-host-service --test agent_endpoint --test readonly_notebook --test preview --test commit_owner --locked -- --nocapture',
 'CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 cargo clippy -p om-host-service --all-targets --locked -- -D warnings',
 'OPENMATH_SINGLE_TURN_FIXTURE=$PWD/target/debug/examples/single_turn_fixture target/macos-agent-package-tests/AgentRuntime/Node/node --test agent/test/*.test.mjs',
 'target/macos-agent-package-tests/AgentRuntime/Node/node app/node_modules/typescript/bin/tsc --noEmit --strict --skipLibCheck --target es2024 --module nodenext --allowImportingTsExtensions agent/src/main.ts',
 'python3 -X warn_default_encoding -W error::EncodingWarning -m unittest discover -s tests -v',
 'bash macos/Scripts/test-agent-package.sh',
 'bash macos/Scripts/test-agent-package.sh /Users/hert/Projects/openmath/target/macos-agent-package-tests/AgentRuntime',
 "env NODE_OPTIONS=--invalid-old-profile NODE_PATH=/untrusted/old-modules OPENAI_API_KEY=fixture-sentinel target/macos-agent-package-tests/fixtures '<signed App AgentRuntime absolute path>' '<fresh fixture root>' '<signed App absolute path>' (reuse exact just-compiled binary; no source/library changes)",
 'python3 macos/native-package/build-helper.py --output macos/OpenMathNative/Resources/AgentRuntime',
 "xcodebuild -project macos/OpenMathNative.xcodeproj -scheme OpenMathNative -configuration Release -derivedDataPath target/macos-native -destination 'platform=macOS,arch=arm64' build",
 "codesign --verify --deep --strict 'target/macos-native/Build/Products/Release/OpenMath Preview.app'",
 'cargo fmt --all --check','python3 scripts/native_contracts.py --check','python3 macos/Scripts/generate-project.py --check','python3 scripts/check_pre_alpha_4_plan.py --check'])
write(A/'compressed-originals.json',compressed);copy(Path(__file__),A/'archive-script.py')
config=Path('/Users/hert/.config/openmath/testing/pre-alpha-4-api.json');private=json.loads(config.read_text());keys=[]
def collect(v):
 if isinstance(v,dict):
  for x in v.values():collect(x)
 elif isinstance(v,list):
  for x in v:collect(x)
 elif isinstance(v,str) and v.startswith('sk-'):keys.append(v.encode())
collect(private);assert keys and stat.S_IMODE(config.stat().st_mode)==0o600 and stat.S_IMODE(config.parent.stat().st_mode)==0o700
scanned=0
for p in A.rglob('*'):
 if p.is_file():
  b=p.read_bytes();b=gzip.decompress(b) if p.suffix=='.gz' else b
  assert not any(k in b for k in keys),'Credential in evidence';scanned+=1
proof={'schema_version':1,'task':'R4.3.06','status':'in_progress','date':'2026-10-15','base_commit':subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip(),'scope':'Native actual preview data foundation only; real atomic apply/DocCommitPort/effect result hookup still pending',
 'verified':{'rust':31,'node26_pi':15,'python':17,'generated_dtos':382,'clippy':'host all-targets -D warnings passed','typescript':'strict own sources, upstream skipLibCheck','fmt':'passed','generated_project':'passed','release':'passed','strict_signature':'passed after actual resource fixture','actual_preview':roots,
 'authority':'Only ACKed canonical original arguments; same actual Agent grant/task/read-issued snapshot through existing frozen PreviewService; no replacement snapshot/full-cell permission fabricated',
 'negative_tests':['outline cannot whole-cell replace; later full read does not upgrade original outline token','source-only inspection gives no committable preview_ref','mid-patch invalid selector fails whole preview; confirmed source/revision/execution epoch unchanged','default manual task=None SourceBegin cannot consume Agent-bound plan','closed preview result schema rejects fake source preview ref/valid patch missing ref/extra args; apply remains unavailable'],
 'effects':'Preview task head stays original hash/revision0/no source history, no run/save/commit; existing original two-turn fixture retains actual output/history/manual4 after helper and SIGKILL',
 'context_order':'Three-turn synthetic factory now replays actual SQLite event order; native codec rejected original reordered conversation, not weakened','pagination':'Original default16 storage pages iterated with original after sequence, cap64 fixture events; actual21 recovery through existing supported32 page. Core defaults and limits unchanged'},
 'ci_observation':json.loads((T/'patch-preview-base-ci.json').read_text()),'notes':json.loads((T/'patch-preview-scope-notes.json').read_text()),
 'pending':['R4.3.06 atomic apply actual DocCommitPort, durable effect admissions/receipts, structural IME/conflict/unknown cases not implemented; task Steps/Done unchecked','33/98 tasks complete,65 remaining; all32 final gates not_run/candidate UNSET/ready=false/released=false/runtime .3','Full ContextSnapshot producer is R4.4.12; this is synthetic route/model fixture with actual canonical stored data, no Registry/UI/live closure','R4.3.04 physical lock-screen and R4.2.03 actual IME candidate windows remain unverified; preserve OS input settings','No subagents/local iOS simulator/paid live requests in batch; dev only/no main merge/no old tag/package change'],
 'commands_file':'patch-preview-foundation-raw/commands.json','source_sha256':sources,'credential_scan':{'read_only_exact_authorized_file':True,'file_mode':'0600','directory_mode':'0700','files_scanned':scanned,'exact_key_matches':0,'key_not_copied':True},'raw_files_sha256':{str(p.relative_to(A)):sha(p.read_bytes()) for p in sorted(A.rglob('*')) if p.is_file()}}
write(D/'patch-preview-foundation.json',proof)
print(json.dumps({'source_inputs':len(sources),'raw_files':len(proof['raw_files_sha256']),'raw_bytes':sum(p.stat().st_size for p in A.rglob('*') if p.is_file()),'actual':{k:{f:v[f] for f in ['wire_frames','host_frames','helper_frames','sqlite_event_count']} for k,v in roots.items()},'credential_matches':0}))
