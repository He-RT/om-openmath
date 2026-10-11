"""SIGKILL real file replacement and same-DB receipt boundaries, retain every attempt."""
from pathlib import Path
import json, os, selectors, signal, subprocess, tempfile

base=Path.cwd()/"target/macos-storage-tests"
helper=base/"save-fixtures"
environment={**os.environ,"DYLD_INSERT_LIBRARIES":str(base/"libSaveSyncProbe.dylib")}
attempts=[]
for stage in ("save_intent_committed","save_file_synced","save_replaced","save_directory_synced","save_receipt_before_commit","save_receipt_committed"):
    root=Path(tempfile.mkdtemp(prefix="save-crash-",dir=base))
    child=subprocess.Popen([str(helper),str(root),stage],env=environment,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True,encoding="utf-8")
    ready=selectors.DefaultSelector();ready.register(child.stdout,selectors.EVENT_READ)
    if not ready.select(20):
        child.kill();child.wait(timeout=5)
        raise AssertionError((stage,"boundary not reached",child.stderr.read()))
    marker=child.stdout.readline().strip();ready.close()
    if marker!=stage:
        child.kill();child.wait(timeout=5)
        raise AssertionError((stage,marker,child.stderr.read()))
    child.kill();child.wait(timeout=5)
    assert child.returncode==-signal.SIGKILL
    inspection=subprocess.run([str(helper),str(root),"inspect"],env=environment,capture_output=True,text=True,encoding="utf-8",timeout=20)
    assert inspection.returncode==0,(stage,inspection.returncode,inspection.stderr)
    record=json.loads(inspection.stdout)
    expected=stage not in ("save_intent_committed","save_file_synced")
    assert record["saved"]==expected and record["kernel_executed"] is False
    assert record["prior_receipt"]==(stage=="save_receipt_committed")
    attempts.append({"stage":stage,"root":str(root),"exit_code":child.returncode,"readback":record})
    (base/"save-interruption-attempt.json").write_text(json.dumps(attempts,ensure_ascii=False,indent=2)+"\n",encoding="utf-8")
print("Six real SIGKILL save/restart boundaries: original intent/bytes/receipt reconciled, pre-replacement file untouched, source retained, no CAS replay")
root=Path(tempfile.mkdtemp(prefix="save-displaced-conflict-",dir=base))
child=subprocess.Popen([str(helper),str(root),"save_replaced"],env=environment,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True,encoding="utf-8")
ready=selectors.DefaultSelector();ready.register(child.stdout,selectors.EVENT_READ)
if not ready.select(20):
    child.kill();child.wait(timeout=5);raise AssertionError("displaced file boundary not reached")
assert child.stdout.readline().strip()=="save_replaced";ready.close()
child.kill();child.wait(timeout=5);assert child.returncode==-signal.SIGKILL
displaced=root/".openmath-save-sigkill-save"
original=json.loads(displaced.read_text(encoding="utf-8"));original["title"]="外部修改原件"
displaced.write_text(json.dumps(original,ensure_ascii=False),encoding="utf-8")
inspection=subprocess.run([str(helper),str(root),"inspect-conflict"],env=environment,capture_output=True,text=True,encoding="utf-8",timeout=20)
assert inspection.returncode==0,(inspection.returncode,inspection.stderr)
record=json.loads(inspection.stdout);assert record["external_original_preserved"] and not record["saved"]
attempts.append({"stage":"displaced-original-conflict","root":str(root),"exit_code":child.returncode,"readback":record})
(base/"save-interruption-attempt.json").write_text(json.dumps(attempts,ensure_ascii=False,indent=2)+"\n",encoding="utf-8")
print("Mismatched displaced external bytes after crash: retained and reported as conflict, no fake save receipt")
