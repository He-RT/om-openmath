import { useEffect, useRef, useState } from 'react';
import type { KernelClient } from '../../kernel/client';
import type { Locale, Messages } from '../../i18n';
import type { ExpressionView } from '../../kernel/generated/ExpressionView';
import type { ValuePage } from '../../kernel/generated/ValuePage';
import type { ValueEntry } from '../../kernel/generated/ValueEntry';
import { Katex } from './Katex';
import { ArtifactButton } from './ArtifactButton';
import { SourceActions } from './Actions';
import { fieldLabel, kindLabel, natureLabel, statusLabel } from './valueLabels';

export function ValueView({ initial, cellId, outIndex, value, fresh, kernel, language, t, onInsert }: {
  initial: ValuePage; cellId: string; outIndex: number; value: ExpressionView; fresh: boolean;
  kernel: KernelClient; language: Locale; t: Messages; onInsert: (source: string) => void;
}) {
  const [page, setPage] = useState(initial);
  const [trail, setTrail] = useState<ValuePage[]>([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [sourceVisible, setSourceVisible] = useState(false);
  const [selectedValue, setSelectedValue] = useState<ExpressionView | null>(null);
  const sequence = useRef(0);
  const alive = useRef(true);
  const text = (zh: string, en: string) => language === 'zh-CN' ? zh : en;
  useEffect(() => { alive.current = true; return () => { alive.current = false; sequence.current++; }; }, []);
  useEffect(() => { sequence.current++; setBusy(false); setError(null); }, [fresh]);
  const load = (path: number[], offset = 0, columnOffset = 0, fullSource = false, descend = false) => {
    if (!fresh || !cellId || busy) return;
    const request = ++sequence.current;
    setBusy(true); setError(null);
    void kernel.request({ type: 'inspect_value', query: {
      cell_id: cellId, out_index: outIndex, view_id: initial.view_id, path,
      offset, limit: 32, column_offset: columnOffset, column_limit: 8, include_source: fullSource,
    } }).then(reply => {
      if (!alive.current || sequence.current !== request) return;
      if (reply.type !== 'value_page' || reply.page.view_id !== initial.view_id) {
        setError(reply.type === 'error' ? reply.message : text('结果读取失败', 'Cannot read result')); return;
      }
      if (descend) setTrail(previous => [...previous, page]);
      setPage(reply.page); setSourceVisible(fullSource); setSelectedValue(null);
    }, () => { if (alive.current && sequence.current === request) setError(text('结果读取失败', 'Cannot read result')); })
      .finally(() => { if (alive.current && sequence.current === request) setBusy(false); });
  };
  const root = page.path.length === 0;
  const authoritative = root && page.origin !== null;
  const shownSource = root ? value : page.source;
  const rowEnd = page.offset + page.rows.length;
  const columnEnd = page.column_offset + page.columns.length;
  const renderEntry = (entry: ValueEntry) => {
    if (entry.source) {
      const label = statusLabel(entry.source.input_form, language, authoritative);
      return <div className="value-scalar">
        {label ? <span>{label}</span> : entry.nature === 'text' || entry.nature === 'boolean' || entry.nature === 'null'
          ? <code>{entry.source.input_form}</code> : <Katex latex={entry.source.latex} label={entry.source.input_form} />}
        <span className="value-nature">{natureLabel(entry.nature, language)}</span>
        <button className="value-cell-action" aria-label={text(`值操作 ${entry.source.input_form}`, `Value actions ${entry.source.input_form}`)} onClick={() => setSelectedValue(entry.source)}>{text('操作', 'Actions')}</button>
      </div>;
    }
    return <button className="value-expand" disabled={!fresh || busy || !cellId}
      onClick={() => load(entry.path, 0, 0, false, true)}>
      {kindLabel(entry.kind, language)} · {entry.count} {text('项', 'items')}
    </button>;
  };
  return <section className="structured-value" aria-label={text('结构化计算结果', 'Structured result')} aria-busy={busy}>
    <div className="value-heading">
      <strong>{kindLabel(page.kind, language)}</strong>
      {root && page.origin && <span>{page.origin.name} · {text('实际计算', 'Actual computation')}</span>}
      {page.row_count > 0 && <span>{page.row_count} {text('行/字段', 'rows/fields')} · {page.column_count} {text('列', 'columns')}</span>}
      {trail.length > 0 && <button disabled={busy} onClick={() => {
        const parent = trail.at(-1); if (!parent) return;
        sequence.current++; setPage(parent); setTrail(previous => previous.slice(0, -1)); setError(null); setSourceVisible(false); setSelectedValue(null);
      }}>{text('返回上层', 'Back')}</button>}
    </div>
    {page.rows.length > 0 ? <div className="value-table-scroll" tabIndex={0} aria-label={text('表格，左右滚动查看', 'Table; scroll horizontally')}>
      <table className="value-table"><thead><tr>
        <th scope="col">{page.kind === 'record' ? text('字段', 'Field') : '#'}</th>
        {page.columns.map((column, index) => <th scope="col" key={`${page.view_id}:${page.column_offset + index}`}>
          {page.kind === 'record' || page.kind === 'list' ? text('值', 'Value') : column}
        </th>)}
      </tr></thead><tbody>{page.rows.map(row => <tr key={row.id}>
        <th scope="row">{fieldLabel(row.label, language, authoritative)}</th>
        {row.cells.map(entry => <td key={entry.id}>{renderEntry(entry)}</td>)}
      </tr>)}</tbody></table>
    </div> : page.row_count === 0 && (page.kind === 'table' || page.kind === 'list' || page.kind === 'record')
      ? <p className="value-empty">{text('没有数据行', 'No data rows')}{page.columns.length > 0 && <span> · {page.columns.join(' · ')}</span>}</p>
      : page.source && <pre className="value-source">{page.source.modern_form}</pre>}
    {(page.row_count > page.rows.length || page.column_count > page.columns.length) && <nav className="value-pagination" aria-label={text('结果分页', 'Result pages')}>
      <button disabled={!fresh || busy || page.offset === 0 || !cellId} onClick={() => load(page.path, Math.max(0, page.offset - 32), page.column_offset)}>{text('上一页', 'Previous')}</button>
      <span>{page.row_count === 0 ? 0 : page.offset + 1}–{rowEnd} / {page.row_count}</span>
      <button disabled={!fresh || busy || rowEnd >= page.row_count || !cellId} onClick={() => load(page.path, rowEnd, page.column_offset)}>{text('下一页', 'Next')}</button>
      {page.column_count > page.columns.length && <>
        <button disabled={!fresh || busy || page.column_offset === 0 || !cellId} onClick={() => load(page.path, page.offset, Math.max(0, page.column_offset - 8))}>{text('前列', 'Previous columns')}</button>
        <span>{page.column_count === 0 ? 0 : page.column_offset + 1}–{columnEnd} / {page.column_count}</span>
        <button disabled={!fresh || busy || columnEnd >= page.column_count || !cellId} onClick={() => load(page.path, page.offset, columnEnd)}>{text('后列', 'Next columns')}</button>
      </>}
    </nav>}
    <div className="value-actions">
      {(['csv','json'] as const).map(format=><ArtifactButton key={format} kernel={kernel} t={t} title="OpenMath-data" label={format==='csv'?t.exportCsv:t.exportJson}
        version={`${initial.view_id}:${page.path.join('.')}`} enabled={fresh&&!busy&&!!cellId}
        request={()=>({type:'export_value',format,query:{cell_id:cellId,out_index:outIndex,view_id:initial.view_id,path:page.path,offset:0,limit:32,column_offset:0,column_limit:8,include_source:false}})}/>)}

      {shownSource && <SourceActions value={shownSource} t={t} onInsert={onInsert} />}
      <button disabled={!root && (!fresh || busy || !cellId)} onClick={() => {
        if (shownSource) setSourceVisible(previous => !previous); else load(page.path, page.offset, page.column_offset, true);
      }}>{text('完整源码', 'Full source')}</button>
    </div>
    {selectedValue && <div className="value-selection" role="group" aria-label={text('选中值操作', 'Selected value actions')}>
      <code>{selectedValue.modern_form}</code><SourceActions value={selectedValue} t={t} onInsert={onInsert} />
      <button onClick={() => setSelectedValue(null)}>{text('关闭', 'Close')}</button>
    </div>}
    {sourceVisible && shownSource && <pre className="value-source">{shownSource.modern_form}</pre>}
    {busy && <span role="status">{text('读取结果…', 'Reading result…')}</span>}
    {error && <p role="alert" className="output-error">{error}</p>}
  </section>;
}
