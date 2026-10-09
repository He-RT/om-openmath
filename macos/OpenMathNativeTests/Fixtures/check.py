"""Verify sealed fixture bytes and real loopback/process failures; no business/CAS stub."""
from pathlib import Path
import hashlib, json, subprocess, sys, tempfile, urllib.error, urllib.request, signal
ROOT=Path(__file__).resolve().parent
manifest=json.loads((ROOT/'manifest.json').read_text(encoding='utf-8'))
for entry in manifest['files']:
 data=(ROOT/entry['path']).read_bytes()
 assert hashlib.sha256(data).hexdigest()==entry['sha256'],entry['path']
class RejectRedirect(urllib.request.HTTPRedirectHandler):
 def redirect_request(self,*args,**kwargs):return None
server=subprocess.Popen([sys.executable,str(ROOT/'Network/server.py')],stdout=subprocess.PIPE,text=True,encoding='utf-8')
try:
 ready=json.loads(server.stdout.readline());base=f'http://127.0.0.1:{ready["port"]}'
 opener=urllib.request.build_opener(urllib.request.ProxyHandler({}))
 data=opener.open(base+'/stream',timeout=2).read()
 assert data==(ROOT/'Network/utf8-stream.sse').read_bytes()
 assert '中文🙂 π' in data.decode('utf-8')
 for status in (401,429,500):
  try:opener.open(base+f'/status/{status}',timeout=2);raise AssertionError('HTTP fault faked success')
  except urllib.error.HTTPError as error:assert error.code==status
 try:urllib.request.build_opener(urllib.request.ProxyHandler({}),RejectRedirect()).open(base+'/redirect',timeout=2);raise AssertionError('redirect was accepted')
 except urllib.error.HTTPError as error:assert error.code==302
 try:opener.open(base+'/timeout',timeout=0.05);raise AssertionError('timeout was faked success')
 except TimeoutError:pass
finally:server.terminate();server.wait(timeout=5)
work=Path.cwd()/'target/native-fixture-attempts';work.mkdir(parents=True,exist_ok=True)
records=[]
for stage in ('opened','written','file_synced','renamed','directory_synced','enospc'):
 root=Path(tempfile.mkdtemp(prefix=stage+'-',dir=work))
 child=subprocess.Popen([sys.executable,str(ROOT/'IO/interrupted_child.py'),str(root),stage],stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True,encoding='utf-8')
 if stage=='enospc':
  out,error=child.communicate(timeout=5);assert child.returncode!=0 and not (root/'active.json').exists()
 else:
  lines=[]
  while True:
   line=child.stdout.readline();assert line,'child exited before requested kill point';lines.append(json.loads(line))
   if lines[-1]['stage']==stage:break
  child.kill();child.wait(timeout=5);assert child.returncode==-signal.SIGKILL
  assert (root/'active.json').exists()==(stage in ('renamed','directory_synced'))
 records.append({'stage':stage,'root':str(root),'exit_code':child.returncode,'files':[p.name for p in root.iterdir()]})
(work/'attempt.json').write_text(json.dumps(records,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print('Fixture hashes, actual UTF-8 HTTP bytes/errors/redirect rejection/timeout and six real interruption points passed')
