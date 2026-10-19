import {EditorView} from '@codemirror/view';
import type {NotebookController} from '../state/notebookStore';
import {NotebookToolPort,observeNotebookSource} from './toolPort';
import {bindToolPort} from './rpc';
import type {AgentRuntimeEvent,BrowserAgentModel,BrowserAgentTask} from './runtimeTypes';
import type {AgentCanonicalMessage} from './modelTypes';
import type {ToolReply} from './toolTypes';
import type {UserPresentation} from './attachments';
import {AgentStore,type AgentDocumentRecord} from './store';
async function withDocumentLock<T>(id:string,run:()=>Promise<T>):Promise<T>{
  if(!navigator.locks)return run();
  return navigator.locks.request('openmath.agent.document.'+id,{ifAvailable:true},async lock=>{if(!lock)throw new Error('This notebook is being used by another agent tab');return run();});
}
export interface AgentTaskHandle {taskId:string;accepted:Promise<AgentCanonicalMessage>;finished:Promise<Extract<AgentRuntimeEvent,{type:'settled'}>>}
function deferred<T>(){let resolve!:(value:T)=>void,reject!:(error:Error)=>void;const promise=new Promise<T>((yes,no)=>{resolve=yes;reject=no;});return {promise,resolve,reject};}
/** Lazy worker lifecycle; settings/credentials are frozen once and never exposed in UI events. */
export class BrowserAgentService {
  private binding:{generation:number;documentId:string;sessionId:string}|undefined;
  private active:{taskId:string;worker:Worker;owner:NotebookToolPort;closePort:()=>void;handle:AgentTaskHandle;cancelRequested:boolean}|undefined;
  private listeners=new Set<(event:AgentRuntimeEvent)=>void>();
  private control=false;
  private attached=false;
  private compositions=new Set<Element>();
  private compositionStart=(event:CompositionEvent)=>{if(event.target instanceof Element&&event.target.closest('.notebook-cell[data-cell-id]'))this.compositions.add(event.target);};
  private compositionEnd=(event:CompositionEvent)=>{if(event.target instanceof Element)this.compositions.delete(event.target);};
  constructor(private controller:NotebookController,private version:string){}
  attach(){if(this.attached)return;this.attached=true;observeNotebookSource(this.controller);document.addEventListener('compositionstart',this.compositionStart,true);document.addEventListener('compositionend',this.compositionEnd,true);}
  private composing(){
    const ids=new Set<string>();
    for(const target of this.compositions){if(!target.isConnected){this.compositions.delete(target);continue;}const cell=target.closest<HTMLElement>('.notebook-cell[data-cell-id]');if(cell?.dataset.cellId)ids.add(cell.dataset.cellId);}
    for(const cell of document.querySelectorAll<HTMLElement>('.notebook-cell[data-cell-id]')){const editor=cell.querySelector<HTMLElement>('.cm-editor');if(editor&&EditorView.findFromDOM(editor)?.composing)ids.add(cell.dataset.cellId!);}
    return [...ids];
  }
  onEvent(listener:(event:AgentRuntimeEvent)=>void){this.listeners.add(listener);return()=>{this.listeners.delete(listener);};}
  isComposing(id:string){return this.composing().includes(id);}
  private emit(event:AgentRuntimeEvent){for(const listener of [...this.listeners]){try{listener(event);}catch{/* An independent view cannot change task settlement. */}}}
  get document(){
    const generation=this.controller.store.getState().documentGeneration;
    if(this.binding?.generation!==generation)this.binding={generation,documentId:'web-document-'+crypto.randomUUID(),sessionId:'session-'+crypto.randomUUID()};
    return this.binding;
  }
  newConversation(){if(this.active||this.control)throw new Error('An agent task is running');this.document.sessionId='session-'+crypto.randomUUID();return this.document.sessionId;}
  selectConversation(sessionId:string){if(this.active||this.control)throw new Error('An agent task is running');this.document.sessionId=sessionId;}
  async restoreDocument(record:AgentDocumentRecord,sessionId:string){
    this.attach();
    if(this.active||this.control)throw new Error('An agent task is running');if(!navigator.locks)throw new Error('Restoring a conversation requires browser document locks');
    await withDocumentLock(record.id,async()=>{const store=new AgentStore();try{const latest=await store.document(record.id);if(!latest||latest.updatedAt!==record.updatedAt)throw new Error('The stored notebook changed; reopen history before restoring');await this.controller.load(record.file);const generation=Math.max(this.controller.store.getState().documentGeneration,record.generation+1);this.controller.store.setState({documentGeneration:generation,dirty:true,savedSource:undefined});this.binding={generation,documentId:record.id,sessionId};await store.putDocument({...record,generation,revision:0,updatedAt:Date.now()});}finally{store.close();}});
  }
  undo(refs:string[]):Promise<ToolReply>{
    this.attach();
    if(this.active||this.control)return Promise.reject(new Error('An agent task is running'));this.control=true;
    const binding={...this.document},owner=new NotebookToolPort(this.controller,binding.documentId,binding.generation,true,()=>this.composing()),channel=new MessageChannel(),close=bindToolPort(channel.port1,owner);let worker:Worker;
    try{worker=new Worker(new URL('./controlWorker.ts',import.meta.url),{type:'module'});}catch(error){this.control=false;close();owner.dispose();return Promise.reject(error);}
    return withDocumentLock(binding.documentId,()=>new Promise<ToolReply>((resolve,reject)=>{worker.onmessage=(event:MessageEvent<{reply?:ToolReply;error?:string}>)=>{if(event.data.reply)resolve(event.data.reply);else reject(new Error(event.data.error??'Undo unavailable'));};worker.onerror=()=>reject(new Error('Undo outcome is unknown; query its operation before retrying'));worker.postMessage({...binding,taskId:'control-'+crypto.randomUUID(),refs,port:channel.port2},[channel.port2]);})).finally(()=>{this.control=false;close();owner.dispose();worker.terminate();});
  }
  start(text:string,mode:'execute'|'discuss',model:BrowserAgentModel,sessionId=this.document.sessionId,presentation?:UserPresentation):AgentTaskHandle{
    this.attach();
    if(this.active||this.control)throw new Error('An agent task is running');
    if(!text.trim())throw new Error('Message is empty');
    if(mode==='execute'&&!model.supportsTools)throw new Error('This model does not support tools');
    const binding=this.document,taskId='task-'+crypto.randomUUID();
    const user:AgentCanonicalMessage={record_type:'canonical_agent_message',message_version:1,message_id:'user-'+crypto.randomUUID(),role:'user',task_id:taskId,agent_turn_id:null,parts:[{kind:'text',part_id:'user-text',text},...(presentation?.attachments??[]).filter(item=>item.sentImage).map(item=>({kind:'media' as const,part_id:item.id,media_ref:item.id,submission_kind:'image' as const}))]};
    const accepted=deferred<AgentCanonicalMessage>(),finished=deferred<Extract<AgentRuntimeEvent,{type:'settled'}>>(),started=performance.now();
    void accepted.promise.catch(()=>{});void finished.promise.catch(()=>{});
    const handle={taskId,accepted:accepted.promise,finished:finished.promise};
    const owner=new NotebookToolPort(this.controller,binding.documentId,binding.generation,mode==='execute',()=>this.composing());
    const channel=new MessageChannel(),closePort=bindToolPort(channel.port1,owner);
    let worker:Worker;
    try{worker=new Worker(new URL('./worker.ts',import.meta.url),{type:'module'});}catch(error){closePort();owner.dispose();throw error;}
    const task:BrowserAgentTask={taskId,sessionId,documentId:binding.documentId,generation:binding.generation,user,mode,version:this.version,model:structuredClone(model)};
    if(presentation)task.presentation=structuredClone(presentation);
    const finish=(event:Extract<AgentRuntimeEvent,{type:'settled'}>)=>{
      if(this.active?.taskId!==taskId)return;
      this.active=undefined;closePort();owner.dispose();worker.terminate();
      accepted.reject(new Error(event.error??'Agent input was not accepted'));finished.resolve(event);this.emit(event);
    };
    worker.onmessage=(event:MessageEvent<AgentRuntimeEvent>)=>{
      if(event.data.taskId!==taskId||this.active?.taskId!==taskId)return;
      if(event.data.type==='accepted')accepted.resolve(event.data.message);
      if(event.data.type==='settled')finish(event.data);else this.emit(event.data);
    };
    worker.onerror=()=>{void owner.cancelOwnedWork().catch(()=>{}).then(()=>finish({type:'settled',taskId,reason:'failed',requests:null,tools:null,durationMs:performance.now()-started,error:'Agent worker failed; committed changes are retained'}));};
    this.active={taskId,worker,owner,closePort,handle,cancelRequested:false};void withDocumentLock(binding.documentId,async()=>{if(this.active?.cancelRequested)throw new DOMException('Agent input cancelled before acceptance','AbortError');worker.postMessage({type:'start',task,port:channel.port2},[channel.port2]);await finished.promise;}).catch(error=>finish({type:'settled',taskId,reason:this.active?.cancelRequested?'stopped':'failed',requests:0,tools:0,durationMs:performance.now()-started,error:error instanceof Error?error.message:String(error)}));return handle;
  }
  async stop(){const active=this.active;if(!active)return;active.cancelRequested=true;active.worker.postMessage({type:'stop',taskId:active.taskId});await active.owner.cancelOwnedWork();return active.handle.finished;}
  async dispose(){try{await this.stop();}finally{document.removeEventListener('compositionstart',this.compositionStart,true);document.removeEventListener('compositionend',this.compositionEnd,true);this.attached=false;this.compositions.clear();this.listeners.clear();}}
}
