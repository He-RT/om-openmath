from pathlib import Path
import json, hashlib, gzip, subprocess, re, stat, sqlite3
R=Path.cwd(); T=R/'target/acceptance/pre-alpha.4/development/r43'
D=R/'docs/acceptance/pre-alpha.4/development/r43'; A=D/'math-run-planning-raw'
def sha(b): return hashlib.sha256(b).hexdigest()
def write(p,v):
 p.parent.mkdir(parents=True,exist_ok=True); p.write_text(json.dumps(v,ensure_ascii=False,indent=2)+'\n')
def report(name):
 rows=[]
 for line in (T/name).read_text().splitlines():
  try:
   d=json.loads(line)
   if isinstance(d,dict) and 'evidence_root' in d: rows.append(d)
  except ValueError: pass
 assert len(rows)==1, name
 return rows[0]
def count(name):
 rows=re.findall(r'test result: ok\. (\d+) passed; (\d+) failed',(T/name).read_text())
 assert rows and all(int(b)==0 for a,b in rows), name
 return sum(int(a) for a,b in rows)
assert count('math-plan-kernel-indexed.log')==35
assert count('math-plan-title-green-corrected.log')==30
assert 'BUILD SUCCEEDED' in (T/'math-plan-native-release.log').read_text()
assert 'panicked' in (T/'math-plan-title-red.log').read_text()
compressed=[]; A.mkdir(parents=True,exist_ok=True)
def copy(src,dst,compress=False):
 assert src.is_file() and not src.is_symlink(); b=src.read_bytes(); dst.parent.mkdir(parents=True,exist_ok=True)
 if compress:
  dst=Path(str(dst)+'.gz'); z=gzip.compress(b,mtime=0); assert gzip.decompress(z)==b; dst.write_bytes(z)
  compressed.append({'source':str(src.relative_to(R)),'archive':str(dst.relative_to(A)),'original_bytes':len(b),'original_sha256':sha(b)})
 else: dst.write_bytes(b); assert dst.read_bytes()==b
def tree(src,dst):
 for p in sorted(src.rglob('*')):
  if p.is_file(): copy(p,dst/p.relative_to(src),p.name.endswith(('.sqlite','.sqlite-wal','.sqlite-shm')))
for p in sorted(T.glob('math-plan-*')):
 if p.is_file() and p.suffix in {'.log','.json','.jsonl','.rs','.swift'}: copy(p,A/'logs'/p.name,p.suffix=='.jsonl')
tree(T/'math-plan-base-ci-artifacts',A/'predecessor-ci-artifacts')
provider_facts={}
for label,name in [('isolated','math-plan-provider-total-isolated.log'),('forced','math-plan-provider-total-forced.log')]:
 text=(T/name).read_text(); assert 'route boundaries passed' in text
 root=Path(re.search(r'^Evidence: (.+)$',text,re.M).group(1)); assert root.parent==R/'target/macos-provider-transport-tests'
 tree(root,A/'provider-timeout'/label)
 provider_facts[label]={p.stem:json.loads(p.read_text()) for p in root.glob('timeout-*.json')}
forced=provider_facts['forced']['timeout-total-delayed-header']
assert forced['actual_reason']=='totalTimeout' and forced['competing_timer_actually_woke'] and forced['requests']==1 and forced['outcome']=='dispatchOutcomeUnknown'
cleanup=json.loads((T/'math-plan-cache-cleanup.json').read_text()); assert cleanup['deleted'] and cleanup['all_candidates_absent']
facts={}
for label,name in [('standalone','math-plan-pi-title-settled.log'),('signed-app','math-plan-pi-title-signed.log')]:
 rep=report(name); root=Path(rep['evidence_root']); assert root.parent==R/'target/macos-agent-package-tests'
 tree(root,A/label)
 body=json.loads((root/'actual-readonly-result.json').read_text()); wire=json.loads((root/'actual-production-wire.json').read_text())
 events=json.loads((root/'actual-production-ledger.json').read_text()); cell=body['data']['cells'][0]
 assert body['ok'] and body['document']['revision']==2 and cell['status']=='Done' and cell['result_ref'].startswith('result-')
 assert len(body['data']['recent_transaction_refs'])==2 and len(wire)==20 and len(events)==14
 http=json.loads((root/'actual-native-http-body-2.json').read_text()); assert http['messages'][-1]['content']==(root/'actual-readonly-result.json').read_text()
 p=root/'apply-task'; assert len(json.loads((p/'actual-apply-wire.json').read_text()))==38 and len(json.loads((p/'actual-apply-ledger.json').read_text()))==31
 bounds=json.loads((root/'apply-boundaries/actual-boundaries.json').read_text()); assert len(bounds)==5 and all(x['storage_only_recovery'] for x in bounds)
 db=sqlite3.connect('file:'+str(root/'Documents/00000000-0000-0000-0000-000000000432/Generations/000001/authority.sqlite')+'?mode=ro',uri=True)
 schema=[{'name':n,'sql':s} for n,s in db.execute("SELECT name,sql FROM sqlite_schema WHERE type='table' ORDER BY name")]; db.close()
 write(A/label/'actual-document-sql-schema.json',schema)
 facts[label]={'report':rep,'read_result':body,'original_result_kept_after_actual_title_commit':True,'actual_document_revision':2,'read_wire_frames':20,'session_events':14,'http_body_contains_exact_result':True,'apply_regression_frames':38,'apply_regression_events':31,'actual_boundary_cases':bounds}
# The original invocation ended without a terminal fixture report. Preserve its partial bytes;
# no exit status/cause is inferred and this attempt contributes no pass evidence.
incomplete=R/'target/macos-agent-package-tests/attempt-qtIzgA'
if incomplete.is_dir(): tree(incomplete,A/'incomplete-original-attempt')
changed=subprocess.check_output(['git','diff','--name-only'],text=True).splitlines()+subprocess.check_output(['git','ls-files','--others','--exclude-standard'],text=True).splitlines()
extra=['crates/om-host-service/src/kernel/worker/runtime.rs','crates/om-host-service/src/kernel/mod.rs','crates/om-host-service/src/document/coordinator.rs','crates/om-host-service/src/tools/dispatch.rs','crates/om-host-service/src/results/mod.rs','crates/om-kernel/src/session/working.rs','macos/Scripts/test-agent-package.sh','macos/Scripts/build-native.sh','macos/Scripts/test-provider-transport.sh','macos/OpenMathNative/Models/ProviderTransport.swift','macos/OpenMathNativeTests/ProviderTransportFixtures/LoopbackServer.swift','docs/acceptance/pre-alpha.4/gates.toml']
sources={}
for n in sorted(set(changed+extra)):
 if n.startswith('docs/acceptance/pre-alpha.4/development/r43/math-run-planning'): continue
 p=R/n
 if p.is_file(): copy(p,A/'inputs'/n); sources[n]=sha(p.read_bytes())
write(A/'commands.json',[
 'CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 cargo test -p om-kernel --test run_plan --test reactive --test reactive_edges --test source_transactions --test working_session --locked -- --nocapture',
 'CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 cargo test -p om-host-service --test math_run_plan --test readonly_notebook --test kernel_endpoint --test kernel_worker --test kernel_acceptance --test kernel_states --locked -- --nocapture',
 'CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 cargo clippy -p om-host-service -p om-kernel --all-targets --locked -- -D warnings',
 'python3 -X warn_default_encoding -W error::EncodingWarning -m unittest discover -s tests -v',
 'python3 scripts/native_contracts.py --check', 'python3 macos/Scripts/generate-project.py --check', 'cargo fmt --all --check',
 'bash macos/Scripts/test-agent-package.sh /Users/hert/Projects/openmath/target/macos-agent-package-tests/AgentRuntime (original incomplete attempt; no terminal report)',
 'env NODE_OPTIONS=--invalid-old-profile NODE_PATH=/untrusted/old-modules OPENAI_API_KEY=fixture-sentinel target/macos-agent-package-tests/fixtures "$PWD/target/macos-agent-package-tests/AgentRuntime" <fresh math-plan-standalone root> (successful original binary reuse)',
 'bash macos/Scripts/build-native.sh',
 'env NODE_OPTIONS=--invalid-old-profile NODE_PATH=/untrusted/old-modules OPENAI_API_KEY=fixture-sentinel target/macos-agent-package-tests/fixtures <signed App Resources/AgentRuntime> <fresh math-plan-signed root> <signed App>',
 'codesign --verify --deep --strict target/macos-native/Build/Products/Release/OpenMath\\ Preview.app',
 'python3 scripts/check_pre_alpha_4_plan.py --check',
 'gh run view 38605677497 --json status,conclusion,jobs',
 'gh run view 38605677497 --log-failed',
 'gh run download 38605677497 -n macos-native-development-evidence -D target/acceptance/pre-alpha.4/development/r43/math-plan-base-ci-artifacts',
 'bash macos/Scripts/test-provider-transport.sh (isolated and final forced delayed-header cases)',
 'CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 cargo test -p om-kernel --test run_plan --test reactive --test reactive_edges --test source_transactions --test working_session --locked --no-run --message-format=json',
 'CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 cargo test -p om-host-service --test math_run_plan --test readonly_notebook --test kernel_endpoint --test kernel_worker --test kernel_acceptance --test kernel_states --locked --no-run --message-format=json'])
write(A/'compressed-originals.json',compressed); copy(Path(__file__),A/'archive-script.py')
config=Path('/Users/hert/.config/openmath/testing/pre-alpha-4-api.json'); private=json.loads(config.read_text()); keys=[]
def collect(v):
 if isinstance(v,dict):
  for x in v.values(): collect(x)
 elif isinstance(v,list):
  for x in v: collect(x)
 elif isinstance(v,str) and v.startswith('sk-'): keys.append(v.encode())
collect(private); assert keys and stat.S_IMODE(config.stat().st_mode)==0o600 and stat.S_IMODE(config.parent.stat().st_mode)==0o700
scanned=0
for p in A.rglob('*'):
 if p.is_file():
  b=p.read_bytes(); b=gzip.decompress(b) if p.suffix=='.gz' else b
  assert not any(k in b for k in keys), 'Credential in evidence'; scanned+=1
proof={'schema_version':1,'task':'R4.3.07','status':'in_progress','date':'2026-10-15','base_commit':subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip(),
 'scope':'Non-evaluating actual selected/required/affected planning foundation plus real native title-only math freshness regression; no Agent run/inspect/scratch completion',
 'verified':{'rust_kernel':35,'rust_host':30,'new_planner_tests':10,'python':17,'generated_dtos':389,'clippy':'host+kernel all-targets -D warnings','release':'passed','strict_signature':'passed after actual signed App resource fixture',
 'planning':'Actual parsed dependencies/live write owners, shared topological cycle/blocked ordering, Done prerequisite readiness, trusted native allowed set, 1000-cell cap, cancellation and charged traversal. No evaluation or checkpoint/defs/Out/random mutation',
 'host_input':'Original RunCells intent arguments SHA and closed schema; exact current real read-issued snapshot; actual accepted KernelState restore/reconcile on separate thread; missing parent/stale source/task/grant/config/editor/expiry rejected; no full-cell source exposure upgrade',
 'receipt_limit':'math_run_plan unit test uses genuine KernelWorker states with synthetic matching acceptance receipts. It is not physical SQLite/main execution or an ACKed Agent proposal integration proof',
 'title_freshness_red_green':'Original whole-document hash comparison falsely marked Done math Stale after title rename; retained original read.rs and failing assertion. Actual math source+epoch+config comparison keeps original producer facts; math changes remain stale',
 'actual_native':facts,'provider_timeout_fixture':{'predecessor_ci':'38605677497 terminal failure; Rust/frontend/dependencies/iOS success; Mac total negative observed firstResponseTimeout before any headers, original artifact JSON retained','production_change':False,'isolation':'Existing trusted timer wake probe holds competing callbacks for total case only. Original 30ms first-response and idle tests unchanged. Forced 200ms headers causes actual competing callback wake and real independent total timeout','actual_observations':provider_facts},
 'cache_cleanup':{k:cleanup[k] for k in ['count','logical_bytes','deleted','all_candidates_absent']},
 'progress':'34/98 complete, R4.3.07 in_progress with all Steps/Done unchecked; all32 final gates not_run/candidate UNSET/ready=false/released=false/runtime .3'},
 'original_attempts':['Initial test attempted to serialize non-Serialize execution context; corrected unit observation','Modern callable fixture did not create intended global owner; actual supported Wolfram delayed Set does, original failure retained','Nonexistent kernel_runtime test target corrected to actual explicit targets','Unused import Clippy failure corrected','Title RED retained; first GREEN queried symbols in outline, corrected to explicit real symbols view','Original standalone invocation ended without a terminal report, process absent and partial directory retained; cause/exit unknown, contributes no pass. Successful fresh-directory rerun reuses exact compiled binary'],
 'pending':['Actual run_cells coordinator/KernelCommitPort/durable executed/partial/resultrefs','Actual inspect_result and standalone retained scratch origin; independent writable defs/Out/random/budget without host IO','R4.2.03 real IME candidates and R4.3.04 physical lock-screen remain unverified; user OS input settings preserved','64 other tasks plus all32 final gates and actual release required; no main/old public tag or package changes; no subagents/local iOS simulator/paid live calls'],
 'ci_observation':json.loads((T/'math-plan-base-ci.json').read_text()),'commands_file':'math-run-planning-raw/commands.json','source_sha256':sources,
 'credential_scan':{'read_only_exact_authorized_file':True,'file_mode':'0600','directory_mode':'0700','files_scanned':scanned,'exact_key_matches':0,'key_not_copied':True},
 'raw_files_sha256':{str(p.relative_to(A)):sha(p.read_bytes()) for p in sorted(A.rglob('*')) if p.is_file()}}
write(D/'math-run-planning.json',proof)
print(json.dumps({'source_inputs':len(sources),'raw_files':len(proof['raw_files_sha256']),'raw_bytes':sum(p.stat().st_size for p in A.rglob('*') if p.is_file()),'rust_passed':65,'credential_matches':0}))
