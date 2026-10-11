"""Real TextKit marked-text snapshot COMMIT/ACK interruption, with no source replay."""
from pathlib import Path
import json,os,select,signal,sqlite3,subprocess,tempfile,time
ROOT=Path(__file__).resolve().parents[3];os.chdir(ROOT)
binary=ROOT/'target/macos-storage-tests/draft-fixtures'
env=os.environ.copy();env['DYLD_INSERT_LIBRARIES']=str(ROOT/'target/macos-storage-tests/libDraftSyncProbe.dylib')
records=[]
for stage in ['draft_backup_before_commit','draft_backup_committed','draft_backup_readback']:
    root=Path(tempfile.mkdtemp(prefix='draft-kill-',dir=ROOT/'target/macos-storage-tests'))
    child=subprocess.Popen([str(binary),str(root),'crash',stage],stdout=subprocess.PIPE,stderr=subprocess.STDOUT,env=env)
    output=b'';deadline=time.monotonic()+45
    while (stage+'\n').encode() not in output and time.monotonic()<deadline:
        ready,_,_=select.select([child.stdout],[],[],0.25)
        if ready:
            part=os.read(child.stdout.fileno(),4096)
            if not part:break
            output+=part
        if child.poll() is not None:break
    if (stage+'\n').encode() not in output:
        if child.poll() is None:child.kill()
        child.wait();raise AssertionError((stage,output.decode(errors='replace')))
    assert child.poll() is None
    assert 'T' in subprocess.check_output(['ps','-o','stat=','-p',str(child.pid)],text=True)
    child.kill();assert child.wait()==-signal.SIGKILL
    result=subprocess.run([str(binary),str(root),'recover'],capture_output=True,text=True,env=env,timeout=45)
    assert result.returncode==0,(stage,result.stdout,result.stderr)
    if stage=='draft_backup_before_commit':assert 'No draft COMMIT' in result.stdout
    else:assert 'Original draft COMMIT recovered' in result.stdout
    database=root/'Documents/00000000-0000-0000-0000-000000000512/Generations/000001/authority.sqlite'
    db=sqlite3.connect(database)
    assert db.execute('select revision from document_head').fetchone()[0]==0
    assert db.execute('select count(*) from operations').fetchone()[0]==0
    assert db.execute('select count(*) from accepted_checkpoints').fetchone()[0]==0
    drafts=db.execute('select count(*) from draft_batches').fetchone()[0];db.close()
    assert drafts==(0 if stage=='draft_backup_before_commit' else 1)
    records.append({'boundary':stage,'process_exit':-signal.SIGKILL,'root':str(root.relative_to(ROOT)),'draft_batches':drafts,'source_revision':0,'source_operations':0,'accepted_checkpoints':0})
out=ROOT/'target/acceptance/pre-alpha.4/development/r41';out.mkdir(parents=True,exist_ok=True)
(out/'draft-interruptions.json').write_text(json.dumps(records,ensure_ascii=False,indent=2)+'\n')
print('Three real draft SIGKILL boundaries: absent or exact original marked-text COMMIT, unchanged source and zero evaluation')
