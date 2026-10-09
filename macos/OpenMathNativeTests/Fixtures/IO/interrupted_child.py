"""Disposable child for real process-interruption points, not a power-loss durability claim."""
from pathlib import Path
import json, os, signal, sys
root=Path(sys.argv[1]); stage=sys.argv[2]
root.mkdir(parents=True,exist_ok=True)
def reached(name):
 print(json.dumps({'stage':name,'pid':os.getpid()}),flush=True)
 if stage==name:os.kill(os.getpid(),signal.SIGSTOP)
fd=os.open(root/'pending.tmp',os.O_WRONLY|os.O_CREAT|os.O_EXCL,0o600)
reached('opened')
if stage=='enospc':
 os.close(fd);raise OSError(28,'Injected ENOSPC before write; no durable receipt')
os.write(fd,b'{"fixture":"owned","revision":1}\n');reached('written')
os.fsync(fd);reached('file_synced');os.close(fd)
os.replace(root/'pending.tmp',root/'active.json');reached('renamed')
directory=os.open(root,os.O_RDONLY);os.fsync(directory);os.close(directory);reached('directory_synced')
print(json.dumps({'status':'complete'}),flush=True)
