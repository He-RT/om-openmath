"""Real BlobStore publication SIGKILL points; no database can acquire an unpublished reference."""
from pathlib import Path
import json, selectors, signal, subprocess, tempfile
ROOT=Path.cwd();BASE=ROOT/'target/macos-storage-tests';HELPER=BASE/'blob-fixtures';FIXTURES=ROOT/'macos/OpenMathNativeTests/Fixtures'
records=[]
for stage in ('blob_stage_opened','blob_copied','blob_file_synced','before_blob_publish','blob_linked','blob_directory_synced'):
 root=Path(tempfile.mkdtemp(prefix='blob-crash-',dir=BASE))
 child=subprocess.Popen([str(HELPER),str(root),str(FIXTURES),stage],stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True,encoding='utf-8')
 ready=selectors.DefaultSelector();ready.register(child.stdout,selectors.EVENT_READ)
 if not ready.select(10):
  child.kill();child.wait(timeout=5);raise AssertionError('did not reach '+stage)
 line=child.stdout.readline().strip();ready.close()
 if line!=stage:
  child.kill();child.wait(timeout=5);raise AssertionError((line,stage))
 child.kill();child.wait(timeout=5);assert child.returncode==-signal.SIGKILL
 read=subprocess.run([str(HELPER),str(root),str(FIXTURES),'inspect-crash'],capture_output=True,text=True,encoding='utf-8',timeout=10)
 assert read.returncode==0,(stage,read.stderr)
 expected='complete-unreferenced-bytes' if stage in ('blob_linked','blob_directory_synced') else 'missing-no-ready-reference'
 assert read.stdout.strip()==expected,(stage,read.stdout)
 records.append({'stage':stage,'root':str(root),'exit':child.returncode,'after_restart':expected})
(BASE/'blob-interruption-attempt.json').write_text(json.dumps(records,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print('Six actual BlobStore publication crashes: no partial target, complete orphan only after link, zero references/pins on restart')
