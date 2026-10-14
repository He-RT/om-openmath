from pathlib import Path
import json,hashlib,gzip,subprocess,stat,re,os
R=Path.cwd();T=R/'target/acceptance/pre-alpha.4/development/r43';D=R/'docs/acceptance/pre-alpha.4/development/r43';A=D/'readonly-outputs-history-raw';A.mkdir(parents=True,exist_ok=True)
def sha(b): return hashlib.sha256(b).hexdigest()
def write(p,d):p.parent.mkdir(parents=True,exist_ok=True);p.write_text(json.dumps(d,ensure_ascii=False,indent=2)+'\n')
compressed=[]
def copy(src,dst,compress=False):
 assert src.is_file() and not src.is_symlink()
 b=src.read_bytes();dst.parent.mkdir(parents=True,exist_ok=True)
 if compress:
  dst=Path(str(dst)+'.gz');z=gzip.compress(b,mtime=0);assert gzip.decompress(z)==b;dst.write_bytes(z)
  compressed.append({'source':str(src.relative_to(R)),'archive':str(dst.relative_to(A)),'original_bytes':len(b),'original_sha256':sha(b)})
 else:dst.write_bytes(b);assert dst.read_bytes()==b

def tree(src,dst,all_gzip=False):
 for p in sorted(src.rglob('*')):
  if p.is_file():copy(p,dst/p.relative_to(src),all_gzip or p.name.endswith(('.sqlite','.sqlite-wal','.sqlite-shm')))
for p in sorted(T.glob('readonly-outputs*')):
 if p.is_file() and p.suffix in {'.log','.json','.jsonl'}:copy(p,A/'logs'/p.name,p.suffix=='.jsonl')
tree(T/'readonly-outputs-advisories',A/'rustsec')
tree(T/'readonly-outputs-ci-artifact',A/'ci-38573532854')
for attempt,label in [('attempt-NqM8BA','standalone'),('attempt-imKCnb','signed-app')]:tree(R/'target/macos-agent-package-tests'/attempt,A/label)
tree(R/'target/macos-provider-transport-tests/attempt-AQ49Nh',A/'transport-corrected')
report=json.loads((R/'target/macos-storage-tests/history-report.json').read_text());history=Path(report['fixture_root']);assert history.parent==R/'target/macos-storage-tests'
tree(history,A/'history-206',True)
copy(R/'target/macos-storage-tests/history-report.json',A/'history-report.json')
copy(R/'target/macos-storage-tests/history-plans.json',A/'history-plans.json',True)
for name in ['HelperManifest.json','production-dependencies.json','node-release-signature.txt']:copy(R/'target/macos-agent-package-tests/AgentRuntime'/name,A/'runtime'/name)
changed=subprocess.check_output(['git','diff','--name-only'],text=True).splitlines()+subprocess.check_output(['git','ls-files','--others','--exclude-standard'],text=True).splitlines()
extra=['crates/om-host-service/src/tools/catalog.rs','crates/om-host-service/src/tools/dispatch.rs','macos/Scripts/test-history.sh','macos/Scripts/test-agent-package.sh','macos/Scripts/test-provider-transport.sh','docs/acceptance/pre-alpha.4/gates.toml','scripts/check_pre_alpha_4_plan.py']
sources={}
for name in sorted(set(changed+extra)):
 if name.startswith('docs/acceptance/pre-alpha.4/development/r43/readonly-outputs-history'):continue
 p=R/name
 if p.is_file():copy(p,A/'inputs'/name);sources[name]=sha(p.read_bytes())
commands=[
 'CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 cargo check -p om-host-service --locked',
 'CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 cargo test -p om-host-service -p om-agent-contract -p om-llm -p om-apple-ffi --locked -- --nocapture',
 'CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 cargo test --workspace --locked -- --nocapture',
 'CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 cargo clippy --workspace --all-targets --locked -- -D warnings',
 'CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 cargo test -p om-host-service --test result_runtime notebook_output_refs_resolve_real_exact_value_and_never_substitute_a_later_state --locked -- --nocapture',
 'OPENMATH_SINGLE_TURN_FIXTURE=$PWD/target/debug/examples/single_turn_fixture target/macos-agent-package-tests/AgentRuntime/Node/node --test agent/test/*.test.mjs',
 'target/macos-agent-package-tests/AgentRuntime/Node/node app/node_modules/typescript/bin/tsc --noEmit --strict --skipLibCheck --target es2024 --module nodenext --allowImportingTsExtensions agent/src/main.ts',
 'python3 -X warn_default_encoding -W error::EncodingWarning -m unittest discover -s tests -v',
 'CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 bash macos/Scripts/test-history.sh',
 'bash macos/Scripts/test-agent-package.sh /Users/hert/Projects/openmath/target/macos-agent-package-tests/AgentRuntime',
 "bash macos/Scripts/test-agent-package.sh '/Users/hert/Projects/openmath/target/macos-native/Build/Products/Release/OpenMath Preview.app/Contents/Resources/AgentRuntime' '/Users/hert/Projects/openmath/target/macos-native/Build/Products/Release/OpenMath Preview.app'",
 'CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 bash macos/Scripts/test-provider-transport.sh',
 'cargo update -p rustls --precise 0.23.46 (unrelated resolver edge changes subsequently restored; final diff only rustls version/checksum)',
 'cargo deny --locked check',
 'python3 macos/native-package/build-helper.py --output macos/OpenMathNative/Resources/AgentRuntime',
 "xcodebuild -project macos/OpenMathNative.xcodeproj -scheme OpenMathNative -configuration Release -derivedDataPath target/macos-native -destination 'platform=macOS,arch=arm64' build",
 "codesign --verify --deep --strict 'target/macos-native/Build/Products/Release/OpenMath Preview.app'",
 'python3 scripts/native_contracts.py --check', 'python3 macos/Scripts/generate-project.py --check','cargo fmt --all --check',
 'CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 cargo test --workspace --locked --no-run --message-format=json',
 'python3 scripts/check_pre_alpha_4_plan.py --check'
]
write(A/'commands.json',commands)
write(A/'compressed-originals.json',compressed)
copy(Path(__file__).resolve(),A/'archive-script.py')
# Read only the exact user-authorized testing config, never enumerate profiles/env/old keys.
config=Path('/Users/hert/.config/openmath/testing/pre-alpha-4-api.json');private=json.loads(config.read_text());secrets=[]
def collect(x):
 if isinstance(x,dict):
  for v in x.values():collect(v)
 elif isinstance(x,list):
  for v in x:collect(v)
 elif isinstance(x,str) and x.startswith('sk-'):secrets.append(x.encode())
collect(private);assert secrets,'No API credential found in explicitly authorized file'
assert stat.S_IMODE(config.stat().st_mode)==0o600 and stat.S_IMODE(config.parent.stat().st_mode)==0o700
matches=[];scanned=0
for p in A.rglob('*'):
 if p.is_file():
  data=p.read_bytes();data=gzip.decompress(data) if p.suffix=='.gz' else data
  scanned+=1
  if any(s in data for s in secrets):matches.append(str(p.relative_to(A)))
assert not matches,'Credential present in evidence; abort'
work=(T/'readonly-outputs-workspace-tests.log').read_text();counts=re.findall(r'test result: ok\. (\d+) passed; (\d+) failed',work)
assert sum(int(a) for a,b in counts)==1297 and sum(int(b) for a,b in counts)==0
actual=json.loads((A/'signed-app/actual-readonly-result.json').read_text());fresh=json.loads((A/'signed-app/actual-new-host-persisted-history.json').read_text());recovery=json.loads((A/'signed-app/actual-readonly-recovery.json').read_text())
assert actual['data']['cells'][0]['result_ref'].startswith('result-') and actual['data']['recent_transaction_refs']==fresh['data']['recent_transaction_refs'] and fresh['data']['cells'][0]['result_ref'] is None
assert recovery['complete'] and not recovery['host_or_helper_created'] and recovery['last_event_sequence']==14
cleanup=json.loads((T/'readonly-outputs-cache-cleanup.json').read_text());assert cleanup['deleted'] and cleanup['count']==216
proof={'schema_version':1,'task':'R4.3.05','status':'completed','date':'2026-10-15','base_commit':subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip(),'scope':'Complete actual readonly tool output references and durable history; development evidence only, no final candidate gate',
 'verified':{'readonly_tools':['read_notebook','search_functions','get_function_docs'],'rust_workspace':{'passed':1297,'failed':0,'suites':len(counts)},'rust_selected':207,'node26_pi':14,'python':17,'generated_dtos':382,'typescript':'strict own sources / upstream skipLibCheck','clippy':'workspace all-targets -D warnings','fmt':'passed','release_build':'passed','strict_signature':'passed after signed App fixture',
 'output_authority':'ResultRuntime real retained StoredResult, exact doc/cell/source hash and original read snapshot upper kernel revision; scoped HMAC ref, no source evaluation or random draw/checkpoint mutation',
 'output_tests':['actual retained exact 1/3 resolves through inspector','actual second evaluation preserves old producer1 at upper1 and selects producer2 at upper2','wrong source/prior upper -> null; future upper and cross-task token rejected','independent result worker paused: source status/direct cancel remain responsive'],
 'history_authority':'Single serialized document read of confirmed head plus 32 actual durable history transactions; native callback checks scope and head hash; hash/summary/SQL columns must agree; no grants restored',
 'history_tests':['206 actual SQLite source transactions and original prune/rollback/reopen regression','tampered SQL operation vs original hashed summary rejected','wrong head hash callback rejected','new explicitly activated host after storage reopen sees same durable transaction with no original live controller receipt','historical snapshots keep original refs/limits/duplicates rejected'],
 'actual_signed_app':{'frames':20,'host_frames':11,'helper_frames':9,'sqlite_events':14,'source_setup':'real DocCommitPort rename transaction, then actual KernelCommitPort 2+2','original_args':'{ "view" : "outline", "include_outputs" : true }','result_ref':actual['data']['cells'][0]['result_ref'],'transaction_refs':actual['data']['recent_transaction_refs'],'payload_in_original_blob_and_second_http_body':True,'readonly_effect_events':0,'storage_only_recovery':recovery,'later_explicit_new_host_history':fresh,'manual_after_helper_and_sigkill':'4','model_route':'synthetic fixture factory with real persisted canonical data; Registry/UI/full ContextSnapshot live producer still later'},
 'cache_cleanup':{k:cleanup[k] for k in ['count','logical_bytes','deleted','all_candidates_absent']},'plan':'33/98 tasks complete; 32 final gates not_run/candidate UNSET/ready=false/released=false/runtime .3'},
 'ci_observation':json.loads((T/'readonly-outputs-base-ci.json').read_text()),
 'ci_repairs':{'prior_run':'https://github.com/He-RT/om-openmath/actions/runs/38573532854','prior_result':'terminal failure; Rust/frontend/iPhone+iPad success, dependencies and native_macos failed','rustls':{'before':'0.23.45','after':'0.23.46','change':'Cargo.lock version/checksum only; unrelated resolver edges restored','check':'cargo deny --locked check passed','advisories':[{'id':f'RUSTSEC-2026-{i}','url':f'https://github.com/RustSec/advisory-db/blob/main/crates/rustls/RUSTSEC-2026-{i}.md','date':'2026-10-14','patched':'>=0.23.46'} for i in range(346,350)]},'transport':'Original total case actually returned valid idleTimeout under scheduling jitter. Isolated total guard by giving only that fixture idle=1s; total100ms, independent idle30ms and production10/60/30/180s unchanged. All actual transport regression passed; original failure raw cases retained. New commit CI must be checked independently.'},
 'pending':['R4.3.06 actual edit/preview hookup next','R4.3.04 physical lock-screen Keychain and R4.2.03 actual IME candidate windows unverified; preserve system input settings','R4.4.12 full ContextSnapshot/Registry/UI/live pipeline later; no paid live API calls in this batch','65 other tasks plus all32 final gates and actual release remain required; dev only/no main merge/no old public tag or package changes'],
 'original_failures':['Swift explicit self.document capture/local variable name fixes; original pi-initial log retained','Swift HistoryFixtures duplicate recent local renamed; original regression log retained','Original exact HEAD CI failed jobs and actual total idle mismatch retained','Cache cleanup first process-output decode failed before mutation; fixed by reading process names without arguments'],
 'commands_file':'readonly-outputs-history-raw/commands.json','source_sha256':sources,'credential_scan':{'read_only_exact_authorized_file':True,'config_file_mode':'0600','config_directory_mode':'0700','files_scanned':scanned,'exact_key_matches':0,'key_not_copied':True},'raw_files_sha256':{str(p.relative_to(A)):sha(p.read_bytes()) for p in sorted(A.rglob('*')) if p.is_file()}}
write(D/'readonly-outputs-history.json',proof)
print(json.dumps({'raw_files':len(proof['raw_files_sha256']),'source_inputs':len(sources),'compressed_originals':len(compressed),'raw_bytes':sum(p.stat().st_size for p in A.rglob('*') if p.is_file()),'credential_matches':0}))
