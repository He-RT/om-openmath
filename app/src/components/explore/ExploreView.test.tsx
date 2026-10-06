import {act,fireEvent,render,screen,waitFor} from '@testing-library/react';
import {expect,test} from 'vitest';
import {ExploreView} from './ExploreView';
import {ExploreTask,type ExploreTaskPort} from '../../kernel/exploreTask';
import {MockKernel} from '../../test/mockKernel';
import {messages} from '../../i18n';
import type {OutputItem} from '../../kernel/generated/OutputItem';
import type {Response} from '../../kernel/client';
const item:Extract<OutputItem,{type:'explore'}>={type:'explore',out_index:3,view_id:'snapshot',input_form:'Explore[a^2]',modern_form:'explore(a^2)',controls:[{name:'a',range:[0,4],initial:2}],result:{view_id:'snapshot',revision:0,values:{a:2},item:{type:'expr',out_index:3,input_form:'4.',modern_form:'4.',latex:'4'},messages:[],timing_ms:0}};
class Port implements ExploreTaskPort{
 onmessage:ExploreTaskPort['onmessage']=null;onerror:ExploreTaskPort['onerror']=null;terminated=false;
 request: {request: Extract<import('../../kernel/client').Request,{type:'run_explore_context'}>} | undefined;
 postMessage(v:unknown){this.request=v as typeof this.request;}terminate(){this.terminated=true;}
 reply(response:Response){this.onmessage?.({data:{response}} as MessageEvent<{response:Response}>);}
}
test('actual context retrieval, rapid local values, last success and disposal share producer and generation guards',async()=>{
 const ports:Port[]=[];const task=new ExploreTask(()=>{const p=new Port();ports.push(p);return p;});
 const kernel=new MockKernel(()=>({type:'explore_context',view_id:'snapshot',context:'pure snapshot'}));
 const view=render(<ExploreView item={item} cellId="cell" fresh kernel={kernel} t={messages('en')} language="en" onInsert={()=>{}} taskFactory={()=>task}/>);
 const slider=screen.getByRole('slider',{name:'Explore parameter a'});
 await waitFor(()=>expect((slider as HTMLInputElement).disabled).toBe(false));
 expect(kernel.requests).toEqual([{type:'get_explore_context',query:{cell_id:'cell',out_index:3,view_id:'snapshot'}}]);
 fireEvent.change(slider,{target:{value:'1'}});await waitFor(()=>expect(ports).toHaveLength(1));
 fireEvent.change(slider,{target:{value:'3'}});await waitFor(()=>expect(ports).toHaveLength(2));
 expect(ports[0]!.terminated).toBe(true);
 await act(async()=>ports[0]!.reply({type:'explored',result:{...item.result,revision:ports[0]!.request!.request.revision,values:{a:1}}}));
 expect(document.querySelector('.output-stale')).toBeTruthy();
 const revision=ports[1]!.request!.request.revision;
 await act(async()=>ports[1]!.reply({type:'explored',result:{...item.result,revision,values:{a:3},item:{type:'expr',out_index:3,input_form:'9.',modern_form:'9.',latex:'9'}}}));
 expect(document.querySelector('.math-latex')?.textContent ?? document.body.textContent).toContain('9');
 fireEvent.change(slider,{target:{value:'4'}});await waitFor(()=>expect(ports).toHaveLength(3));
 fireEvent.click(screen.getByRole('button',{name:'Cancel'}));
 expect(ports[2]!.terminated).toBe(true);
 expect(document.body.textContent).toContain('Below: last successful parameter result');
 view.unmount();
 expect(kernel.requests).toHaveLength(1); // Main notebook never receives slider evaluation, assignments or interrupt.
});
