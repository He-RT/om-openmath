import type { NotebookFile } from './generated/NotebookFile';
import type { KernelConfig } from './generated/KernelConfig';
import type { Request } from './client';
/** Mirror only synchronized editable source; the Kernel computes all dependency metadata. */
export function sourceMutation(file: NotebookFile, request: Request): NotebookFile {
  const next = structuredClone(file);
  if (request.type === 'load_notebook') {
    const ids = request.file.cells.map(c => c.id);
    if (request.file.version === 1 && ids.every(Boolean) && new Set(ids).size === ids.length) return structuredClone(request.file);
  } else if (request.type === 'upsert_cell' || request.type === 'evaluate') {
    const cell = request.type === 'upsert_cell' ? request.cell : { id: request.cell_id, kind: 'Math' as const, source: request.source, dialect: request.dialect };
    if (!cell.id) return next;
    const index = next.cells.findIndex(c => c.id === cell.id);
    if (request.type === 'evaluate' && index >= 0 && next.cells[index]?.kind !== 'Math') return next;
    if (index < 0) next.cells.push(structuredClone(cell)); else next.cells[index] = structuredClone(cell);
  } else if (request.type === 'delete_cell') next.cells = next.cells.filter(c => c.id !== request.cell_id);
  else if (request.type === 'move_cell') {
    const index = next.cells.findIndex(c => c.id === request.cell_id);
    if (index >= 0 && request.to_index >= 0 && request.to_index < next.cells.length) {
      const [cell] = next.cells.splice(index, 1); if (cell) next.cells.splice(request.to_index, 0, cell);
    }
  }
  return next;
}
/** Keep browser credentials in memory across a restart; masks never replace actual values. */
export function mergeConfig(next: KernelConfig, previous?: KernelConfig): KernelConfig {
  const config = structuredClone(next);
  for (const profile of config.llm.profiles) if (profile.api_key === '***') profile.api_key = previous?.llm.profiles.find(p => p.name === profile.name)?.api_key ?? null;
  return config;
}
