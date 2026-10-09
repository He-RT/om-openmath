"""Kill the real SQLite bootstrap at actual host fault boundaries; no fake durable receipts."""
from pathlib import Path
import hashlib, json, os, selectors, signal, subprocess, tempfile, time
BASE=Path.cwd()/'target/macos-storage-tests'
HELPER=BASE/'storage-fixtures'
ENV={**os.environ,'DYLD_INSERT_LIBRARIES':str(BASE/'libStorageSyncProbe.dylib')}
records=[]
def run(root,mode):
 p=subprocess.run([str(HELPER),str(root),mode],env=ENV,capture_output=True,text=True,encoding='utf-8',timeout=10)
 assert p.returncode==0,(mode,p.returncode,p.stderr)
 return p.stdout.strip()
def stopped(root,stage):
 p=subprocess.Popen([str(HELPER),str(root),stage],env=ENV,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True,encoding='utf-8')
 ready=selectors.DefaultSelector();ready.register(p.stdout,selectors.EVENT_READ)
 if not ready.select(10):
  p.kill();raise AssertionError('real bootstrap did not reach '+stage)
 line=p.stdout.readline().strip();ready.close()
 expected="lease-held" if stage=="hold-open" else stage
 if line!=expected:
  p.kill();p.wait(timeout=5);raise AssertionError((line,stage))
 return p
for stage in ('root_published','before_bootstrap_commit','bootstrap_committed','before_selector_publish','selector.renamed','selector_published'):
 root=Path(tempfile.mkdtemp(prefix='crash-',dir=BASE))
 p=stopped(root,stage);p.kill();p.wait(timeout=5)
 assert p.returncode==-signal.SIGKILL
 if stage=='root_published':
  info=json.loads(run(root,'inspect'));outcome='new-library-no-prior-database'
 elif stage in ('selector.renamed','selector_published'):
  selector=json.loads((root/'Library/active.json').read_text(encoding='utf-8'))
  info=json.loads(run(root,'inspect'));assert info['identity']['store_id']==selector['store_id'];outcome='same-selected-generation'
 else:
  assert run(root,'check-recovery')=='recovery_required';outcome='unselected-database-preserved'
 records.append({'stage':stage,'exit_code':p.returncode,'outcome':outcome,'root':str(root)})
root=Path(tempfile.mkdtemp(prefix='cross-process-',dir=BASE))
p=stopped(root,'hold-open')
second=subprocess.run([str(HELPER),str(root),'inspect'],env=ENV,capture_output=True,text=True,encoding='utf-8',timeout=10)
assert second.returncode!=0 and 'inUse' in second.stderr
p.kill();p.wait(timeout=5)
info=json.loads(run(root,'inspect'))
records.append({'stage':'cross-process-root-lease','second_exit':second.returncode,'after_process_death':'opened selected store'})
(BASE/'interruption-attempt.json').write_text(json.dumps(records,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print('Real SQLite crash/restart: six boundaries, no unselected generation guessed, selector identity retained, process-owned lock released only on death')
