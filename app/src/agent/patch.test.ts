import {expect,test,vi} from 'vitest';
import {webcrypto} from 'node:crypto';
import {buildPatch,invertSourceTransaction,sha256} from './patch';
import type {NotebookFile} from '../kernel/generated/NotebookFile';
const file=(cells:[string,string][],title='original'):NotebookFile=>({version:1,title,cells:cells.map(([id,source])=>({id,source,kind:'Math',dialect:'Modern'}))});
test('patch uses original hashes, exact Unicode and assigned insert selectors without touching source',async()=>{
  vi.stubGlobal('crypto',webcrypto);
  try{
    const original=file([['a','α = 2; β = 3']]),hash=await sha256(original.cells[0]!.source);
    let id=0;
    const result=await buildPatch(original,[{type:'replace_text',target:{cell_id:'a'},expected_source_hash:hash,match:'2',replacement:'5'},{type:'replace_text',target:{cell_id:'a'},expected_source_hash:hash,match:'3',replacement:'6'},
      {type:'insert_cell',client_key:'one',after:{cell_id:'a'},kind:'Math',dialect:'Modern',source:'a+1'},
      {type:'insert_cell',client_key:'two',after:{client_key:'one'},kind:'Text',dialect:'Modern',source:'notes'},
      {type:'move_cell',target:{client_key:'two'},after:null}],new Set(['a']),()=>`new-${++id}`);
    expect(result.after.cells.map(cell=>cell.id)).toEqual(['new-2','a','new-1']);
    expect(result.after.cells[1]!.source).toBe('α = 5; β = 6');expect(result.planHash).toHaveLength(64);
    expect(original.cells[0]!.source).toBe('α = 2; β = 3');
    await expect(buildPatch(original,[{type:'update_cell',target:{cell_id:'a'},expected_source_hash:hash,source:'changed'}],new Set())).rejects.toThrow('INCOMPLETE_SOURCE');
    await expect(buildPatch(original,[{type:'delete_cell',target:{cell_id:'a'},expected_source_hash:'0'.repeat(64)}],new Set(['a']))).rejects.toThrow('STALE_SNAPSHOT');
    const repeated=file([['a','x x']]);
    await expect(buildPatch(repeated,[{type:'replace_text',target:{cell_id:'a'},expected_source_hash:await sha256('x x'),match:'x',replacement:'y'}],new Set(['a']))).rejects.toThrow('NONUNIQUE');
  }finally{vi.unstubAllGlobals();}
});
test('inverse preserves unrelated edits and inserted user content, while touched edits reject the entire undo',()=>{
  const before=file([['a','2'],['b','3'],['removed','old']]),after=file([['new','agent'],['a','5'],['b','3']]);
  const current=file([['new','agent'],['a','5'],['user','notes'],['b','manual']]);
  const reverted=invertSourceTransaction(current,before,after);
  expect(reverted).toEqual(file([['a','2'],['user','notes'],['b','manual'],['removed','old']]));
  expect(()=>invertSourceTransaction(file([['new','manual'],['a','5'],['b','3']]),before,after)).toThrow('UNDO_CONFLICT');
  expect(current.cells[0]!.source).toBe('agent');
});
test('inverse restores moved relative order without dropping later inserted cells',()=>{
  const before=file([['a','2'],['b','3']]),after=file([['b','3'],['a','2']]);
  expect(invertSourceTransaction(file([['b','3'],['user','notes'],['a','2']]),before,after)).toEqual(file([['a','2'],['user','notes'],['b','3']]));
  expect(()=>invertSourceTransaction(file([['a','2'],['b','3']]),before,after)).toThrow('UNDO_CONFLICT');
});
