from pathlib import Path
import json,hashlib,stat,struct,subprocess,sys
R=Path.cwd();T=R/'target/acceptance/pre-alpha.4/development/r43';deps=(R/'target/debug/deps').resolve()
assert 'test result: FAILED' not in (T/'run-ledger-rust-regression.log').read_text()
assert 'test result: ok. 13 passed; 0 failed' in (T/'run-ledger-rust-regression.log').read_text()
paths=set()
for line in (T/'run-ledger-harness-inventory.jsonl').read_text().splitlines():
 try:x=json.loads(line)
 except ValueError:continue
 if x.get('reason')=='compiler-artifact' and x.get('profile',{}).get('test') and x.get('executable'):
  paths.add(x['executable'])
records=[]
for s in sorted(paths):
 p=Path(s);a=p.lstat();assert p.parent.resolve()==deps and p.parent==deps and stat.S_ISREG(a.st_mode) and not p.is_symlink()
 with p.open('rb') as f:h=f.read(32)
 assert h[:4]==b'\xcf\xfa\xed\xfe' and struct.unpack('<I',h[12:16])[0]==2,(s,'not ordinary arm64 Mach-O executable')
 records.append(dict(path=str(p.relative_to(R)),bytes=a.st_size,sha256=hashlib.sha256(p.read_bytes()).hexdigest(),inode=a.st_ino,device=a.st_dev))
assert records
out=dict(schema_version=1,kind='successful_test_profile_ordinary_macho_executable_only',files=len(records),logical_bytes=sum(x['bytes'] for x in records),records=records,deleted=False)
if '--delete' in sys.argv:
 assert '** BUILD SUCCEEDED **' in (T/'run-ledger-release-verified.log').read_text()
 assert '"evidence_root"' in (T/'run-ledger-signed-pi-verified.log').read_text()
 assert 'satisfies its Designated Requirement' in (T/'run-ledger-signature-verified.log').read_text()
 processes=subprocess.check_output(['ps','-axo','comm='],text=True)
 assert not any('/target/debug/deps/' in l or l.strip().rsplit('/',1)[-1] in ('cargo','rustc','rustdoc') for l in processes.splitlines()),'live Rust test/compiler; preserve cache'
 for x in records:
  p=R/x['path'];a=p.lstat();assert a.st_ino==x['inode'] and a.st_dev==x['device'] and a.st_size==x['bytes'] and hashlib.sha256(p.read_bytes()).hexdigest()==x['sha256'];p.unlink()
 assert all(not (R/x['path']).exists() for x in records)
 out['deleted']=True
out['preserved']=['all rlib/rmeta/static libraries and object caches','CLI/examples','release library/App/runtime','evidence and failed originals','user data and credentials']
(T/'run-ledger-cache-cleanup.json').write_text(json.dumps(out,ensure_ascii=False,indent=2)+'\n')
print(json.dumps({k:out[k] for k in ['files','logical_bytes','deleted']}))
