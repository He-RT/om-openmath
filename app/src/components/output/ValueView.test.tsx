import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { expect, it, vi } from 'vitest';
import { ValueView } from './ValueView';
import { messages } from '../../i18n';
import { MockKernel } from '../../test/mockKernel';
import type { ValuePage } from '../../kernel/generated/ValuePage';
import type { ValueEntry } from '../../kernel/generated/ValueEntry';
import type { Response } from '../../kernel/generated/Response';
const value = { input_form: 'DataTable[columns,rows]', modern_form: 'data', latex: 'data' };
const entry = (id: string, source: string): ValueEntry => ({ id, path: [1, Number(id), 0, 1], kind: 'scalar', nature: 'text', count: 0, source: { input_form: source, modern_form: source, latex: source } });
const table: ValuePage = { view_id: '43', path: [], kind: 'table', row_count: 35, column_count: 1, offset: 0, column_offset: 0, columns: ['文字'], rows: Array.from({length:32},(_,i)=>({id:`row-${i}`,label:String(i+1),cells:[entry(String(i),`"值 ${i}"`)]})), source: null, origin: null };
function mount(page = table, kernel = new MockKernel(), insert = vi.fn(), fresh = true) {
  return render(<ValueView initial={page} cellId="owner" outIndex={7} value={value} fresh={fresh} kernel={kernel} language="zh-CN" t={messages('zh-CN')} onInsert={insert} />);
}
it('uses actual snapshot pages, displays complete text and copies/inserts the selected value', async () => {
  const kernel = new MockKernel(request => {
    if (request.type !== 'inspect_value') throw new Error('readonly page expected');
    expect(request.query).toMatchObject({ cell_id: 'owner', out_index: 7, view_id: '43', offset: 32 });
    return { type: 'value_page', page: { ...table, offset: 32, rows: [{id:'row-32',label:'33',cells:[entry('32','"中文🙂 long text"')]}] } };
  });
  const insert = vi.fn(); mount(table,kernel,insert);
  fireEvent.click(screen.getByRole('button',{name:'下一页'}));
  await screen.findByText('"中文🙂 long text"');
  expect(kernel.requests).toHaveLength(1);
  fireEvent.click(screen.getByRole('button',{name:'值操作 "中文🙂 long text"'}));
  const group = screen.getByRole('group',{name:'选中值操作'});
  const { within } = await import('@testing-library/react');
  fireEvent.click(within(group).getByRole('button',{name:messages('zh-CN').insertResult}));
  expect(insert).toHaveBeenCalledWith('"中文🙂 long text"');
});
it('does not promote authored records to certified results and exposes genuine producer labels', () => {
  const record: ValuePage = { ...table, kind:'record',row_count:1,rows:[{id:'record-row',label:'guarantee',cells:[entry('0','"certified_global"')]}] };
  const view=mount(record); expect(screen.queryByText('精确全局认证')).toBeNull(); expect(screen.getByText('"certified_global"')).toBeTruthy();
  view.unmount();mount({...record,origin:{function_id:'fn_000179',name:'optimize'}});
  expect(screen.getByText('精确全局认证')).toBeTruthy();expect(screen.getByText('数学保证')).toBeTruthy();
});
it('rejects late page replies after the output becomes stale and retains the old displayed page', async () => {
  let resolve!: (reply: Response) => void;
  const kernel=new MockKernel(()=>new Promise<Response>(done=>{resolve=done;}));
  const view=mount(table,kernel);
  fireEvent.click(screen.getByRole('button',{name:'下一页'}));
  await waitFor(()=>expect(kernel.requests).toHaveLength(1));
  view.rerender(<ValueView initial={table} cellId="owner" outIndex={7} value={value} fresh={false} kernel={kernel} language="zh-CN" t={messages('zh-CN')} onInsert={vi.fn()} />);
  resolve({type:'value_page',page:{...table,offset:32,rows:[{id:'late',label:'33',cells:[entry('32','"late result"')]}]}});
  await waitFor(()=>expect((screen.getByRole('button',{name:'下一页'}) as HTMLButtonElement).disabled).toBe(true));
  expect(screen.queryByText('"late result"')).toBeNull();expect(screen.getByText('"值 0"')).toBeTruthy();
});
it('preserves empty table headers and reports a real failed page request', async () => {
  const view=mount({...table,row_count:0,rows:[],columns:['中文列']});expect(screen.getByText('没有数据行')).toBeTruthy();expect(screen.getByText(/中文列/)).toBeTruthy();view.unmount();
  mount(table,new MockKernel(()=>({type:'error',message:'snapshot changed'})));
  fireEvent.click(screen.getByRole('button',{name:'下一页'}));expect((await screen.findByRole('alert')).textContent).toContain('snapshot changed');
});
