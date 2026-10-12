"""SIGKILL actual SQLite backups and generation publication; inspect original selectors/bytes."""
from pathlib import Path
import hashlib, json, os, selectors, signal, subprocess, tempfile

BASE = Path.cwd() / 'target/macos-storage-tests'
HELPER = BASE / 'maintenance-fixtures'
PLANS = BASE / 'maintenance-plans.json'
DOCUMENT = json.loads(PLANS.read_text(encoding='utf-8'))['initial']['document_id']
records = []

def run(root, mode):
    result = subprocess.run([str(HELPER), str(root), str(PLANS), mode], capture_output=True,
                            text=True, encoding='utf-8', timeout=30)
    assert result.returncode == 0, (mode, result.returncode, result.stdout, result.stderr)
    return result.stdout.strip()

stages = ('backup.started', 'backup.pages', 'backup.finished', 'backup.synced',
          'backup.store_verified', 'backup.before_publish', 'backup.published',
          'generation.verified', 'generation.published', 'generation.before_selector',
          'generation.selector.file_synced', 'generation.selector.renamed',
          'generation.selector.directory_synced', 'generation.selected',
          'gc.quarantined', 'gc.before_delete', 'gc.deleted')
for stage in stages:
    root = Path(tempfile.mkdtemp(prefix='maintenance-crash-', dir=BASE))
    run(root, 'restore-init' if stage.startswith('generation.') else 'gc-init' if stage.startswith('gc.') else 'init')
    selected = root / 'Documents' / DOCUMENT / 'active.json'
    before_selector = selected.read_bytes()
    original = selected.parent / 'Generations/000001/authority.sqlite'
    before_hash = hashlib.sha256(original.read_bytes()).hexdigest()
    child = subprocess.Popen([str(HELPER), str(root), str(PLANS), stage], stdout=subprocess.PIPE,
                             stderr=subprocess.PIPE, text=True, encoding='utf-8')
    ready = selectors.DefaultSelector()
    ready.register(child.stdout, selectors.EVENT_READ)
    if not ready.select(30):
        child.kill()
        child.wait(timeout=5)
        raise AssertionError(('boundary not reached', stage, child.stderr.read()))
    line = child.stdout.readline().strip()
    ready.close()
    if line != stage:
        child.kill()
        child.wait(timeout=5)
        raise AssertionError((stage, line, child.stderr.read()))
    child.kill()
    child.wait(timeout=5)
    assert child.returncode == -signal.SIGKILL
    inspection = json.loads(run(root, 'inspect'))
    assert hashlib.sha256(original.read_bytes()).hexdigest() == before_hash
    if stage.startswith('backup.'):
        expected = 1 if stage == 'backup.published' else 0
        assert inspection['backups'] == expected, inspection
        assert selected.read_bytes() == before_selector
        outcome = 'published verified backup' if expected else 'unpublished backup retained'
    elif stage.startswith('generation.'):
        switched = stage in ('generation.selector.renamed', 'generation.selector.directory_synced', 'generation.selected')
        assert json.loads(selected.read_bytes())['generation'] == (2 if switched else 1)
        assert inspection['backups'] == 1
        outcome = 'selected quarantine generation' if switched else 'original generation still selected'
    else:
        descriptor = json.loads((root / 'fixture-orphan.json').read_text(encoding='utf-8'))
        canonical = root / 'Blobs/sha256' / descriptor['hash'][:2] / descriptor['hash']
        quarantined = root / 'Blobs/Quarantine' / descriptor['hash']
        assert not canonical.exists()
        if stage == 'gc.deleted':
            assert not quarantined.exists()
        else:
            assert hashlib.sha256(quarantined.read_bytes()).hexdigest() == descriptor['hash']
            assert run(root, 'gc-resume') == '1'
            assert not quarantined.exists()
        assert selected.read_bytes() == before_selector
        outcome = 'unreferenced orphan removed' if stage == 'gc.deleted' else 'verified orphan retained in quarantine'
    records.append({'stage': stage, 'exit_code': child.returncode, 'outcome': outcome,
                    'original_sha256': before_hash, 'inspection': inspection, 'root': str(root)})

output = Path.cwd() / 'target/acceptance/pre-alpha.4/development/r41'
output.mkdir(parents=True, exist_ok=True)
(output / 'maintenance-interruption.json').write_text(json.dumps(records, ensure_ascii=False, indent=2)+'\n', encoding='utf-8')
print('17 real SIGKILL boundaries: backups/selector/quarantine/GC verified, original document bytes retained')
