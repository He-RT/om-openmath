"""Actual SIGKILL between the two independent durable stores; never replay source or CAS."""
from pathlib import Path
import hashlib,json,os,select,shutil,signal,sqlite3,subprocess,tempfile,time

ROOT=Path(__file__).resolve().parents[3]
os.chdir(ROOT)
BINARY=ROOT/'target/macos-storage-tests/recovery-fixtures'
ENV=os.environ.copy()
ENV['DYLD_INSERT_LIBRARIES']=str(ROOT/'target/macos-storage-tests/libRecoverySyncProbe.dylib')
STAGES=['library_projection_before_commit','library_projection_committed','library_projection_confirmed','document_projection_before_commit','document_projection_committed']
records=[]
for stage in STAGES:
    root=Path(tempfile.mkdtemp(prefix='recovery-kill-',dir=ROOT/'target/macos-storage-tests'))
    child=subprocess.Popen([str(BINARY),str(root),'crash',stage],stdout=subprocess.PIPE,stderr=subprocess.STDOUT,env=ENV)
    output=b'';deadline=time.monotonic()+50
    while (stage+'\n').encode() not in output and time.monotonic()<deadline:
        ready,_,_=select.select([child.stdout],[],[],0.25)
        if ready:
            block=os.read(child.stdout.fileno(),4096)
            if not block:break
            output+=block
        if child.poll() is not None:break
    if (stage+'\n').encode() not in output:
        if child.poll() is None:child.kill()
        child.wait();raise AssertionError((stage,'real boundary missing',output.decode(errors='replace')))
    assert child.poll() is None
    status=subprocess.check_output(['ps','-o','stat=','-p',str(child.pid)],text=True).strip()
    assert 'T' in status,(stage,status)
    child.kill();assert child.wait()==-signal.SIGKILL
    manifest=json.loads((root/'manifest.json').read_bytes())
    folder=root/'managed/Documents'/manifest['documentID']/'Generations/000001'
    db=sqlite3.connect(folder/'authority.sqlite')
    source_before=db.execute('select revision,snapshot_hash from document_head').fetchone()
    kernel_before=db.execute('select count(*) from accepted_checkpoints').fetchone()[0]
    pending_before=db.execute('select projected from outbox').fetchone()[0]
    assert kernel_before==5  # Initial checkpoint plus the four independently checked outputs.
    assert pending_before==(1 if stage=='document_projection_committed' else 0)
    db.close()
    lib=sqlite3.connect(root/'managed/Library/Generations/000001/library.sqlite')
    events_before=lib.execute('select count(*) from document_effect_events').fetchone()[0];lib.close()
    assert events_before==(0 if stage=='library_projection_before_commit' else 1)
    saved=root/'user-files/恢复🙂.omnb';before=hashlib.sha256(saved.read_bytes()).hexdigest()
    for _ in range(2):
        recovered=subprocess.run([str(BINARY),str(root),'recover'],capture_output=True,text=True,env=ENV,timeout=50)
        assert recovered.returncode==0,(stage,recovered.stdout,recovered.stderr)
    db=sqlite3.connect(folder/'authority.sqlite')
    assert db.execute('select revision,snapshot_hash from document_head').fetchone()==source_before
    assert db.execute('select count(*) from accepted_checkpoints').fetchone()[0]==kernel_before
    for name in ['outbox','kernel_outbox','save_outbox']:
        assert db.execute(f'select count(*) from {name} where projected<>1').fetchone()[0]==0
    db.close()
    lib=sqlite3.connect(root/'managed/Library/Generations/000001/library.sqlite')
    assert lib.execute('select count(*) from document_effect_events').fetchone()[0]==7
    assert lib.execute('select count(*) from (select source_store_id,event_id from document_effect_events group by source_store_id,event_id)').fetchone()[0]==7
    lib.close()
    assert hashlib.sha256(saved.read_bytes()).hexdigest()==before==manifest['fileHash']
    records.append({'boundary':stage,'process_exit':-signal.SIGKILL,'root':str(root.relative_to(ROOT)),'source_snapshot_hash':manifest['snapshotHash'],'accepted_checkpoints':kernel_before,'library_events_before':events_before,'document_projected_before':pending_before,'library_events_after':7,'file_hash':before})
    if stage==STAGES[0]:
        # Closed, disposable fixture branch only; this is not the app's backup algorithm.
        corrupt_root=Path(tempfile.mkdtemp(prefix='recovery-corrupt-',dir=ROOT/'target/macos-storage-tests'))
        shutil.copytree(root,corrupt_root,dirs_exist_ok=True)
        corrupt=subprocess.run([str(BINARY),str(corrupt_root),'session-corrupt'],capture_output=True,text=True,env=ENV,timeout=50)
        assert corrupt.returncode==0,(corrupt.stdout,corrupt.stderr)
        assert 'Damaged original checkpoint refused' in corrupt.stdout
        records[-1]['damaged_blob_refused_without_source_loss']=corrupt.stdout.strip()
        assert hashlib.sha256(saved.read_bytes()).hexdigest()==before
    native=subprocess.run([str(BINARY),str(root),'session-recover'],capture_output=True,text=True,env=ENV,timeout=50)
    assert native.returncode==0,(stage,native.stdout,native.stderr)
    assert 'explicit b-only run uses a=5 and returns6' in native.stdout
    records[-1]['native_restore_without_replay_and_explicit_b6']=native.stdout.strip()
evidence=ROOT/'target/acceptance/pre-alpha.4/development/r41'
evidence.mkdir(parents=True,exist_ok=True)
(evidence/'library-projection-interruptions.json').write_text(json.dumps(records,ensure_ascii=False,indent=2)+'\n')
print('Five actual SIGKILL projection boundaries: one stable Library event per original fact, original document ACKs recovered; source/bootstrap+4 calculation checkpoints/file unchanged, no evaluation or network replay')
