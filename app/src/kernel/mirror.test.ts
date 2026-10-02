import { expect, test } from 'vitest';
import { sourceMutation, mergeConfig } from './mirror';
import type { NotebookFile } from './generated/NotebookFile';
import type { KernelConfig } from './generated/KernelConfig';
test('source synchronization preserves failed source, document ordering and excludes invalid file shapes', () => {
  const original: NotebookFile = { version: 1, title: 'keep', cells: [{ id: 'a', kind: 'Math', source: '1', dialect: 'Modern' }] };
  const current = sourceMutation(original, { type: 'evaluate', cell_id: 'a', source: 'solve(', dialect: 'Modern' });
  expect(current.cells[0]?.source).toBe('solve('); expect(original.cells[0]?.source).toBe('1');
  expect(sourceMutation(current, { type: 'load_notebook', file: { version: 2, title: '', cells: [] } })).toEqual(current);
  const added = sourceMutation(current, { type: 'upsert_cell', cell: { id: 'b', kind: 'Text', source: 'notes', dialect: 'Auto' } });
  const moved = sourceMutation(added, { type: 'move_cell', cell_id: 'b', to_index: 0 });
  expect(moved.cells.map(c => c.id)).toEqual(['b', 'a']);
  expect(sourceMutation(moved, { type: 'evaluate', cell_id: 'b', source: '2', dialect: 'Modern' }).cells[0]?.kind).toBe('Text');
});
test('secret masks preserve actual same-name in-memory credentials without writing a notebook file', () => {
  const config = { llm: { profiles: [{ name: 'same', api_key: 'synthetic-key' }] } } as KernelConfig;
  const next = { llm: { profiles: [{ name: 'same', api_key: '***' }, { name: 'renamed', api_key: '***' }] } } as KernelConfig;
  const merged = mergeConfig(next, config);
  expect(merged.llm.profiles[0]?.api_key).toBe('synthetic-key'); expect(merged.llm.profiles[1]?.api_key).toBeNull();
});
