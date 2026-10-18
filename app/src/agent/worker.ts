/// <reference lib="webworker" />
import init from '../kernel/wasm/om_wasm.js';
import {BrowserAgentRuntime} from './runtime';
import {WorkerToolPort} from './rpc';
import {AgentStore} from './store';
import type {BrowserAgentTask} from './runtimeTypes';
let running:{id:string;runtime:BrowserAgentRuntime}|undefined;
self.onmessage=(event:MessageEvent<{type:'start';task:BrowserAgentTask;port:MessagePort}|{type:'stop';taskId:string}>)=>{
  const packet=event.data;
  if(packet.type==='stop'){if(running?.id===packet.taskId)running.runtime.stop();return;}
  if(running){packet.port.close();return;}
  const runtime=new BrowserAgentRuntime();running={id:packet.task.taskId,runtime};
  const owner=new WorkerToolPort(packet.port),store=new AgentStore();
  void(async()=>{
    try{await init();await runtime.run(packet.task,owner,value=>self.postMessage(value),store);}
    catch{self.postMessage({type:'settled',taskId:packet.task.taskId,reason:'failed',requests:0,tools:0,durationMs:0,error:'Agent worker initialization failed'});}
    finally{owner.close();store.close();running=undefined;}
  })();
};
