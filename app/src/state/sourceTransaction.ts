import type { CellInput } from '../kernel/generated/CellInput';
import type { NotebookFile } from '../kernel/generated/NotebookFile';
export function sameSource(a: CellInput, b: CellInput): boolean {
  return a.id === b.id && a.kind === b.kind && a.dialect === b.dialect && a.source === b.source;
}
export function sameNotebookSource(a: NotebookFile, b: NotebookFile): boolean {
  return a.version === b.version && a.title === b.title && a.cells.length === b.cells.length && a.cells.every((cell, index) => sameSource(cell, b.cells[index]!));
}
function includeMissing(order: string[], desired: string[], present: Set<string>) {
  for (const [index, id] of desired.entries()) {
    if (!present.has(id) || order.includes(id)) continue;
    const previous = desired.slice(0, index).reverse().find(id => order.includes(id));
    const next = desired.slice(index + 1).find(id => order.includes(id));
    order.splice(previous ? order.indexOf(previous) + 1 : next ? order.indexOf(next) : order.length, 0, id);
  }
}
/** Project an already ACKed immutable change while preserving later local edits/deletes/order. */
export function projectAcceptedSource(before: NotebookFile, accepted: NotebookFile, latest: NotebookFile): NotebookFile {
  const original = new Map(before.cells.map(cell => [cell.id, cell]));
  const proposed = new Map(accepted.cells.map(cell => [cell.id, cell]));
  const current = new Map(latest.cells.map(cell => [cell.id, cell]));
  const oldOrder = before.cells.filter(cell => current.has(cell.id)).map(cell => cell.id);
  const newOrder = latest.cells.filter(cell => original.has(cell.id)).map(cell => cell.id);
  const moved = JSON.stringify(oldOrder) !== JSON.stringify(newOrder);
  const cells = new Map<string, CellInput>();
  for (const cell of latest.cells) {
    const old = original.get(cell.id), planned = proposed.get(cell.id);
    if (!old || !sameSource(old, cell)) cells.set(cell.id, cell);
    else if (planned) cells.set(cell.id, planned);
  }
  for (const cell of accepted.cells) if (!original.has(cell.id) && !cells.has(cell.id)) cells.set(cell.id, cell);
  const order = (moved ? latest : accepted).cells.map(cell => cell.id).filter(id => cells.has(id));
  const present = new Set(cells.keys());
  includeMissing(order, latest.cells.map(cell => cell.id), present);
  includeMissing(order, accepted.cells.map(cell => cell.id), present);
  return { version: 1, title: latest.title === before.title ? accepted.title : latest.title, cells: order.map(id => cells.get(id)!) };
}
