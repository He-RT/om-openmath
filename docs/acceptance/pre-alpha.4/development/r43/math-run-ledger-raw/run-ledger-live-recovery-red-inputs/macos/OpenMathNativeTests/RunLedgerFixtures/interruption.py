"""Real disposable Swift/SQLite children. Recovery never receives an execution callback."""
from pathlib import Path
import hashlib
import json
import os
import signal
import sqlite3
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parents[3]
BASE = ROOT / 'target/macos-storage-tests'
CLI = BASE.parent / 'debug/examples/kernel_store_contract'
FIXTURE = BASE / 'run-ledger-fixtures'
ATTEMPT = Path(tempfile.mkdtemp(prefix='run-ledger-', dir=BASE))


def run(args, log, expected=0):
    with log.open('wb') as output:
        result = subprocess.run([str(a) for a in args], stdout=output, stderr=subprocess.STDOUT, check=False, timeout=60)
    assert result.returncode == expected, (result.returncode, str(log))


def seed(name):
    folder = ATTEMPT / name
    folder.mkdir()
    run([CLI, 'seed', folder / 'contract'], folder / 'seed.log')
    return folder


def audit(folder):
    doc = '00000000-0000-0000-0000-000000000409'
    physical = folder / 'physical'
    with sqlite3.connect(f'file:{physical}/Library/Generations/000001/library.sqlite?mode=ro', uri=True) as db:
        assert db.execute('PRAGMA integrity_check').fetchone()[0] == 'ok'
        events = [(json.loads(bytes(row[0])), json.loads(bytes(row[1]))) for row in db.execute('SELECT record,receipt FROM session_events ORDER BY event_sequence')]
        claims = db.execute('SELECT store_id,operation_id,session_id,event_id FROM session_operation_claims ORDER BY operation_id').fetchall()
        assert len({(row[0], row[1]) for row in claims}) == len(claims)
        assert db.execute("SELECT count(*) FROM session_tool_index WHERE session_id='run-session'").fetchone()[0] == 1
        assert db.execute('SELECT count(*) FROM session_request_index').fetchone()[0] == 0
        projected = db.execute('SELECT operation_id,record FROM document_effect_events').fetchall()
    decoded = []
    for event, receipt in events:
        payload = event['payload_inline_json']
        assert event['payload_hash'] == hashlib.sha256(payload.encode()).hexdigest()
        assert receipt['eventID'] == event['event_id']
        decoded.append({'event': event, 'payload': json.loads(payload), 'receipt': receipt})
    (folder / 'original-events.json').write_text(json.dumps(decoded, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
    (folder / 'original-claims.json').write_text(json.dumps(claims, indent=2) + '\n', encoding='utf-8')
    by_id = {item['event']['event_id']: item for item in decoded}
    admitted = [item for item in decoded if item['event']['event_kind'] == 'tool_admitted']
    assert len(admitted) == 1
    group = admitted[0]['payload']
    children = group['run']['children']
    for index, child in enumerate(children):
        identity = ['openmath/run-child/v1', group['target']['operationID'], group['argumentsHash'], index]
        expected = 'run-' + hashlib.sha256(json.dumps(identity, ensure_ascii=False, separators=(',', ':')).encode()).hexdigest()
        assert child['operationID'] == expected
    for item in decoded:
        body = item['payload']
        if body.get('kind') == 'run_child_started':
            parent = by_id[item['event']['parent_event_id']]
            assert parent['payload']['kind'] == 'run_child_admitted'
            assert parent['event']['operation_id'] == item['event']['operation_id']
        if body.get('kind') == 'run_child_result':
            original = body['original']
            assert original['receipt']['operation_id'] == item['event']['operation_id']
            assert original['sourceEventID'] == original['receipt']['outbox_event_id']
            assert any(op == item['event']['operation_id'] for op, _ in projected)
    with sqlite3.connect(f'file:{physical}/Documents/{doc}/Generations/000001/authority.sqlite?mode=ro', uri=True) as db:
        assert db.execute('PRAGMA integrity_check').fetchone()[0] == 'ok'
        counts = {table: db.execute('SELECT count(*) FROM ' + table).fetchone()[0]
                  for table in ('kernel_operations', 'accepted_results', 'outbox', 'transactions')}
        actual_ids = {row[0] for row in db.execute('SELECT operation_id FROM kernel_operations')}
    accepted = {item['event']['operation_id'] for item in decoded if item['payload'].get('kind') == 'run_child_result'}
    assert actual_ids.issuperset(accepted)
    assert group['target']['operationID'] not in actual_ids  # No fabricated aggregate kernel receipt.
    return {'counts': counts, 'events': len(events), 'claims': len(claims), 'accepted_operations': sorted(accepted), 'actual_kernel_operations': sorted(actual_ids)}


reports = []
for mode in ('normal', 'lost-ack', 'admission-ack-loss', 'started-ack-loss', 'cancel', 'native-failure', 'stop-error'):
    folder = seed(mode)
    run([FIXTURE, folder, mode, CLI], folder / 'actual.log')
    before = (folder / 'executed.json').read_bytes()
    run([FIXTURE, folder, 'recover', CLI], folder / 'reopen-recovery.log')
    assert (folder / 'executed.json').read_bytes() == before
    recovered = json.loads((folder / 'recovered.json').read_text(encoding='utf-8'))
    expected = {'normal': ('partial', 3), 'lost-ack': ('partial', 1), 'cancel': ('cancelled', 0),
                'native-failure': ('failed', 0), 'stop-error': ('partial', 2),
                'admission-ack-loss': ('tool_outcome_unknown', 0), 'started-ack-loss': ('tool_outcome_unknown', 0)}[mode]
    assert (recovered['after'], recovered['accepted_cells']) == expected
    if mode == 'lost-ack':
        assert recovered['execution_callbacks'] == ['bootstrap', 'a']
        assert recovered['unknown_observed']
    if mode in ('admission-ack-loss', 'started-ack-loss'):
        assert recovered['execution_callbacks'] == ['bootstrap']
    detail = audit(folder)
    assert detail['counts']['transactions'] == 0 and detail['counts']['outbox'] == 0
    reports.append({'case': mode, 'recovered': recovered, 'actual': detail})

for selected, point in (
    ('group', 'run_group_admission_acknowledged'),
    ('group', 'run_group_execution_claimed'),
    ('a', 'run_child_admission_acknowledged'),
    ('a', 'run_child_execution_claimed'),
    ('a', 'kernel_committed'),
    ('a', 'run_child_effect_returned'),
    ('a', 'run_child_receipt_acknowledged'),
    ('group', 'run_group_outcome_acknowledged'),
):
    folder = seed('kill-' + point)
    log = folder / 'killed-original.log'
    with log.open('wb') as output:
        child = subprocess.Popen([str(FIXTURE), str(folder), 'normal', str(CLI), selected, point], stdout=output, stderr=subprocess.STDOUT)
        deadline = time.monotonic() + 60
        stopped = False
        while time.monotonic() < deadline:
            state, status = os.waitpid(child.pid, os.WNOHANG | os.WUNTRACED)
            if state and os.WIFSTOPPED(status):
                stopped = True
                break
            if state:
                raise AssertionError(('fixture exited before original boundary', point, status, str(log)))
            time.sleep(0.025)
        try:
            assert stopped, ('live original fixture did not reach boundary', point, child.pid)
            assert 'STOP ' + point in log.read_text(encoding='utf-8')
        finally:
            child.kill()
            child.wait(timeout=10)
    marker = folder / 'executed.json'
    before = marker.read_bytes() if marker.exists() else b''
    run([FIXTURE, folder, 'recover', CLI], folder / 'recovery.log')
    run([FIXTURE, folder, 'recover', CLI], folder / 'recovery-again.log')
    assert (marker.read_bytes() if marker.exists() else b'') == before
    recovered = json.loads((folder / 'recovered.json').read_text(encoding='utf-8'))
    detail = audit(folder)
    if point in ('kernel_committed', 'run_child_effect_returned', 'run_child_receipt_acknowledged'):
        assert recovered['after'] == 'partial' and recovered['accepted_cells'] == 1
        assert recovered['execution_callbacks'] == ['bootstrap', 'a']
    elif point == 'run_group_outcome_acknowledged':
        assert recovered['after'] == 'partial' and recovered['accepted_cells'] == 3
    elif point == 'run_group_execution_claimed':
        # No child was ever admitted, so recovery can close this interrupted group without
        # claiming an uncertain child operation or inventing any mathematical effect.
        assert recovered['after'] == 'failed' and recovered['accepted_cells'] == 0
        assert recovered['execution_callbacks'] == []
    else:
        assert recovered['after'] == 'tool_outcome_unknown' and recovered['accepted_cells'] == 0
    reports.append({'case': 'kill-' + point, 'recovered': recovered, 'actual': detail, 'original_exit': child.returncode})

report = {'evidence_root': str(ATTEMPT), 'cases': reports,
          'scope': 'Actual Swift group/child ledger + physical SQLite/outbox + real Rust math recovered from original receipts; Agent RunExecutor/C ABI/Pi dispatch still separate'}
(ATTEMPT / 'report.json').write_text(json.dumps(report, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
print(json.dumps(report, ensure_ascii=False))
