from pathlib import Path
import json,hashlib,gzip,subprocess,re,stat
R=Path.cwd();T=R/'target/acceptance/pre-alpha.4/development/r43';D=R/'docs/acceptance/pre-alpha.4/development/r43';A=D/'math-run-executor-raw'
def sha(b):return hashlib.sha256(b).hexdigest()
def write(p,v):
 p.parent.mkdir(parents=True,exist_ok=True);p.write_text(json.dumps(v,ensure_ascii=False,indent=2)+'\n')
def count(name):
 s=(T/name).read_text(); rows=re.findall(r'test result: (\w+)\. (\d+) passed; (\d+) failed',s);assert rows and all(a=='ok' and int(c)==0 for a,b,c in rows),name
 return dict(passed=sum(int(b) for a,b,c in rows),failed=0,suites=len(rows))
regression=count('run-executor-regression.log');assert regression['passed']==398
facts=count('run-executor-real-facts.log');assert facts['passed']==13
assert 'Finished `dev`' in (T/'run-executor-clippy.log').read_text()
assert 'Ran 17 tests' in (T/'run-executor-python-corrected.log').read_text() and (T/'run-executor-python-corrected.log').read_text().rstrip().endswith('OK')
assert 'Actual C ABI main kernel + shared source gate + Swift/SQLite:' in (T/'run-executor-native-kernel.log').read_text()
compressed=[];A.mkdir(parents=True,exist_ok=True)
def copy(src,dst,compress=False):
 assert src.is_file() and not src.is_symlink();b=src.read_bytes();dst.parent.mkdir(parents=True,exist_ok=True)
 if compress:
  dst=Path(str(dst)+'.gz');z=gzip.compress(b,mtime=0);assert gzip.decompress(z)==b;dst.write_bytes(z)
  compressed.append(dict(source=str(src.relative_to(R)),archive=str(dst.relative_to(A)),original_bytes=len(b),original_sha256=sha(b)))
 else:dst.write_bytes(b);assert dst.read_bytes()==b
paths=subprocess.check_output(['git','diff','HEAD','--name-only'],text=True).splitlines()+['crates/om-host-service/src/tools/run/execution.rs','crates/om-host-service/tests/math_run_execution.rs']
paths+=['Cargo.toml','Cargo.lock','crates/om-host-service/build.rs','crates/om-host-service/examples/kernel_store_contract.rs','macos/Scripts/test-kernel-runtime.sh','macos/OpenMathNativeTests/KernelRuntimeFixtures/main.swift','macos/OpenMathNative/Kernel/KernelCommitPort.swift','macos/OpenMathNative/Storage/KernelStore.swift']
sources={}
for s in sorted(set(paths)):
 p=R/s
 if p.is_file() and '/math-run-executor-raw/' not in s and not s.endswith('/math-run-executor.json'):
  copy(p,A/'inputs'/s);sources[s]=sha(p.read_bytes())
for folder in ['run-executor-initial-inputs','run-executor-final-inputs']:
 for p in sorted((T/folder).rglob('*')):
  if p.is_file():copy(p,A/folder/p.relative_to(T/folder))
for p in sorted(T.glob('run-executor-*.log')):copy(p,A/'logs'/p.name,True)
copy(T/'run-base-ci.json',A/'run-base-ci.json')
copy(T/'archive-run-executor.py',A/'archive-run-executor.py')
copy(T/'cleanup-run-executor.py',A/'cleanup-run-executor.py')
copy(T/'run-executor-harness-inventory.jsonl',A/'harness-inventory.jsonl',True)
start=(T/'run-executor-native-kernel.log').stat().st_birthtime
roots=[p for p in (R/'target/macos-storage-tests').glob('kernel-runtime-*') if p.is_dir() and p.name!='kernel-runtime-source' and p.stat().st_birthtime>=start]
assert len(roots)==1,[(p.name,p.stat().st_birthtime) for p in roots]
for p in sorted(roots[0].rglob('*')):
 if p.is_file():copy(p,A/'native-kernel-original'/p.relative_to(roots[0]),True)
copy(R/'target/macos-storage-tests/kernel-runtime-source/source.json',A/'native-kernel-source.json')
cleanup=T/'run-executor-cache-cleanup.json'
if cleanup.exists():copy(cleanup,A/'cache-cleanup.json')
write(A/'compressed-originals.json',compressed)
write(A/'commands.json',[
 {'command':'CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 cargo test --locked -p om-host-service --test math_run_execution -- --nocapture','log':'logs/run-executor-initial.log.gz','exit':0,'original_inputs':'run-executor-initial-inputs','tests':8},
 {'command':'CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 cargo test --locked -p om-host-service --test math_run_execution --test math_run_plan -- --nocapture','log':'logs/run-executor-final.log.gz','exit':0,'tests':16,'note':'intermediate 12 executor +4 planning tests; final unknown-race case added afterward'},
 {'command':'CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 cargo test --locked -p om-kernel -p om-host-service -p om-apple-ffi','log':'logs/run-executor-regression.log.gz','exit':0},
 {'command':'CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 cargo test --locked -p om-host-service --test math_run_execution -- --nocapture','log':'logs/run-executor-real-facts.log.gz','exit':0},
 {'command':'CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 cargo clippy --locked -p om-kernel -p om-host-service -p om-apple-ffi --all-targets -- -D warnings','log':'logs/run-executor-clippy.log.gz','exit':0},
 {'command':'CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 bash macos/Scripts/test-kernel-runtime.sh','log':'logs/run-executor-native-kernel.log.gz','exit':0,'note':'actual existing C ABI/KernelCommitPort/SQLite regression; new RunExecutor is not yet wired to Swift/Pi'},
 {'command':'python3 -X warn_default_encoding -W error::EncodingWarning -m unittest discover -s tests -v','log':'logs/run-executor-python-corrected.log.gz','exit':0},
 {'command':'python3 -m unittest discover -s scripts/tests','log':'logs/run-executor-python.log.gz','exit':1,'note':'wrong discovery directory; corrected command above'},
 {'command':'python3 macos/Scripts/project.py --check','log':'logs/run-executor-project.log.gz','exit':2,'note':'wrong script path; corrected generate-project.py check passed'},
 {'command':'python3 scripts/check_pre_alpha_4_plan.py --check --self-test','log':'logs/run-executor-plan-final.log.gz','exit':1,'note':'proof link written before its evidence file existed; final proof/check written after terminal checks'}])
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
proof=dict(schema_version=1,task='R4.3.07',status='in_progress',date='2026-10-15',base_commit=subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip(),
 implementation='Frozen actual read-issued plan -> explicit stable bootstrap/child admission boundary -> real main KernelEndpoint/KernelWorker -> original candidate/hash/final scope barrier/receipt chain. No RunAll, report rehydration, new-ID unknown replay or source-patch grant requirement.',
 verified=dict(rust_regression=regression,executor_real_facts=facts,clippy_all_targets=True,python_tests=17,native_dtos=389,existing_native_kernel_sqlite_regression=True,native_kernel_evidence_root=str(roots[0].relative_to(R))),
 cases=['required actual a2/b3/answer4 topological receipts/revisions, unrelated source not run','selected refuses unready real prerequisites with reactive=false and continue=true still runs independent branch','affected actual descendants rerun under a distinct model-call operation','real recursion error retains 2 successful statements in accepted main checkpoint; dependent blocked and independent follows original continue_on_error','stable AwaitingAdmission does no dispatch before native acknowledgment; wrong child rejected; cancel before acknowledgment never creates operation','actual cycle vs blocked descendant separate; independent scope follows original error policy','source head changed between real accepted child boundaries preserves original prefix and refuses next admission','final barrier rechecks live task/grant/editor/mode/source/config/definition','pre-barrier cancel discards; after physical boundary original receipt still retained','unknown is latched immediately including receipt-before-poll race, keeps original child and refuses later admissions; final original receipt may prove complete','duplicate original group cannot shadow bootstrap/children or invent a different child barrier'],
 limitations=['New RunExecutor Rust tests use matching synthetic IO receipt acknowledgments, explicitly labeled; they do not claim durable group, Swift/Pi execution or Agent result refs','Existing C ABI/Swift/SQLite regression uses real physical storage/receipt recovery; it exercises existing manual math entry points, not the still-unwired RunExecutor','Swift durable group/child admission-start/outcome records and recovery, actual original Agent producer/C ABI/Pi dispatch, Agent-scope result refs remain required','R4.3.07 Step1/2/4 and Done remain unchecked; overall34/98, 32final gates not_run, no candidate/ready/release; no subagents, OS input changes, local iOS simulator or paid API tests'],
 source_sha256=sources,commands_file='math-run-executor-raw/commands.json',
 credential_scan=dict(read_only_exact_authorized_file=True,file_mode='0600',directory_mode='0700',files_scanned=scanned,exact_key_matches=0,key_not_copied=True),
 ci_observation=json.loads((T/'run-base-ci.json').read_text()),
 raw_files_sha256={str(p.relative_to(A)):sha(p.read_bytes()) for p in sorted(A.rglob('*')) if p.is_file()})
if cleanup.exists():proof['verified']['cache_cleanup']=json.loads(cleanup.read_text())
write(D/'math-run-executor.json',proof)
print(json.dumps(dict(rust=regression,executor=facts,source_inputs=len(sources),raw_files=len(proof['raw_files_sha256']),raw_bytes=sum(p.stat().st_size for p in A.rglob('*') if p.is_file()),credential_matches=0)))
