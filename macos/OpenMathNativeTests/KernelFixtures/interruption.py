"""SIGKILL the actual Swift SQLite kernel acceptance, then read the original same-DB facts."""
from pathlib import Path
import json, selectors, signal, subprocess, tempfile
ROOT=Path.cwd();BASE=ROOT/'target/macos-storage-tests'
RUST=ROOT/'target/debug/examples/kernel_store_contract';SWIFT=BASE/'kernel-fixtures'
records=[]
def run(cmd):
    value=subprocess.run([str(x) for x in cmd],capture_output=True,text=True,encoding='utf-8',timeout=15)
    assert value.returncode==0,(cmd,value.returncode,value.stdout,value.stderr)
    return value.stdout.strip()
for stage in ('kernel_blob_referenced','kernel_checkpoint_inserted','before_kernel_commit','kernel_committed','kernel_receipt_readback'):
    root=Path(tempfile.mkdtemp(prefix='kernel-crash-',dir=BASE));fixture=root/'contract';store=root/'physical'
    run([RUST,'seed',fixture]);run([SWIFT,'initialize',fixture,store])
    run([RUST,'prepare',fixture,'bootstrap']);run([SWIFT,'commit',fixture,store,'bootstrap'])
    run([RUST,'prepare',fixture,'a','bootstrap'])
    child=subprocess.Popen([str(SWIFT),'crash',str(fixture),str(store),'a',stage],stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True,encoding='utf-8')
    ready=selectors.DefaultSelector();ready.register(child.stdout,selectors.EVENT_READ)
    if not ready.select(15):
        child.kill();child.wait(timeout=5);raise AssertionError('actual kernel boundary not reached '+stage)
    line=child.stdout.readline().strip();ready.close()
    if line!=stage:
        child.kill();child.wait(timeout=5);raise AssertionError((stage,line))
    child.kill();child.wait(timeout=5);assert child.returncode==-signal.SIGKILL
    result=run([SWIFT,'inspect-crash',fixture,store,'a'])
    expected='same-accepted-candidate' if stage in ('kernel_committed','kernel_receipt_readback') else 'previous-accepted-parent'
    assert result==expected,(stage,result)
    records.append({'stage':stage,'root':str(root),'exit':child.returncode,'actual_restart':result})
(BASE/'kernel-interruption-attempt.json').write_text(json.dumps(records,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print('Five actual kernel acceptance SIGKILL boundaries: Blob ref/head/checkpoint/operation/outbox commit together; previous parent or exact accepted candidate retained')
