import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { splitBytes } from './fixtures/chunks.mjs';
const root = new URL('../../macos/OpenMathNativeTests/Fixtures/', import.meta.url);
const manifest = JSON.parse(readFileSync(new URL('manifest.json', root), 'utf8'));
test('sealed own fixture bytes and real streaming UTF-8 boundaries', () => {
  for (const entry of manifest.files) {
    const bytes = readFileSync(new URL(entry.path, root));
    assert.equal(createHash('sha256').update(bytes).digest('hex'), entry.sha256);
  }
  const bytes = readFileSync(new URL('Network/utf8-stream.sse', root));
  const plan = JSON.parse(readFileSync(new URL('./fixtures/stream-splits.json', import.meta.url), 'utf8'));
  for (const width of plan.chunk_sizes) {
    const decoder = new TextDecoder('utf-8', { fatal: true });
    let text = '';
    for (const part of splitBytes(bytes, width)) text += decoder.decode(part, { stream: true });
    text += decoder.decode();
    assert.equal(text, bytes.toString('utf8'));
    assert.ok(text.includes(plan.expected_text));
  }
  const bad = new TextDecoder('utf-8', { fatal: true });
  bad.decode(readFileSync(new URL('Network/broken-utf8.bin', root)), { stream: true });
  assert.throws(() => bad.decode());
});
