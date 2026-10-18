import { useEffect, useMemo, useState } from 'react';
import type { NotebookController, UiCell } from '../state/notebookStore';
import type { HoverInfo } from '../kernel/generated/HoverInfo';
import type { Messages } from '../i18n';

export function VariablesPanel({ cells, controller, generation, busy, language, t, phone }: {
  cells: UiCell[]; controller: NotebookController; generation: number; busy: boolean;
  language: string; t: Messages; phone: boolean;
}) {
  const [values, setValues] = useState<[string, HoverInfo][]>([]);
  const [loadedRevision, setLoadedRevision] = useState('');
  const [error, setError] = useState(''), [retry, setRetry] = useState(0);
  const [loading, setLoading] = useState(false);
  // Draft keystrokes in an already stale cell do not require another worker query.
  const revision = cells.map(cell => `${cell.id}:${cell.status}:${cell.exec_count ?? ''}:${cell.defines.join(',')}`).join('|');
  useEffect(() => {
    if (busy) return;
    let current = true; setLoading(true); setError('');
    void controller.kernel.request({ type: 'get_variables' }).then(response => {
      if (!current) return;
      if (response.type !== 'variables') throw new Error(response.type === 'error' ? response.message : t.variableReadError);
      setValues(response.items);
      setLoadedRevision(revision);
    }).catch(failure => { if (current) setError(failure instanceof Error ? failure.message : t.variableReadError); })
      .finally(() => { if (current) setLoading(false); });
    return () => { current = false; };
  }, [controller, generation, revision, busy, language, retry, t.variableReadError]);
  const rows = useMemo(() => {
    const accepted = new Set(values.map(([name]) => name));
    return [
      ...values.map(([name, info]) => ({ name, value: info.value, cellId: info.cell_id, missing: false })),
      ...cells.filter(cell => cell.status === 'Error' && !busy && !loading && !error && loadedRevision === revision).flatMap(cell => cell.defines
        .filter(name => !accepted.has(name))
        .map(name => ({ name, value: null, cellId: cell.id, missing: true }))),
    ].sort((a, b) => a.name.localeCompare(b.name));
  }, [values, cells, busy, loading, error, loadedRevision, revision]);
  const locate = (id: string | null) => {
    if (!id || !cells.some(cell => cell.id === id)) return;
    controller.select(id);
    if (phone) controller.store.setState({ panel: null });
    requestAnimationFrame(() => {
      const target = [...document.querySelectorAll<HTMLElement>('.notebook-cell[data-cell-id]')].find(node => node.dataset.cellId === id);
      if (!target) return;
      target.scrollIntoView({ block: 'nearest', behavior: 'instant' });
      if (!matchMedia('(prefers-reduced-motion: reduce)').matches)
        target?.animate?.([{ outline: '3px solid var(--indigo)' }, { outline: '3px solid transparent' }], { duration: 1000 });
    });
  };
  return <section className="variables-panel" aria-label={t.variables} aria-busy={busy || loading}>
    <p className="inspector-description">{t.variablesHint}</p>
    {rows.length === 0 && !error && <p className="muted">{loading || busy ? t.variablesLoading : t.noVariables}</p>}
    {rows.length > 0 && <table className="variables-table">
      <thead><tr><th scope="col">{t.variableName}</th><th scope="col">{t.variableValue}</th><th scope="col">{t.variableFrom}</th></tr></thead>
      <tbody>{rows.map(row => {
        const index = cells.findIndex(cell => cell.id === row.cellId), owner = cells[index];
        const stale = !!owner && (owner.status !== 'Done' || !!owner.resultSource && (owner.resultSource.source !== owner.source || owner.resultSource.dialect !== owner.dialect));
        const label = index >= 0 ? t.cellNumber.replace('{n}', String(index + 1)) : t.variableOwnerMissing;
        return <tr key={`${row.name}:${row.cellId}`} data-stale={stale && !row.missing || undefined} data-missing={row.missing || undefined}
          tabIndex={index >= 0 ? 0 : undefined} onClick={() => locate(row.cellId)}
          onKeyDown={event => { if (event.key === 'Enter' || event.key === ' ') { event.preventDefault(); locate(row.cellId); } }}>
          <th scope="row"><code>{row.name}</code></th>
          <td><span className="variable-value" title={row.missing ? t.variableUndefined : row.value ?? t.variableNoSummary}>{row.missing ? t.variableUndefined : row.value ?? t.variableNoSummary}</span>
            {stale && !row.missing && <> <small>{t.variableStale}</small></>}</td>
          <td>{label}</td>
        </tr>;
      })}</tbody>
    </table>}
    {error && <div className="variables-error" role="alert"><p>{t.variableReadError}: {error}</p><button onClick={() => setRetry(value => value + 1)}>{t.retry}</button></div>}
  </section>;
}
