from pathlib import Path
import json, hashlib, gzip, subprocess, re, stat, sqlite3
R=Path.cwd();T=R/'target/acceptance/pre-alpha.4/development/r43';D=R/'docs/acceptance/pre-alpha.4/development/r43';A=D/'math-scratch-native-raw'
def sha(b):return hashlib.sha256(b).hexdigest()
def write(p,v):
 p.parent.mkdir(parents=True,exist_ok=True);p.write_text(json.dumps(v,ensure_ascii=False,indent=2)+'\n')
def completed(n):
 rows=[]
 for line in (T/n).read_text().splitlines():
  try:
   x=json.loads(line)
   if isinstance(x,dict) and 'evidence_root' in x:rows.append(x)
  except ValueError:pass
 assert len(rows)==1,n
 return rows[0]
def counts(n):
 rows=re.findall(r'test result: ok\. (\d+) passed; (\d+) failed',(T/n).read_text());assert rows and all(int(b)==0 for a,b in rows),n
 return {'passed':sum(int(a) for a,b in rows),'failed':0,'suites':len(rows)}
regression=counts('scratch-kernel-host-ffi-final.log');assert regression['passed']>200
assert 'BUILD SUCCEEDED' in (T/'scratch-release.log').read_text()
assert (T/'scratch-signature.log').read_text().strip()=='signature_exit=0'
assert 'ℹ pass 19' in (T/'scratch-pi-tests.log').read_text()
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
for p in sorted(T.glob('scratch-*')):
 if p.is_file() and p.suffix in {'.log','.json','.jsonl','.rs','.swift'}:copy(p,A/'logs'/p.name,p.suffix=='.jsonl')
for p in sorted(T.glob('scratch-failed-attempt-*')):
 if p.is_dir():tree(p,A/'original-failures'/p.name)
for p in sorted((T/'scratch-base-ios-artifacts').rglob('*')):
 if p.is_file():copy(p,A/'predecessor-ios-remote'/p.relative_to(T/'scratch-base-ios-artifacts'),True)
facts={}
for label,n in [('standalone','scratch-native-pi.log'),('signed-app','scratch-signed-pi.log')]:
 report=completed(n);root=Path(report['evidence_root']);assert root.parent==R/'target/macos-agent-package-tests';tree(root,A/label);cases={}
 for name in ['local','solve','ode','scene','partial']:
  p=root/'scratch-cases'/name
  scratch=json.loads((p/'actual-evaluate_scratch.json').read_text());inspect=json.loads((p/'actual-inspect_result.json').read_text());parent=json.loads((p/'actual-scratch-parent.json').read_text());wire=json.loads((p/'actual-scratch-wire.json').read_text());events=json.loads((p/'actual-scratch-ledger.json').read_text());recovery=json.loads((p/'actual-scratch-recovery.json').read_text())
  assert len(wire)==38 and sum(w['direction']=='helper' for w in wire)==17 and len(events)==28
  assert scratch['ok']==(name!='partial') and scratch['data']['evaluation_status']==('error' if name=='partial' else 'done') and scratch['data']['notebook_modified'] is False
  b=inspect['data']['payload']['binding'];assert inspect['ok'] and inspect['data']['origin']=='scratch' and b['acceptance']=='isolated' and 'checkpoint_id' not in b and 'kernel_state_revision' not in b
  assert b['result_ref']==scratch['data']['result_ref'] and b['parent_checkpoint']['checkpoint_id']==parent['checkpoint_id'] and b['parent_checkpoint']['checkpoint_hash']==parent['checkpoint_blob_hash']
  third=json.loads((p/'actual-native-http-body-3.json').read_text());fourth=json.loads((p/'actual-native-http-body-4.json').read_text());assert third['messages'][-1]['content']==(p/'actual-evaluate_scratch.json').read_text() and fourth['messages'][-1]['content']==(p/'actual-inspect_result.json').read_text()
  assert recovery['complete'] and not recovery['host_or_helper_created'] and recovery['main_checkpoint_unchanged'] and recovery['scratch_has_no_main_receipt'] and len(recovery['tools'])==3
  phases=[t['phase'] for t in recovery['tools']];assert phases.count('readonly_failed')==(1 if name=='partial' else 0) and phases.count('readonly_completed')==(2 if name=='partial' else 3)
  if name=='local':
   rows=inspect['data']['payload']['page']['rows'];assert rows[0]['cells'][0]['source']['input_form']=='99' and rows[1]['cells'][0]['source']['input_form']=='2'
  if name=='solve':assert {b['input_form'] for row in inspect['data']['payload']['presentation']['presentation']['solutions'] for b in row['bindings']}=={'2','3'}
  if name=='scene':assert inspect['data']['render_state']=='data_only' and inspect['data']['media_ref'] is None
  if name=='partial':assert b['status']=='Error' and len(scratch['data']['output_summary']['summary']['statements'])>=1 and scratch['error']['code']=='EXECUTION_FAILED'
  db=sqlite3.connect('file:'+str(p/'Documents/00000000-0000-0000-0000-000000000440/Generations/000001/authority.sqlite')+'?mode=ro&immutable=1',uri=True)
  sql={'kernel_operations':db.execute('SELECT count(*) FROM kernel_operations').fetchone()[0],'accepted_results':db.execute('SELECT count(*) FROM accepted_results').fetchone()[0]};db.close();assert sql=={'kernel_operations':2,'accepted_results':1}
  cases[name]={'wire_frames':38,'helper_frames':17,'sqlite_events':28,'http_turns':4,'third_and_fourth_http_exact_original_results':True,'actual_parent_receipt':parent,'scratch':scratch,'inspection':inspect,'storage_only_recovery':recovery,'original_main_sql_counts':sql}
 assert len(json.loads((root/'apply-task/actual-apply-wire.json').read_text()))==38 and len(json.loads((root/'apply-boundaries/actual-boundaries.json').read_text()))==5
 assert all(len(json.loads((root/'inspect-cases'/name/'actual-inspect-wire.json').read_text()))==29 for name in ['rational','solve','ode','scene'])
 facts[label]={'report':report,'scratch_cases':cases,'original_inspect_four_cases_read_preview_apply_and_five_boundaries_regression':True}
for n in ['HelperManifest.json','production-dependencies.json','node-release-signature.txt']:copy(R/'target/macos-agent-package-tests/AgentRuntime'/n,A/'runtime'/n)
changed=subprocess.check_output(['git','diff','--name-only'],text=True).splitlines()+subprocess.check_output(['git','ls-files','--others','--exclude-standard'],text=True).splitlines()
extra=['crates/om-host-service/build.rs','crates/om-host-service/src/tools/read.rs','crates/om-host-service/src/tools/run.rs','crates/om-host-service/src/references.rs','crates/om-host-service/src/kernel/mod.rs','crates/om-kernel/src/checkpoint/codec.rs','crates/om-kernel/tests/session.rs','crates/om-kernel/src/session/editing.rs','crates/om-kernel/src/retained_results.rs','macos/Scripts/test-agent-package.sh','macos/Scripts/build-native.sh','macos/OpenMathNative/Kernel/KernelCommitPort.swift','macos/OpenMathNative/Storage/AgentSessionTypes.swift','macos/OpenMathNative/Storage/AgentSessionStore.swift','docs/design/agent-tools.schema.json','docs/acceptance/pre-alpha.4/gates.toml']
sources={}
for n in sorted(set(changed+extra)):
 if n.startswith('docs/acceptance/pre-alpha.4/development/r43/math-scratch-native'):continue
 p=R/n
 if p.is_file():copy(p,A/'inputs'/n);sources[n]=sha(p.read_bytes())
write(A/'commands.json',[
 'CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 cargo check -p om-host-service --locked (initial private-method errors then corrected)',
 'CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 cargo test -p om-host-service --test math_scratch --test math_inspect --test result_runtime --test results --locked -- --nocapture (initial source-invalidation and fixture observations, corrected source)',
 'CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 cargo test -p om-kernel -p om-host-service -p om-apple-ffi --locked -- --nocapture (initial prefix fixtures then final genuine runtime recursion error)',
 'CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 cargo clippy -p om-host-service -p om-kernel --all-targets --locked -- -D warnings',
 'OPENMATH_SINGLE_TURN_FIXTURE=$PWD/target/debug/examples/single_turn_fixture target/macos-agent-package-tests/AgentRuntime/Node/node --test agent/test/*.test.mjs',
 'target/macos-agent-package-tests/AgentRuntime/Node/node app/node_modules/typescript/bin/tsc --noEmit --strict --skipLibCheck --target es2024 --module nodenext --allowImportingTsExtensions agent/src/main.ts',
 'python3 -X warn_default_encoding -W error::EncodingWarning -m unittest discover -s tests -v',
 'python3 scripts/native_contracts.py --check','python3 macos/Scripts/generate-project.py --check','cargo fmt --all --check',
 'bash macos/Scripts/test-agent-package.sh (original decimal callback remained symbolic, failed expected-error fixture retained)',
 'bash macos/Scripts/test-agent-package.sh $PWD/target/macos-agent-package-tests/AgentRuntime (final genuine recursive error fixture)',
 'bash macos/Scripts/build-native.sh',
 'env NODE_OPTIONS=--invalid-old-profile NODE_PATH=/untrusted/old-modules OPENAI_API_KEY=fixture-sentinel target/macos-agent-package-tests/fixtures <signed App Resources/AgentRuntime> <fresh attempt root> <signed App>',
 'codesign --verify --deep --strict target/macos-native/Build/Products/Release/OpenMath\\ Preview.app',
 'python3 scripts/check_pre_alpha_4_plan.py --check',
 'gh api repos/He-RT/om-openmath/actions/jobs/116034233545/logs (remote iOS startup failure)',
 'gh run download 38616794772 -n ios-acceptance-evidence -D target/acceptance/pre-alpha.4/development/r43/scratch-base-ios-artifacts',
 'xcrun xcresulttool get test-results summary --path <downloaded remote phone xcresult> (no simulator launch)',
 'CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 cargo test -p om-kernel -p om-host-service -p om-apple-ffi --locked --no-run --message-format=json'])
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
  b=p.read_bytes();b=gzip.decompress(b) if p.suffix=='.gz' else b;assert not any(k in b for k in keys),'Credential in evidence';scanned+=1
proof={'schema_version':1,'task':'R4.3.07','status':'in_progress','date':'2026-10-15','base_commit':subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip(),
 'scope':'Actual independently writable scratch and isolated retained-result origin through ACKed original intent/real accepted parent/own CAS thread/durable native readonly results/Pi; run_cells executor still required',
 'verified':{'rust_kernel_host_ffi':regression,'new_scratch_tests':8,'node26_pi':19,'python':17,'generated_dtos':389,'clippy':'host+kernel all-targets -D warnings','typescript':'strict own sources / upstream skipLibCheck','release':'passed','strict_signature':'passed after final signed App resource fixture','actual_native':facts,
 'authority':'Exact original canonical arguments SHA/current actual read-issued snapshot/current Agent task scope. Only genuinely accepted parent bytes; current source/config non-evaluating reconcile, only requested inherited definitions must be ready. definitions=false starts pure independent state',
 'isolation':'Dedicated four-job scratch CAS thread separate from main and readonly result workers; direct operation cancellation/close and bounded result publication. Scratch Session has no native config/file/model owner. Shared run_cell executor consumes actual bounded Interrupt; no cascade/RunAll, notebook mutation, main acceptance method or fabricated NativeKernelReceipt',
 'retention':'Real scratch checkpoint with its own scratch document/build crosses workers as bytes, capped16MiB; retained origin separate32 results/32MiB plus actual pin accounting, HMAC readonly refs10min/current-or-history scopes. Never registered as an accepted main checkpoint',
 'budgets':'Actual host clock required; restore/reconcile/evaluation/encoding share clamped min(host5s,actual config timeout,caller deadline)/4Msteps/cancel. Immutable publication decode has independent bounded result-owner budget. Request/source/queue/cursor/packet/output limits remain enforced',
 'tests':'Local Set99/parent Out2/same original random sample in two independent jobs, original checkpoint bytes unchanged; unready inherited definition fails, constant/unrelated/title/fresh-no-defs work, changed owner retires, exact SHA/scope/TTL/invalid args and actual mid-evaluation step cap, unsupported Import remains symbolic. Real recursion error keeps successful prefix; paused scratch still permits actual main result query and direct cancel; returned ref inspects as scratch',
 'unit_receipt_limit':'Unit parent KernelWorker real but matching acceptance receipts synthetic contracts, separately from actual SQLite/Pi cases',
 'progress':'34/98 complete; scratch Step3 checked after verified real signed fixtures; run_cells Steps1/2/4 and task Done still required; all32 final gates not_run/candidate UNSET/ready=false/released=false/runtime .3'},
 'original_failures':['Initial implementation called private Notebook.to_file/ReferenceRegistry.prune; corrected without widening private APIs','Appending scratch through normal source coordinator changed Math sequence and retired inherited definitions: actual red tests retained; added isolated append only to independently owned IO-free Session; main source semantics unchanged','Original whitespace assertion compared formatted InputForm; changed to exact original value paths','Decode exhausted caller budget initially mapped to CONTEXT_NOT_READY; preserved actual cancellation/deadline/step failure','Mixed Modern/Wolfram loop source had no successful record; decimal callback returned genuine symbolic unsupported value instead of a fatal error. Preserved both unit failures, original Swift input and actual native exit133/whole partial store. Final fixture uses existing proven Wolfram newline recursion-error program; kernel math semantics were not changed to force a failure'],
 'ci_observation':json.loads((T/'scratch-base-ci.json').read_text()),'predecessor_ios_failure':json.loads((T/'scratch-base-ios-observation.json').read_text()),
 'pending':['Actual run_cells selected/required/affected executor/KernelCommitPort/durable group and child executed/partial/unknown receipts and original result refs','R4.2.03 actual IME candidates and R4.3.04 physical lock-screen pending; user OS input settings preserved','64 unfinished tasks plus all32 final gates and actual release remain required; no main/old public tag or package changes; no subagents/local iOS simulator/paid live calls'],
 'commands_file':'math-scratch-native-raw/commands.json','source_sha256':sources,'credential_scan':{'read_only_exact_authorized_file':True,'file_mode':'0600','directory_mode':'0700','files_scanned':scanned,'exact_key_matches':0,'key_not_copied':True},
 'raw_files_sha256':{str(p.relative_to(A)):sha(p.read_bytes()) for p in sorted(A.rglob('*')) if p.is_file()}}
cleanup=T/'scratch-cache-cleanup.json'
if cleanup.is_file():proof['verified']['cache_cleanup']=json.loads(cleanup.read_text())
write(D/'math-scratch-native.json',proof)
print(json.dumps({'source_inputs':len(sources),'raw_files':len(proof['raw_files_sha256']),'raw_bytes':sum(p.stat().st_size for p in A.rglob('*') if p.is_file()),'rust_regression':regression,'credential_matches':0}))
