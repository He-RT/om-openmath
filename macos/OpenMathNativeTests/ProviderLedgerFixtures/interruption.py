"""SIGKILL disposable producer children at actual durable request boundaries."""
import os
from pathlib import Path
import signal
import subprocess
import sys
import time

binary, root = Path(sys.argv[1]).resolve(), Path(sys.argv[2]).resolve()
stages = {
    'provider_request_plan_durable': 'not_dispatched',
    'provider_dispatch_durable': 'dispatch_outcome_unknown',
    'provider_response_durable': 'response_observed',
    'provider_finish_durable': 'finished',
    'provider_model_reply_durable': 'finished',
}
for stage, expected in stages.items():
    attempt = root / stage
    attempt.mkdir(parents=True)
    store = attempt / 'store'
    store.mkdir()
    with (attempt / 'child.log').open('wb') as output:
        child = subprocess.Popen([str(binary), str(store), 'run', stage], stdout=output, stderr=subprocess.STDOUT)
        deadline = time.monotonic() + 30
        while not (store / 'stopped').exists():
            if child.poll() is not None:
                raise RuntimeError(f'{stage}: child exited before boundary')
            if time.monotonic() > deadline:
                child.kill()
                child.wait()
                raise RuntimeError(f'{stage}: boundary not reached')
            time.sleep(0.02)
        os.kill(child.pid, signal.SIGKILL)
        assert child.wait(timeout=5) == -signal.SIGKILL
    with (attempt / 'recovery.log').open('wb') as output:
        subprocess.run([str(binary), str(store), 'inspect', expected], stdout=output, stderr=subprocess.STDOUT, timeout=30, check=True)
    print(f'{stage}: actual SIGKILL -> {expected}; read-only recovery, no replay')
