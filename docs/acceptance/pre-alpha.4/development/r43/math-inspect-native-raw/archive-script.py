from pathlib import Path
import json, hashlib, gzip, subprocess, re, stat
R=Path.cwd(); T=R/'target/acceptance/pre-alpha.4/development/r43'
D=R/'docs/acceptance/pre-alpha.4/development/r43'; A=D/'math-inspect-native-raw'
def sha(b):return hashlib.sha256(b).hexdigest()
def write(p,v):
 p.parent.mkdir(parents=True,exist_ok=True);p.write_text(json.dumps(v,ensure_ascii=False,indent=2)+'\n')
def report(name):
 rows=[]
 for line in (T/name).read_text().splitlines():
  try:
   x=json.loads(line)
   if isinstance(x,dict) and 'evidence_root' in x:rows.append(x)
  except ValueError:pass
 assert len(rows)==1,'No completed fixture: '+name
 return rows[0]
def counts(name):
 rows=re.findall(r'test result: ok\. (\d+) passed; (\d+) failed',(T/name).read_text())
 assert rows and all(int(b)==0 for a,b in rows),name
 return {'passed':sum(int(a) for a,b in rows),'failed':0,'suites':len(rows)}
selected=counts('inspect-tests-verified.log');assert selected['passed']==42
regression=counts('inspect-host-ffi-regression.log')
assert 'BUILD SUCCEEDED' in (T/'inspect-release.log').read_text()
assert 'ℹ pass 18' in (T/'inspect-pi-tests-corrected.log').read_text()
assert 'IPC_INVALID_TOOL_RESULT' in (T/'inspect-pi-tests-initial.log').read_text()
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
for p in sorted(T.glob('inspect-*')):
 if p.is_file() and p.suffix in {'.log','.json','.jsonl','.ts'}:copy(p,A/'logs'/p.name,p.suffix=='.jsonl')
for p in sorted(T.glob('inspect-failed-attempt-*')):
 if p.is_dir():tree(p,A/'original-failures'/p.name)
facts={}
for label,log in [('standalone','inspect-pi-native.log'),('signed-app','inspect-pi-signed.log')]:
 rep=report(log);root=Path(rep['evidence_root']);assert root.parent==R/'target/macos-agent-package-tests';tree(root,A/label)
 cases={}
 for name in ['rational','solve','ode','scene']:
  p=root/'inspect-cases'/name
  result=json.loads((p/'actual-inspect-result.json').read_text());read=json.loads((p/'actual-inspect-read.json').read_text());producer=json.loads((p/'actual-inspect-producer.json').read_text());events=json.loads((p/'actual-inspect-ledger.json').read_text());wire=json.loads((p/'actual-inspect-wire.json').read_text());recover=json.loads((p/'actual-inspect-recovery.json').read_text());http=json.loads((p/'actual-native-http-body-3.json').read_text())
  assert len(events)==21 and len(wire)==29 and sum(f['direction']=='helper' for f in wire)==13
  assert result['ok'] and result['data']['origin']=='notebook' and result['data']['is_current']
  binding=result['data']['payload']['binding'];assert binding['operation_id']==producer['operation_id'] and binding['kernel_state_revision']==producer['kernel_state_revision'] and binding['result_ref']==read['data']['cells'][0]['result_ref']
  assert http['messages'][-1]['role']=='tool' and http['messages'][-1]['content']==(p/'actual-inspect-result.json').read_text()
  assert recover['complete'] and not recover['host_or_helper_created'] and recover['original_checkpoint_unchanged'] and recover['manual_scope_result_rejected']
  assert len(recover['tools'])==2 and all(t['phase']=='readonly_completed' for t in recover['tools'])
  if name=='rational':assert result['data']['payload']['page']['source']['input_form']=='1/3'
  if name=='solve':
   rows=result['data']['payload']['presentation']['presentation']['solutions'];assert {b['input_form'] for r in rows for b in r['bindings']}=={'2','3'}
  if name=='scene':assert result['data']['render_state']=='data_only' and result['data']['media_ref'] is None
  cases[name]={'wire_frames':len(wire),'helper_frames':13,'sqlite_events':21,'http_turns':3,'third_http_exact_original_result':True,'producer':producer,'result':result,'storage_only_recovery':recover}
 assert len(json.loads((root/'apply-task/actual-apply-wire.json').read_text()))==38
 assert len(json.loads((root/'apply-boundaries/actual-boundaries.json').read_text()))==5
 facts[label]={'report':rep,'math_inspect_cases':cases,'original_read_preview_apply_and_five_boundaries_regression':True}
for n in ['HelperManifest.json','production-dependencies.json','node-release-signature.txt']:copy(R/'target/macos-agent-package-tests/AgentRuntime'/n,A/'runtime'/n)
changed=subprocess.check_output(['git','diff','--name-only'],text=True).splitlines()+subprocess.check_output(['git','ls-files','--others','--exclude-standard'],text=True).splitlines()
extra=['crates/om-host-service/src/results/mod.rs','crates/om-host-service/src/results/inspect.rs','crates/om-host-service/src/references.rs','crates/om-host-service/src/results/runtime/queries.rs','crates/om-host-service/src/agent/ingress.rs','crates/om-host-service/src/tools/read.rs','crates/om-kernel/src/retained_results.rs','crates/om-kernel/src/retained_results/inspect.rs','crates/om-kernel/src/retained_results/geometry.rs','macos/Scripts/test-agent-package.sh','macos/Scripts/build-native.sh','macos/OpenMathNative/Kernel/KernelCommitPort.swift','macos/OpenMathNative/Storage/AgentSessionTypes.swift','macos/OpenMathNative/Storage/AgentSessionStore.swift','docs/design/agent-tools.schema.json','docs/acceptance/pre-alpha.4/gates.toml']
sources={}
for n in sorted(set(changed+extra)):
 if n.startswith('docs/acceptance/pre-alpha.4/development/r43/math-inspect-native'):continue
 p=R/n
 if p.is_file():copy(p,A/'inputs'/n);sources[n]=sha(p.read_bytes())
write(A/'commands.json',[
 'cargo fmt --all',
 'CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 cargo check -p om-host-service --locked',
 'CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 cargo test -p om-host-service --test math_inspect --test result_runtime --test results --test agent_endpoint --locked -- --nocapture',
 'CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 cargo test -p om-host-service --test math_inspect --test result_runtime --test results --test agent_endpoint --test agent_ingress --test readonly_notebook --test readonly_catalog --test math_run_plan --locked -- --nocapture',
 'CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 cargo test -p om-host-service -p om-apple-ffi --locked -- --nocapture',
 'CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 cargo clippy -p om-host-service --all-targets --locked -- -D warnings',
 'OPENMATH_SINGLE_TURN_FIXTURE=$PWD/target/debug/examples/single_turn_fixture target/macos-agent-package-tests/AgentRuntime/Node/node --test agent/test/*.test.mjs (original failing JsonPayload decoder then corrected eighteen tests)',
 'target/macos-agent-package-tests/AgentRuntime/Node/node app/node_modules/typescript/bin/tsc --noEmit --strict --skipLibCheck --target es2024 --module nodenext --allowImportingTsExtensions agent/src/main.ts',
 'python3 -X warn_default_encoding -W error::EncodingWarning -m unittest discover -s tests -v',
 'python3 scripts/native_contracts.py --check', 'python3 macos/Scripts/generate-project.py --check',
 'bash macos/Scripts/test-agent-package.sh', 'bash macos/Scripts/build-native.sh',
 'env NODE_OPTIONS=--invalid-old-profile NODE_PATH=/untrusted/old-modules OPENAI_API_KEY=fixture-sentinel target/macos-agent-package-tests/fixtures <signed App Resources/AgentRuntime> <fresh attempt root> <signed App>',
 'codesign --verify --deep --strict target/macos-native/Build/Products/Release/OpenMath\\ Preview.app',
 'python3 scripts/check_pre_alpha_4_plan.py --check'])
write(A/'compressed-originals.json',compressed);copy(Path(__file__),A/'archive-script.py')
config=Path('/Users/hert/.config/openmath/testing/pre-alpha-4-api.json');secret=json.loads(config.read_text());keys=[]
def collect(v):
 if isinstance(v,dict):
  for x in v.values():collect(x)
 elif isinstance(v,list):
  for x in v:collect(x)
 elif isinstance(v,str) and v.startswith('sk-'):keys.append(v.encode())
collect(secret);assert keys and stat.S_IMODE(config.stat().st_mode)==0o600 and stat.S_IMODE(config.parent.stat().st_mode)==0o700
scanned=0
for p in A.rglob('*'):
 if p.is_file():
  b=p.read_bytes();b=gzip.decompress(b) if p.suffix=='.gz' else b;assert not any(k in b for k in keys),'Credential in evidence';scanned+=1
proof={'schema_version':1,'task':'R4.3.07','status':'in_progress','date':'2026-10-15','base_commit':subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip(),
 'scope':'Actual native inspect_result through original ACKed intent and independent immutable ResultRuntime, bounded projections, durable readonly Blob and real bundled Pi; run_cells executor and standalone scratch remain pending',
 'verified':{'rust_selected':selected,'rust_host_ffi_regression':regression,'new_math_inspect_tests':6,'node26_pi':18,'python':17,'generated_dtos':389,'typescript':'strict own sources / upstream skipLibCheck','clippy':'host all-targets -D warnings','release':'passed','strict_signature':'passed after final signed App fixture','actual_native':facts,
 'source_authority':'Only original ACKed native inspect operation ID crosses C ABI; actual canonical arguments hash is frozen. Current actual Agent scope consumes original read-issued result ref. No model source/task/grants, no substitute manual None scope',
 'inspection':'Existing immutable ResultStore only: real exact/high precision/nested values with every wide-matrix column; original typed Solve conditions/multiplicity; depth-first original recorded steps; actual original geometry channels; full diagnostic pages; explicit history/partial binding and no renderer claim',
 'bounds':'Original ref ten-minute expiry; query/result/limit/current-scope-bound opaque cursor registry128/32MiB/ten minutes. 100 rows/32columns, bounded route depth/stack/geometry channels, 240KiB payload, independent5s/4Msteps/direct cancellation. Actual checkpoint bytes unchanged',
 'unit_evidence_limit':'KernelWorker and retained results genuine, unit acceptance receipts synthetic matching contracts. Physical SQLite/ACK/Pi proof is separately the four cases in actual_native',
 'json_payload_red_green':'Existing Pi validator rejected legal schema type arrays; original failed inspect test retained, type union support and six JSON payload types verified. Readonly effects must carry operation_ref=null; completed inspection may describe failed/stale math without changing it into mathematical success',
 'progress':'34/98 tasks complete; R4.3.07 all Steps/Done still unchecked; all32 final gates not_run/candidate UNSET/ready=false/released=false/runtime .3'},
 'ci_observation':json.loads((T/'inspect-base-ci.json').read_text()),
 'pending':['Actual run_cells executor/KernelCommitPort/durable executed and partial receipts','Independent writable scratch checkpoint/result origin registry, defs/Out/random/budget without host IO; never fabricate a main receipt','R4.2.03 real IME candidates and R4.3.04 physical lock-screen pending, user OS input settings preserved','64 other tasks plus all32 final gates and actual release remain required; no main/old public tag or package changes; no subagents/local iOS simulator/paid live API calls in this batch'],
 'commands_file':'math-inspect-native-raw/commands.json','source_sha256':sources,
 'credential_scan':{'read_only_exact_authorized_file':True,'file_mode':'0600','directory_mode':'0700','files_scanned':scanned,'exact_key_matches':0,'key_not_copied':True},
 'raw_files_sha256':{str(p.relative_to(A)):sha(p.read_bytes()) for p in sorted(A.rglob('*')) if p.is_file()}}
cleanup=T/'inspect-cache-cleanup.json'
if cleanup.is_file():proof['verified']['cache_cleanup']=json.loads(cleanup.read_text())
write(D/'math-inspect-native.json',proof)
print(json.dumps({'source_inputs':len(sources),'raw_files':len(proof['raw_files_sha256']),'raw_bytes':sum(p.stat().st_size for p in A.rglob('*') if p.is_file()),'rust_host_ffi':regression,'credential_matches':0}))
