"""Real Library/source COMMITs and actual URLSession loopback dispatch; no paid provider."""
from pathlib import Path
import hashlib,json,os,select,shutil,signal,sqlite3,subprocess,tempfile,threading,time
from http.server import BaseHTTPRequestHandler,ThreadingHTTPServer

ROOT=Path(__file__).resolve().parents[3];os.chdir(ROOT)
BINARY=ROOT/'target/macos-storage-tests/session-ledger-fixtures'
PLANS=ROOT/'target/macos-storage-tests/session-ledger-plans.json'
plan=json.loads(PLANS.read_text());document=plan['initial']['document_id']
lock=threading.Lock();calls={};bodies={}
class Fixture(BaseHTTPRequestHandler):
    def do_POST(self):
        length=int(self.headers['Content-Length']);assert 0<length<=2*1024*1024
        body=self.rfile.read(length);value=json.loads(body)
        with lock:calls[self.path]=calls.get(self.path,0)+1;bodies[self.path]=body
        failing=value.get('fixture_failure') is True
        response=b'provider unavailable' if failing else 'fixture答复🙂'.encode()
        self.send_response(503 if failing else 200);self.send_header('Content-Length',str(len(response)));self.end_headers();self.wfile.write(response)
    def log_message(self,*args):pass
server=ThreadingHTTPServer(('127.0.0.1',0),Fixture)
thread=threading.Thread(target=server.serve_forever,daemon=True);thread.start()
env=os.environ.copy();env['DYLD_INSERT_LIBRARIES']=str(ROOT/'target/macos-storage-tests/libSessionLedgerSyncProbe.dylib')
base=f'http://127.0.0.1:{server.server_port}'
stages={
 'session_event_before_commit.tool_admitted':('tool_not_started',0),
 'session_event_committed.tool_admitted':('tool_outcome_unknown',0),
 'tool_admission_acknowledged':('tool_outcome_unknown',0),
 'tool_execution_claimed':('tool_outcome_unknown',0),
 'source_committed':('completed',1),
 'tool_effect_returned':('completed',1),
 'session_event_before_commit.tool_result':('completed',1),
 'session_event_committed.tool_result':('completed',1),
 'request_plan_acknowledged':('not_dispatched',0),
 'request_dispatch_acknowledged':('dispatch_outcome_unknown',0),
 'request_response_observed':('dispatch_outcome_unknown',1),
 'request_response_acknowledged':('response_observed',1),
 'request_finish_acknowledged':('finished',1),
}
records=[]
canonical_events=[]
def validate_events(root):
    db=sqlite3.connect(root/'Library/Generations/000001/library.sqlite')
    for (record,) in db.execute('select record from session_events'):
        data=json.loads(record);canonical_events.append(data)
        assert hashlib.sha256(data['payload_inline_json'].encode()).hexdigest()==data['payload_hash']
    db.close()
try:
    normal=Path(tempfile.mkdtemp(prefix='session-ledger-',dir=ROOT/'target/macos-storage-tests'))
    normal_env={**env,'OPENMATH_SESSION_FIXTURE_URL':base+'/normal'}
    ordinary=subprocess.run([str(BINARY),str(PLANS),str(normal)],capture_output=True,text=True,env=normal_env,timeout=60)
    assert ordinary.returncode==0,(ordinary.stdout,ordinary.stderr)
    assert calls.get('/normal',0)==2;print(ordinary.stdout.strip())
    validate_events(normal)
    damaged=Path(tempfile.mkdtemp(prefix='session-ledger-corrupt-',dir=ROOT/'target/macos-storage-tests'))
    shutil.copytree(normal,damaged,dirs_exist_ok=True)  # Closed disposable branch; not a product backup.
    db=sqlite3.connect(damaged/'Library/Generations/000001/library.sqlite')
    db.execute('update session_events set record=? where event_sequence=2',(b'full damaged record',));db.commit();db.close()
    rejected=subprocess.run([str(BINARY),str(PLANS),str(damaged),'damage-check'],capture_output=True,text=True,env=normal_env,timeout=40)
    assert rejected.returncode==0,(rejected.stdout,rejected.stderr);print(rejected.stdout.strip())
    for stage,(expected,effects) in stages.items():
        root=Path(tempfile.mkdtemp(prefix='session-ledger-kill-',dir=ROOT/'target/macos-storage-tests'))
        request_path='/'+stage;child_env={**env,'OPENMATH_SESSION_FIXTURE_URL':base+request_path}
        child=subprocess.Popen([str(BINARY),str(PLANS),str(root),'crash',stage],stdout=subprocess.PIPE,stderr=subprocess.STDOUT,env=child_env)
        output=b'';deadline=time.monotonic()+40
        while (stage+'\n').encode() not in output and time.monotonic()<deadline:
            ready,_,_=select.select([child.stdout],[],[],0.25)
            if ready:
                part=os.read(child.stdout.fileno(),4096)
                if not part:break
                output+=part
            if child.poll() is not None:break
        if (stage+'\n').encode() not in output:
            if child.poll() is None:child.kill()
            child.wait();raise AssertionError((stage,'actual boundary missing',output.decode(errors='replace')))
        assert child.poll() is None
        assert 'T' in subprocess.check_output(['ps','-o','stat=','-p',str(child.pid)],text=True)
        child.kill();assert child.wait()==-signal.SIGKILL
        before_calls=calls.get(request_path,0)
        snapshots=[]
        for _ in range(2):
            recovery=subprocess.run([str(BINARY),str(PLANS),str(root),'recover'],capture_output=True,text=True,env=child_env,timeout=40)
            assert recovery.returncode==0,(stage,recovery.stdout,recovery.stderr)
            snapshots.append(json.loads(recovery.stdout))
        assert snapshots[0]['after']==snapshots[1]['after']
        if stage.startswith('request_'):
            assert snapshots[0]['requests']==[{'context':'context-crash','phase':expected}]
            assert calls.get(request_path,0)==effects==before_calls
            assert snapshots[0]['source_revision']==0
        else:
            assert snapshots[0]['after']==expected and snapshots[0]['source_revision']==effects
            assert before_calls==0 and calls.get(request_path,0)==0
        db=sqlite3.connect(root/'Documents'/document/'Generations/000001/authority.sqlite')
        assert db.execute('select count(*) from operations').fetchone()[0]==(0 if stage.startswith('request_') else effects)
        db.close()
        library=sqlite3.connect(root/'Library/Generations/000001/library.sqlite')
        count=library.execute('select count(*) from session_events').fetchone()[0]
        assert count==snapshots[0]['events']==snapshots[1]['events']
        assert library.execute('pragma foreign_key_check').fetchall()==[];library.close()
        validate_events(root)
        records.append({'boundary':stage,'root':str(root.relative_to(ROOT)),'exit_code':-signal.SIGKILL,'recovery':snapshots[0],'session_events_after_repeated_recovery':count,'actual_http_calls_before':before_calls,'actual_http_calls_after':calls.get(request_path,0),'wire_sha256':hashlib.sha256(bodies[request_path]).hexdigest() if request_path in bodies else None})
finally:
    server.shutdown();server.server_close();thread.join(timeout=3)
out=ROOT/'target/acceptance/pre-alpha.4/development/r41';out.mkdir(parents=True,exist_ok=True)
(out/'session-ledger-interruptions.json').write_text(json.dumps(records,ensure_ascii=False,indent=2)+'\n')
(out/'session-ledger-canonical-events.json').write_text(json.dumps(canonical_events,ensure_ascii=False,indent=2)+'\n')
print('13 actual SIGKILL session/tool/source/URLSession dispatch boundaries: no duplicate source effect or HTTP request, original completed facts recovered, not-started/unknown retained')
