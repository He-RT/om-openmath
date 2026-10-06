import { useEffect, useMemo, useRef, useState } from 'react';
import type { OutputItem } from '../../kernel/generated/OutputItem';
import type { KernelClient } from '../../kernel/client';
import type { Messages,Locale } from '../../i18n';
import { ExploreTask } from '../../kernel/exploreTask';
import { OutputView } from '../output/OutputView';
export function ExploreView({item,cellId,fresh,kernel,t,language,onInsert,taskFactory=()=>new ExploreTask()}: {
  item:Extract<OutputItem,{type:'explore'}>;cellId:string;fresh:boolean;kernel:KernelClient;t:Messages;language:Locale;onInsert:(source:string)=>void;taskFactory?:()=>ExploreTask;
}) {
  const zh=language==='zh-CN',label=(a:string,b:string)=>zh?a:b;
  const [task]=useState(taskFactory);
  const [values,setValues]=useState({...item.result.values});
  const [result,setResult]=useState(item.result);
  const [context,setContext]=useState<string|null>(null);
  const [busy,setBusy]=useState(false),[error,setError]=useState<string|null>(null);
  const owner=useRef({alive:true,revision:0,timer:undefined as ReturnType<typeof setTimeout>|undefined});
  const cancel=()=>{owner.current.revision++;if(owner.current.timer)clearTimeout(owner.current.timer);owner.current.timer=undefined;task.cancel();setBusy(false);};
  useEffect(()=>{
    const state=owner.current;state.alive=true;
    if(document.hidden)task.suspend();else task.resume();
    if(!fresh){task.cancel();setBusy(false);return;}
    const token=++state.revision;
    void kernel.request({type:'get_explore_context',query:{cell_id:cellId,out_index:item.out_index,view_id:item.view_id}}).then(response=>{
      if(!state.alive||state.revision!==token)return;
      if(response.type==='explore_context'&&response.view_id===item.view_id){setContext(response.context);setError(null);}
      else setError(response.type==='error'?response.message:label('无法读取参数探索快照','Cannot read exploration snapshot'));
    }).catch(()=>{if(state.alive&&state.revision===token)setError(label('读取快照失败','Snapshot request failed'));});
    const visibility=()=>{if(document.hidden){cancel();task.suspend();}else task.resume();};document.addEventListener('visibilitychange',visibility);
    const off=kernel.onEvent(e=>{if(e.type==='kernel_restarted'){cancel();setContext(null);setError(label('请重新运行单元格','Run this cell again'));}});
    return ()=>{state.alive=false;state.revision++;if(state.timer)clearTimeout(state.timer);task.cancel();document.removeEventListener('visibilitychange',visibility);off();};
  },[kernel,item.view_id,fresh]);
  const run=(next:Record<string,number>,delay=33)=>{
    if(!fresh||!context)return;
    cancel();setValues(next);setError(null);setBusy(true);
    const state=owner.current,token=state.revision;
    state.timer=setTimeout(()=>{
      state.timer=undefined;
      void task.request({type:'run_explore_context',context,values:next,revision:token}).then(response=>{
        if(!state.alive||state.revision!==token)return;
        if(response.type==='explored'&&response.result.revision===token&&response.result.view_id===item.view_id)setResult(response.result);
        else setError(response.type==='error'?response.message:label('参数探索回复无效','Invalid exploration reply'));
        setBusy(false);
      }).catch(failure=>{if(state.alive&&state.revision===token){setError(failure instanceof Error?failure.message:label('参数计算失败','Exploration failed'));setBusy(false);}});
    },delay);
  };
  const changed=item.controls.some(c=>values[c.name]!==result.values[c.name]);
  const proxy=useMemo<KernelClient>(()=>({
    kind:'wasm',ready:Promise.resolve(),onEvent:callback=>kernel.onEvent(callback),
    request:request=>{
      if(!fresh||busy||changed)return Promise.resolve({type:'error',message:label('当前参数结果已过期','Current parameter result is stale')});
      if(!context)return Promise.resolve({type:'error',message:label('等待只读快照','Waiting for readonly snapshot')});
      if(request.type==='sample_plot')return task.request({type:'sample_explore_plot',context,values:result.values,request:request.request});
      if(request.type==='inspect_expression')return task.request({type:'inspect_explore_expression',context,values:result.values,source:request.source,numeric:request.numeric});
      return Promise.resolve({type:'error',message:label('此临时结果不写入主笔记本历史','Ephemeral results do not write notebook history')});
    },interrupt:async()=>task.cancel(),dispose:()=>task.cancel(),
  }),[kernel,context,result,task,language,fresh,busy,changed]);
  return <section className="explore-view" aria-busy={busy}>
    <div className="explore-heading"><strong>{label('参数探索','Parameter exploration')}</strong><span>{label('独立只读作用域','Isolated readonly scope')}</span></div>
    {item.controls.map(c=><label className="explore-control" key={c.name}><code>{c.name}</code>
      <input type="range" aria-label={label('探索参数 ','Explore parameter ')+c.name} min={c.range[0]} max={c.range[1]} step={Math.max(Number.MIN_VALUE,(c.range[1]-c.range[0])/200)} value={values[c.name]} disabled={!fresh||!context} onChange={e=>run({...values,[c.name]:Number(e.target.value)})}/>
      <output>{values[c.name]?.toPrecision(6)}</output></label>)}
    <div className="explore-actions">
      <button disabled={!fresh||!context} onClick={()=>run({...values},0)}>{label('重新计算','Recompute')}</button>
      <button disabled={!fresh||!context} onClick={()=>run({...item.result.values},0)}>{label('复位参数','Reset parameters')}</button>
      <button disabled={!busy} onClick={cancel}>{label('取消','Cancel')}</button>
      <span role="status">{busy?t.sampling:changed?label('下方保留上次成功参数的结果','Below: last successful parameter result'):''}</span>
    </div>
    {error&&<p role="alert">{error}</p>}
    <OutputView key={`${item.view_id}:${result.revision}`} output={{items:[result.item],messages:result.messages,timing_ms:result.timing_ms}} stale={!fresh||busy||changed} t={t} language={language} kernel={proxy} cellId={cellId} onInsert={onInsert} onSteps={()=>{}}/>
  </section>;
}
