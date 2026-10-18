import {expect,test,vi} from 'vitest';
import {NotebookToolPort,type ToolSnapshot} from './toolPort';
import {NotebookController} from '../state/notebookStore';
import {MockKernel} from '../test/mockKernel';
import type {KernelClient} from '../kernel/client';
import type {KernelConfig} from '../kernel/generated/KernelConfig';
test('stopping an unadmitted agent computation never interrupts existing manual work',async()=>{
  const kernel=new MockKernel(()=>({type:'ok'})) as MockKernel&{agentMath:NonNullable<KernelClient['agentMath']>};
  kernel.agentMath=vi.fn(async()=>({type:'scratch' as const,status:'Done' as const,output:null,notebook_modified:false as const}));
  const interrupt=vi.spyOn(kernel,'interrupt'),controller=new NotebookController(kernel);
  const general:KernelConfig['general']={language:'en',dialect:'modern',constants:'math',reactive:true,auto_run_dependents:true,show_steps:true,auto_plot:false,eval_timeout_ms:30000};
  controller.store.setState({busy:true,config:{general,llm:{enabled:false,translate:'',explain:'',complete:'',chat:'',fix:'',send_context:false,profiles:[]}}});
  const owner=new NotebookToolPort(controller,'doc',0,true),snapshot:ToolSnapshot={document:{id:'doc',generation:0,revision:0},file:controller.file(),general,cells:[],composing:[],busy:false};
  const request=owner.math(snapshot,{type:'scratch',source:'2+2',dialect:'Modern',use_notebook_definitions:false});
  const rejected=expect(request).rejects.toThrow();
  await owner.cancelOwnedWork();await rejected;
  expect(interrupt).not.toHaveBeenCalled();expect(kernel.agentMath).not.toHaveBeenCalled();expect(controller.store.getState().busy).toBe(true);
  owner.dispose();controller.dispose();
});

test('an admitted scratch drains its bounded clone without restarting the parent kernel on stop',async()=>{
  let finish!:(value:import('./hostTypes').MathResult)=>void;
  const kernel=new MockKernel(()=>({type:'ok'})) as MockKernel&{agentMath:NonNullable<KernelClient['agentMath']>};
  kernel.agentMath=vi.fn(()=>new Promise<import('./hostTypes').MathResult>(resolve=>{finish=resolve;}));
  const interrupt=vi.spyOn(kernel,'interrupt'),controller=new NotebookController(kernel);
  const general:KernelConfig['general']={language:'en',dialect:'modern',constants:'math',reactive:true,auto_run_dependents:true,show_steps:true,auto_plot:false,eval_timeout_ms:30000};
  controller.store.setState({config:{general,llm:{enabled:false,translate:'',explain:'',complete:'',chat:'',fix:'',send_context:false,profiles:[]}}});
  const owner=new NotebookToolPort(controller,'doc',0,false),snapshot:ToolSnapshot={document:{id:'doc',generation:0,revision:0},file:controller.file(),general,cells:[],composing:[],busy:false};
  const request=owner.math(snapshot,{type:'scratch',source:'2+2',dialect:'Modern',use_notebook_definitions:false});
  await vi.waitFor(()=>expect(kernel.agentMath).toHaveBeenCalledOnce());await owner.cancelOwnedWork();
  expect(interrupt).not.toHaveBeenCalled();expect(controller.store.getState().busy).toBe(true);
  finish({type:'scratch',status:'Done',output:null,notebook_modified:false});await request;
  expect(controller.store.getState().busy).toBe(false);expect(controller.file().cells).toEqual([]);
  owner.dispose();controller.dispose();
});
