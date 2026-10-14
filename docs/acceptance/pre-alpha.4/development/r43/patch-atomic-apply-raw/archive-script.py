from pathlib import Path
import json,hashlib,gzip,subprocess,re,stat
R=Path.cwd();T=R/'target/acceptance/pre-alpha.4/development/r43';D=R/'docs/acceptance/pre-alpha.4/development/r43';A=D/'patch-atomic-apply-raw'
def sha(b):return hashlib.sha256(b).hexdigest()
def write(p,v):p.parent.mkdir(parents=True,exist_ok=True);p.write_text(json.dumps(v,ensure_ascii=False,indent=2)+'\n')
def completed(name):
 rows=[]
 for line in (T/name).read_text().splitlines():
  try:
   d=json.loads(line)
   if isinstance(d,dict) and 'evidence_root' in d:rows.append(d)
  except ValueError:pass
 assert len(rows)==1,'Fixture not completed: '+name
 return rows[0]
standalone=completed('patch-apply-pi-negative-ledger.log');signed=completed('patch-apply-signed-app.log')
suites=re.findall(r'test result: ok\. (\d+) passed; (\d+) failed',(T/'patch-apply-workspace-tests.log').read_text());passed=sum(int(a) for a,b in suites);failed=sum(int(b) for a,b in suites);assert passed==1301 and failed==0,(passed,failed)
assert 'BUILD SUCCEEDED' in (T/'patch-apply-release.log').read_text()
ledger=(T/'patch-apply-session-ledger.log').read_text();assert '13 actual SIGKILL' in ledger and '4374 events' in ledger and '60 actual canonical session events' in ledger
compressed=[];A.mkdir(parents=True,exist_ok=True)
def copy(src,dst,compress=False):
 assert src.is_file() and not src.is_symlink();b=src.read_bytes();dst.parent.mkdir(parents=True,exist_ok=True)
 if compress:
  dst=Path(str(dst)+'.gz');z=gzip.compress(b,mtime=0);assert gzip.decompress(z)==b;dst.write_bytes(z)
  compressed.append({'source':str(src.relative_to(R)),'archive':str(dst.relative_to(A)),'original_bytes':len(b),'original_sha256':sha(b)})
 else:dst.write_bytes(b);assert dst.read_bytes()==b

def tree(src,dst):
 for p in sorted(src.rglob('*')):
  if p.is_file():copy(p,dst/p.relative_to(src),p.name.endswith(('.sqlite','.sqlite-wal','.sqlite-shm')))
for p in sorted(T.glob('patch-apply*')):
 if p.is_file() and p.suffix in {'.json','.log','.jsonl'}:copy(p,A/'logs'/p.name,p.suffix=='.jsonl')
for p in sorted(T.glob('patch-apply-failed-attempt-*')):tree(p,A/'original-failures'/p.name)
facts={}
for label,report in [('standalone',standalone),('signed-app',signed)]:
 root=Path(report['evidence_root']);assert root.parent==R/'target/macos-agent-package-tests';tree(root,A/label)
 p=root/'apply-task';body=json.loads((p/'actual-apply-result.json').read_text());head=json.loads((p/'actual-apply-head.json').read_text());receipt=json.loads((p/'actual-apply-receipt.json').read_text());wire=json.loads((p/'actual-apply-wire.json').read_text());events=json.loads((p/'actual-apply-ledger.json').read_text());recovery=json.loads((p/'actual-apply-recovery.json').read_text());bounds=json.loads((root/'apply-boundaries/actual-boundaries.json').read_text())
 assert len(wire)==38 and sum(f['direction']=='helper' for f in wire)==17 and len(events)==31
 assert body['ok'] and body['data']['saved'] is False and head['revision']==1 and head['file']['cells'][0]['source']=='3+3'
 assert body['operation_ref']==receipt['receipt']['operation_id'] and body['data']['transaction_ref']==receipt['receipt']['transaction_id']
 assert recovery['complete'] and not recovery['host_or_helper_created'] and recovery['source_committed_not_run'] and recovery['last_event_sequence']==31
 roles=[m['role'] for m in json.loads((p/'actual-native-http-body-4.json').read_text())['messages']];assert roles==['user','assistant','tool','assistant','tool','assistant','tool']
 tool=json.loads((p/'actual-native-http-body-4.json').read_text())['messages'][-1];assert tool['content']==(p/'actual-apply-result.json').read_text()
 assert len(bounds)==5 and all(x['no_kernel_checkpoint'] and x['storage_only_recovery'] and x['native_wrong_result_hash_rejected'] for x in bounds)
 assert {b['mode']:b['recovered_phase'] for b in bounds}=={'duplicate':'completed','marked':'failed','source_changed':'failed','lost_source_ack':'completed','cancel_before_commit':'cancelled'}
 assert next(b for b in bounds if b['mode']=='duplicate')['missing_blob_changed_args_absent_actual_effect_rejected']
 facts[label]={'root':str(root.relative_to(R)),'frames':38,'host_frames':21,'helper_frames':17,'sqlite_events':31,'tools':['read_notebook','preview_source','apply_notebook_patch'],'http_turns':4,'fourth_http_roles':roles,'fourth_body_tool_result_exact':True,'result':body,'source_head':head,'original_receipt':receipt,'storage_only_recovery':recovery,'boundaries':bounds,'original_two_turn_regression':report,'preview_three_turn':'same fixture retains29 frames/21 events, no commit/run/save'}
for n in ['session-ledger-canonical-events.json','session-ledger-interruptions.json']:
 copy(R/'target/acceptance/pre-alpha.4/development/r41'/n,A/'session-ledger'/n)
copy(R/'target/macos-storage-tests/session-ledger-plans.json',A/'session-ledger/plans.json',True)
for n in ['HelperManifest.json','production-dependencies.json','node-release-signature.txt']:copy(R/'target/macos-agent-package-tests/AgentRuntime'/n,A/'runtime'/n)
changed=subprocess.check_output(['git','diff','--name-only'],text=True).splitlines()+subprocess.check_output(['git','ls-files','--others','--exclude-standard'],text=True).splitlines()
extra=['crates/om-host-service/src/document/preview.rs','crates/om-host-service/src/document/preview/patch.rs','crates/om-host-service/src/agent/ingress.rs','macos/OpenMathNative/Storage/DocumentStore.swift','macos/OpenMathNative/DraftStore.swift','macos/OpenMathNative/Storage/DocumentOutbox.swift','macos/Scripts/test-session-ledger.sh','macos/OpenMathNativeTests/SessionLedgerFixtures/main.swift','macos/OpenMathNativeTests/SessionLedgerFixtures/interruption.py','macos/Scripts/test-agent-package.sh','docs/acceptance/pre-alpha.4/gates.toml']
sources={}
for n in sorted(set(changed+extra)):
 if n.startswith('docs/acceptance/pre-alpha.4/development/r43/patch-atomic-apply'):continue
 p=R/n
 if p.is_file():copy(p,A/'inputs'/n);sources[n]=sha(p.read_bytes())
write(A/'commands.json',[
 'python3 scripts/native_contracts.py','python3 macos/Scripts/generate-project.py',
 'CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 cargo check -p om-host-service -p om-apple-ffi --locked',
 'CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 cargo test -p om-host-service --test agent_endpoint --test commit_owner --test preview --test readonly_notebook --locked -- --nocapture',
 'CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 cargo test --workspace --locked -- --nocapture',
 'CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 cargo clippy --workspace --all-targets --locked -- -D warnings',
 'OPENMATH_SINGLE_TURN_FIXTURE=$PWD/target/debug/examples/single_turn_fixture target/macos-agent-package-tests/AgentRuntime/Node/node --test agent/test/*.test.mjs',
 'target/macos-agent-package-tests/AgentRuntime/Node/node app/node_modules/typescript/bin/tsc --noEmit --strict --skipLibCheck --target es2024 --module nodenext --allowImportingTsExtensions agent/src/main.ts',
 'python3 -X warn_default_encoding -W error::EncodingWarning -m unittest discover -s tests -v',
 'CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 bash macos/Scripts/test-session-ledger.sh',
 'bash macos/Scripts/test-agent-package.sh',
 'python3 macos/native-package/build-helper.py --output target/macos-agent-package-tests/AgentRuntime (added notebook.ts before fixture launch, while original Rust release build was running)',
 'bash macos/Scripts/test-agent-package.sh /Users/hert/Projects/openmath/target/macos-agent-package-tests/AgentRuntime',
 "env NODE_OPTIONS=--invalid-old-profile NODE_PATH=/untrusted/old-modules OPENAI_API_KEY=fixture-sentinel target/macos-agent-package-tests/fixtures '<signed App AgentRuntime absolute path>' '<fresh fixture root>' '<signed App absolute path>' (reuse exact just-compiled binary)",
 'python3 macos/native-package/build-helper.py --output macos/OpenMathNative/Resources/AgentRuntime',
 "xcodebuild -project macos/OpenMathNative.xcodeproj -scheme OpenMathNative -configuration Release -derivedDataPath target/macos-native -destination 'platform=macOS,arch=arm64' build",
 "codesign --verify --deep --strict 'target/macos-native/Build/Products/Release/OpenMath Preview.app'",
 'CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 cargo test --workspace --locked --no-run --message-format=json',
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
cleanup=json.loads((T/'patch-apply-cache-cleanup.json').read_text())
proof={'schema_version':1,'task':'R4.3.06','status':'completed','date':'2026-10-15','base_commit':subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip(),'scope':'Actual native atomic patch through original frozen plan, durable session admission, real DocCommitPort/editor barrier/SQLite receipt and exact data to bundled Pi; development evidence only',
 'verified':{'rust_workspace':{'passed':passed,'failed':failed,'suites':len(suites)},'rust_selected':33,'node26_pi':17,'python':17,'generated_dtos':389,'clippy':'workspace all-targets -D warnings','typescript':'strict own sources / upstream skipLibCheck','release':'passed','strict_signature':'passed after final signed App resource fixture','actual_apply':facts,
 'native_authority':'SourceBeginAgent receives only entropy native intent ID. Resolve only ACKed original apply args {preview_ref}; existing PreviewService frozen plan/DocCommitPort reused. No caller doc/grants/source or substitute manual snapshot',
 'final_barrier':'Original physical SourceBarrier uses actual live Agent task/grants/epoch scope when mapped to Agent intent; cancel/editor/marked invalidates before COMMIT; after barrier only original receipt can settle',
 'durability':'Canonical ACK -> SessionToolPort durable admission/start -> actual source admission/editor fence/SQLite -> independent original outbox effect/receipt -> Library tool_result -> canonical effect data Blob/readback -> native confirmation -> exact helper data',
 'unknown':'Unknown effect has original operation_ref and query_operation recovery, actual Pi ends loop without next model request or write retry. Original readback may later establish completion; historical unknown canonical bytes remain, indexed/fallback recovery shows actual completed fact',
 'no_fabricated_completion':'Completed canonical effect record requires actual matching durable tool_result and original target/hash. Missing Blob, changed original arguments, absent actual outcome and native wrong result hash rejected; terminal failed/cancelled are not success',
 'prior_storage_regression':'13 actual SIGKILL/60 original closed-schema events/4374 events >16MiB, all37 calls and37 plans recovered in5-row pages, no callbacks replayed',
 'cache_cleanup':{k:cleanup[k] for k in ['count','logical_bytes','deleted','all_candidates_absent']},'plan':'34/98 tasks complete; all32 final gates not_run/candidate UNSET/ready=false/released=false/runtime .3'},
 'ci_observation':json.loads((T/'patch-apply-base-ci.json').read_text()),
 'initial_failures':['Native generator did not support enum with null member; changed new nullable callback enum to supported explicit anyOf before generation, original no source overwrite','First apply fixture structurally decoded readonly records as effect records because effect optional fields allow decode; fixed explicit kind discriminator, original failure archive retains wire/input/SQL. Production SessionSemantics already checks kind before decoding','Initial apply helper snapshot produced before notebook.ts addition; rebuilt actual helper4170-file closure before the fixture launched. Actual final resources and source hash captured'],
 'pending':['Next R4.3.07 actual run/inspect/scratch tools','R4.3.04 physical lock-screen and R4.2.03 real IME candidate windows remain unverified; user OS input settings unchanged','Full ContextSnapshot producer remains R4.4.12; model/route factory here synthetic with actual stored canonical facts, no Registry/UI/live closure','64 other tasks plus all32 final gates and actual release remain required; no main merge/old public tag or package modification; no subagents/local iOS simulator/paid live API calls in batch'],
 'commands_file':'patch-atomic-apply-raw/commands.json','source_sha256':sources,'credential_scan':{'read_only_exact_authorized_file':True,'file_mode':'0600','directory_mode':'0700','files_scanned':scanned,'exact_key_matches':0,'key_not_copied':True},'raw_files_sha256':{str(p.relative_to(A)):sha(p.read_bytes()) for p in sorted(A.rglob('*')) if p.is_file()}}
write(D/'patch-atomic-apply.json',proof)
print(json.dumps({'source_inputs':len(sources),'raw_files':len(proof['raw_files_sha256']),'raw_bytes':sum(p.stat().st_size for p in A.rglob('*') if p.is_file()),'rust_passed':passed,'credential_matches':0,'cache_bytes':cleanup['logical_bytes']}))
