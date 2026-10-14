from pathlib import Path
import gzip,hashlib,json,re,stat,subprocess,plistlib
R=Path.cwd();T=R/'target/acceptance/pre-alpha.4/development/r43';D=R/'docs/acceptance/pre-alpha.4/development/r43';A=D/'math-run-ledger-raw'
def sha(b):return hashlib.sha256(b).hexdigest()
def write(p,x):p.parent.mkdir(parents=True,exist_ok=True);p.write_text(json.dumps(x,ensure_ascii=False,indent=2)+'\n')
def report(log):
 rows=[]
 for line in (T/log).read_text().splitlines():
  try:
   x=json.loads(line)
   if isinstance(x,dict) and 'evidence_root' in x:rows.append(x)
  except ValueError:pass
 assert len(rows)==1,log
 return rows[0]
primary=report('run-ledger-complete-final.log');assert len(primary['cases'])==16
plan_check=json.loads((T/'run-ledger-plan-verified.log').read_text());assert plan_check['status']=='passed' and not plan_check['ready_to_publish'] and not plan_check['released_verified']
pi=report('run-ledger-signed-pi-verified.log')
rust=(T/'run-ledger-rust-regression.log').read_text();m=re.findall(r'test result: (\w+)\. (\d+) passed; (\d+) failed',rust);assert m and all(a=='ok' and int(c)==0 for a,b,c in m)
counts=dict(passed=sum(int(b) for a,b,c in m),failed=0,suites=len(m));assert counts['passed']==175
assert 'test result: ok. 4 passed; 0 failed' in (T/'run-ledger-cli-artifact.log').read_text()
assert '4374 events/>16MiB' in (T/'run-ledger-session-verified.log').read_text()
assert '60 actual canonical session events' in (T/'run-ledger-session-verified.log').read_text()
assert 'Five actual SIGKILL projection boundaries' in (T/'run-ledger-recovery-verified.log').read_text()
assert '** BUILD SUCCEEDED **' in (T/'run-ledger-release-verified.log').read_text()
assert 'satisfies its Designated Requirement' in (T/'run-ledger-signature-verified.log').read_text()
assert (T/'run-ledger-python.log').read_text().rstrip().endswith('OK')
compressed=[];A.mkdir(parents=True,exist_ok=True)
def copy(p,d,zip=False):
 assert p.is_file() and not p.is_symlink();b=p.read_bytes();d.parent.mkdir(parents=True,exist_ok=True)
 if zip:
  d=Path(str(d)+'.gz');z=gzip.compress(b,mtime=0);assert gzip.decompress(z)==b;d.write_bytes(z)
  compressed.append(dict(source=str(p.relative_to(R)),archive=str(d.relative_to(A)),original_bytes=len(b),original_sha256=sha(b)))
 else:d.write_bytes(b);assert d.read_bytes()==b
paths=subprocess.check_output(['git','diff','HEAD','--name-only'],text=True).splitlines()+subprocess.check_output(['git','ls-files','--others','--exclude-standard'],text=True).splitlines()
if (D/'math-run-ledger.json').is_file():paths+=list(json.loads((D/'math-run-ledger.json').read_text()).get('source_sha256',{}))
paths+=['Cargo.toml','Cargo.lock','macos/Scripts/test-session-ledger.sh','macos/Scripts/test-recovery.sh','macos/Scripts/build-native.sh','macos/Scripts/test-agent-package.sh','macos/OpenMathNativeTests/SessionLedgerFixtures/main.swift','macos/OpenMathNativeTests/SessionLedgerFixtures/interruption.py','macos/OpenMathNativeTests/RecoveryFixtures/main.swift','macos/OpenMathNativeTests/RecoveryFixtures/interruption.py','macos/OpenMathNative/Storage/KernelStore.swift','macos/OpenMathNative/Storage/DocumentOutbox.swift','docs/design/macos-storage.schema.json','crates/om-host-service/src/tools/run/execution.rs']
sources={}
for name in sorted(set(paths)):
 p=R/name
 if p.is_file() and not name.startswith('docs/acceptance/pre-alpha.4/development/r43/math-run-ledger'):
  copy(p,A/'inputs'/name);sources[name]=sha(p.read_bytes())
for folder in T.glob('run-ledger-*inputs'):
 if folder.is_dir():
  for p in sorted(folder.rglob('*')):
   if p.is_file():copy(p,A/folder.name/p.relative_to(folder))
for p in sorted(T.glob('run-ledger-*.log')):copy(p,A/'logs'/p.name,True)
for name in ['run-ledger-frozen-design-note.md','run-ledger-audit-before-repair.py','run-ledger-kill-expectation-before-repair.py','run-ledger-base-ci.json','archive-run-ledger.py','cleanup-run-ledger.py','run-ledger-cache-cleanup.json']:
 p=T/name
 if p.is_file():copy(p,A/name)
copy(T/'run-ledger-harness-inventory.jsonl',A/'harness-inventory.jsonl',True)
original_cli=subprocess.check_output(['git','show','1e98702bf4dfb21fb7095bc2ae54a7b7011a2eb4:crates/om-cli/tests/artifact.rs'])
(A/'original-ci-artifact.rs').write_bytes(original_cli)
original_roots={Path(primary['evidence_root'])}
for log in T.glob('run-ledger-*.log'):
 s=log.read_text()
 for value in re.findall(r'/Users/hert/Projects/openmath/target/macos-storage-tests/(run-ledger-[a-zA-Z0-9_-]+)',s):
  p=R/'target/macos-storage-tests'/value
  if p.is_dir():original_roots.add(p)
# Primary original tables/Blobs, completed intermediate runs, and every failed/crashed root.
for root in sorted(original_roots):
 assert root.parent==R/'target/macos-storage-tests' and root.name.startswith('run-ledger-')
 for p in sorted(root.rglob('*')):
  if p.is_file():copy(p,A/'original-run-roots'/root.name/p.relative_to(root),True)
piroot=Path(pi['evidence_root']);assert piroot.parent==R/'target/macos-agent-package-tests'
for p in sorted(piroot.rglob('*')):
 if p.is_file():copy(p,A/'signed-pi-original'/p.relative_to(piroot),True)
p=R/'target/acceptance/pre-alpha.4/development/r41/session-ledger-canonical-events.json'
copy(p,A/'session-canonical-events.json',True)
write(A/'compressed-originals.json',compressed)
write(A/'commands.json',[
 dict(command='python3 scripts/check_pre_alpha_4_plan.py --check --self-test',log='logs/run-ledger-plan-verified.log.gz',exit=0),
 dict(command='CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 cargo test --locked -p om-cli --test artifact -- --nocapture',log='logs/run-ledger-cli-artifact.log.gz',exit=0),
 dict(command='CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 cargo test --locked -p om-host-service -p om-apple-ffi',log='logs/run-ledger-rust-regression.log.gz',exit=0),
 dict(command='bash macos/Scripts/test-run-ledger.sh',log='logs/run-ledger-complete-final.log.gz',exit=0,cases=16),
 dict(command='bash macos/Scripts/test-session-ledger.sh',log='logs/run-ledger-session-verified.log.gz',exit=0),
 dict(command='CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 bash macos/Scripts/test-recovery.sh',log='logs/run-ledger-recovery-verified.log.gz',exit=0),
 dict(command='CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 cargo clippy --locked -p om-host-service --all-targets -- -D warnings',log='logs/run-ledger-clippy.log.gz',exit=0),
 dict(command='CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 bash macos/Scripts/build-native.sh',log='logs/run-ledger-release-verified.log.gz',exit=0),
 dict(command='bash macos/Scripts/test-agent-package.sh "target/macos-native/Build/Products/Release/OpenMath Preview.app/Contents/Resources/AgentRuntime"',log='logs/run-ledger-signed-pi-verified.log.gz',exit=0),
 dict(command='codesign --verify --deep --strict --verbose=2 "target/macos-native/Build/Products/Release/OpenMath Preview.app"',log='logs/run-ledger-signature-verified.log.gz',exit=0),
 dict(command='python3 -X warn_default_encoding -W error::EncodingWarning -m unittest discover -s tests -v',log='logs/run-ledger-python.log.gz',exit=0),
 dict(command='python3 scripts/native_contracts.py --check',log='logs/run-ledger-contracts.log.gz',exit=0),
 dict(command='python3 macos/Scripts/generate-project.py --check',log='logs/run-ledger-project.log.gz',exit=0)])
config=Path('/Users/hert/.config/openmath/testing/pre-alpha-4-api.json');private=json.loads(config.read_text());keys=[]
def visit(v):
 if isinstance(v,dict):
  for x in v.values():visit(x)
 elif isinstance(v,list):
  for x in v:visit(x)
 elif isinstance(v,str) and v.startswith('sk-'):keys.append(v.encode())
visit(private);assert keys and stat.S_IMODE(config.stat().st_mode)==0o600 and stat.S_IMODE(config.parent.stat().st_mode)==0o700
scanned=0
for p in A.rglob('*'):
 if p.is_file():
  b=p.read_bytes();b=gzip.decompress(b) if p.suffix=='.gz' else b;assert not any(k in b for k in keys),'Credential found';scanned+=1
app_root=R/'target/macos-native/Build/Products/Release/OpenMath Preview.app'
executable=plistlib.loads((app_root/'Contents/Info.plist').read_bytes())['CFBundleExecutable']
app=app_root/'Contents/MacOS'/executable
assert app.is_file()
proof=dict(schema_version=1,task='R4.3.07',status='in_progress',date='2026-10-15',base_commit=subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip(),
 implementation='One original run call/group claim, frozen source/config/parent plan, stable independently claimed bootstrap/children. Both durable admission/start ACKs precede callback; each result references only actual document outbox/Library effect/typed original receipt + original raw byte SHA. Linear checked receipt/progress chain; no synthetic aggregate kernel receipt.',
 verified=dict(plan_structure_and_links='passed after restoring the original frozen design; no baseline SHA or gate change',cli_artifact_tests=4,cli_fixed_clock_parallel_directories=16,swift_sqlite_run_cases=16,original_sigkill_boundaries=8,negative_protocol_cases=6,rust=counts,session_ledger='4374events/>16MiB/37calls+37requests/bounded5row pages/restart/60closed canonical events',source_kernel_save_recovery='real F_FULLFSYNC and five SIGKILL boundaries; no evaluation/network replay',active_group_history_recovery='RED actual bootstrap accepted then history recovery prematurely finished live group; GREEN memory-only weak lifetime prevents it, abandoned producer loses liveness and restart still restores no authority',release_app_build=True,strict_signature=True,clippy_all_targets=True,python_tests=17,native_dtos=389,signed_pi_report=pi,release_app_macho_sha256=sha(app.read_bytes()) if app.is_file() else None),
 primary_run_report=primary,
 original_failures=['Implementation notes were appended to a frozen design source; final plan checker caught SHA drift. Restored original frozen document, retained implementation in D063 and original note/failure log, then reran the checker without changing baseline SHA or runtime gates','Base CI om-cli artifact temp directory EEXIST due to concurrent equal clock readings; original remote job log retained; isolated counter/atomic-create allocation and deterministic equal-clock concurrency test added without changing export expectations','Initial Swift throwing-expression/HostSerial.value/private fixture type compilation diagnostics retained with source inputs','Original actual bootstrap receipt was rejected because canonical sorted re-encoding was compared to raw persisted outbox bytes; fixed typed comparison while retaining exact original byte hashes; full failed SQLite/Blob/root retained','Python audit initially queried a nonexistent source_transactions table; corrected to original transactions table without changing app storage','Kill after group started but before any child claim legitimately closes as interrupted/failed with no effect; original audit expected unknown and was corrected to actual durable state','Concurrent storage recovery after bootstrap but before next child ended a live group: original real RED root/log retained; added ephemeral weak liveness only, no serialized grant/executor'],
 limitations=['New durable run port and Rust RunExecutor are separately verified; actual original Agent producer/C ABI/Pi run dispatch and scoped result refs remain required','Physical run fixture invokes real Rust candidate preparation only after durable claims and uses actual Swift SQLite publication/receipts; its ownerBarrier closure is deliberately empty, so it does not replace Rust live acceptance-barrier integration tests','Signed Pi compatibility regression covers existing readonly/patch/inspect/scratch flows, not new run dispatch; AgentSessionService still reports RunCells unavailable','R4.3.07 Steps1/2/4 and task Done remain unchecked; overall34/98, all32 final gates not_run, candidate unset/ready false/released false','No subagents, OS input settings changes, local iOS simulator or paid API calls'],
 source_sha256=sources,commands_file='math-run-ledger-raw/commands.json',original_run_roots=[str(x.relative_to(R)) for x in sorted(original_roots)],
 credential_scan=dict(read_only_exact_authorized_file=True,file_mode='0600',directory_mode='0700',files_scanned=scanned,exact_key_matches=0,key_not_copied=True),
 ci_observation=json.loads((T/'run-ledger-base-ci.json').read_text()),
 raw_files_sha256={str(p.relative_to(A)):sha(p.read_bytes()) for p in sorted(A.rglob('*')) if p.is_file()})
p=T/'run-ledger-cache-cleanup.json'
if p.is_file():proof['verified']['cache_cleanup']=json.loads(p.read_text())
write(D/'math-run-ledger.json',proof)
print(json.dumps(dict(source_inputs=len(sources),raw_files=len(proof['raw_files_sha256']),raw_bytes=sum(p.stat().st_size for p in A.rglob('*') if p.is_file()),run_cases=16,rust=counts,credential_matches=0)))
