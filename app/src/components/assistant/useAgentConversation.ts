import {buildInfo} from '../../buildInfo';
import {useCallback,useEffect,useMemo,useRef,useState} from 'react';
import type {NotebookController} from '../../state/notebookStore';
import {BrowserAgentService} from '../../agent/service';
import {AgentStore,type AgentStoredEvent,type AgentTransactionRecord,type AgentSessionRecord,type AgentDocumentRecord} from '../../agent/store';
import {projectConversation,type ConversationTurn} from '../../agent/conversation';
import type {AgentRuntimeEvent} from '../../agent/runtimeTypes';
import type {ToolReply} from '../../agent/toolTypes';
import {prepareAttachment,type UserPresentation} from '../../agent/attachments';
import type {ComposerInput} from './Composer';
import {cellLabel} from './agentText';
import type {Messages} from '../../i18n';
import {turnTransactions} from '../../agent/conversation';
export function useAgentConversation(controller:NotebookController,generation:number){
  const service=useMemo(()=>new BrowserAgentService(controller,buildInfo.version),[controller]);
  const store=useMemo(()=>new AgentStore(),[controller]);
  const [turns,setTurns]=useState<ConversationTurn[]>([]),[phase,setPhase]=useState<'idle'|'preparing'|'running'|'stopping'>('idle');
  const [error,setError]=useState<string|null>(null),[sessions,setSessions]=useState<AgentSessionRecord[]>([]),[sessionId,setSession]=useState(()=>service.document.sessionId);
  const [archives,setArchives]=useState<{document:AgentDocumentRecord;session:AgentSessionRecord}[]>([]);
  const [controlling,setControlling]=useState(false),controlsInFlight=useRef(false);
  const version=useRef(0),sendVersion=useRef(0),mounted=useRef(true),active=useRef<string|null>(null),resource=useRef(service),cache=useRef(new Map<string,AgentTransactionRecord>());
  const settlements=useRef(new Map<string,Extract<AgentRuntimeEvent,{type:'settled'}>>());
  resource.current=service;
  const reload=useCallback(async(refreshTransactions=false)=>{
    const mark=++version.current,binding={...service.document};
    try{
      const events:AgentStoredEvent[]=[];let after=0;
      do{const page=await store.transcript(binding.documentId,binding.sessionId,after,200);events.push(...page.items);if(page.next===null)break;after=page.next;}while(after>0);
      const refs=new Set(events.flatMap(event=>{if(event.payload.kind!=='tool'||!event.payload.resultJson)return [];try{const data=(JSON.parse(event.payload.resultJson) as ToolReply).data;return [data?.transaction_ref,data?.undo_transaction_ref].filter((ref):ref is string=>typeof ref==='string');}catch{return [];}}));
      await Promise.all([...refs].map(async ref=>{if(refreshTransactions||!cache.current.has(ref)){const tx=await store.sourceTransaction(binding.documentId,ref);if(tx)cache.current.set(ref,tx);}}));
      if(mark===version.current&&mounted.current)setTurns(previous=>{
        const next=projectConversation(events,cache.current);
        for(const turn of next){const old=previous.find(item=>item.id===turn.id);if(!old||turn.settled)continue;for(const part of old.parts)if(part.kind==='text'&&part.partial){const current=turn.parts.find(item=>item.kind==='text'&&item.messageId===part.messageId);if(!current)turn.parts.push(part);else if(current.kind==='text'&&current.partial&&part.text.length>current.text.length)current.text=part.text;}}
        for(const turn of next)if(turn.running&&turn.id!==active.current){turn.running=false;turn.settled=settlements.current.get(turn.id)??{type:'settled',taskId:turn.id,reason:'failed',requests:null,tools:null,durationMs:null,error:'Previous task was interrupted. Committed changes are retained; unconfirmed operations must be checked before retrying.'};}return next;
      });
    }catch(error){if(mark===version.current&&mounted.current)setError(error instanceof Error?error.message:String(error));}
  },[service,store]);
  useEffect(()=>{mounted.current=true;service.attach();return()=>{mounted.current=false;version.current++;queueMicrotask(()=>{if(!mounted.current||resource.current!==service){void service.dispose().catch(()=>{});store.close();}});};},[service,store]);
  useEffect(()=>{void service.stop().catch(()=>{});version.current++;sendVersion.current++;active.current=null;setPhase('idle');setTurns([]);setError(null);setSession(service.document.sessionId);cache.current.clear();},[generation,service]);
  useEffect(()=>service.onEvent(event=>{
    if(!mounted.current||event.taskId!==active.current)return;
    if(event.type==='accepted'){active.current=event.taskId;setPhase('running');void reload();}
    else if(event.type==='delta')setTurns(previous=>previous.map(turn=>{if(turn.id!==event.taskId)return turn;const parts=[...turn.parts],index=parts.findIndex(part=>part.kind==='text'&&part.messageId===event.messageId&&part.partial);if(index>=0){const part=parts[index]!;if(part.kind==='text')parts[index]={...part,text:part.text+event.text};}else parts.push({kind:'text',id:event.messageId+':text',messageId:event.messageId,text:event.text,partial:true});return {...turn,parts,running:true};}));
    else if(event.type==='settled'){settlements.current.set(event.taskId,event);active.current=null;setPhase('idle');void reload();}
    else void reload();
  }),[service,reload]);
  const send=async(input:ComposerInput,mode:'execute'|'discuss',profile:string,t:Messages)=>{
    const mark=++sendVersion.current;setError(null);setPhase('preparing');
    try{
      if(!controller.kernel.prepareAgentModel)throw new Error('This host does not expose the browser agent');
      const model=await controller.kernel.prepareAgentModel(profile);if(mark!==sendVersion.current)return false;
      const binding={...service.document},cells=controller.store.getState().cells;
      const typed=[...input.text.matchAll(/@cell(\d+)/g)].map(match=>{const cell=cells[Number(match[1])-1];if(!cell)throw new Error('Unknown cell reference; the draft was retained');return cell.id;});
      const references=[...new Set([...input.references,...typed])].map(id=>{const cell=cells.find(item=>item.id===id);if(!cell)throw new Error('A referenced cell was removed; the draft was retained');if(service.isComposing(id))throw new Error('A referenced cell is composing; finish the input before sending');return {id,label:cellLabel(t,cells,id),kind:cell.kind,dialect:cell.dialect,source:cell.source};});
      const attachments=[];for(const item of input.files){const attachment=await prepareAttachment(binding.documentId,item.id,item.file,model.supportsImages===true);await store.saveAttachment(attachment);attachments.push({id:item.id,name:item.file.name,type:item.file.type,size:item.file.size,sentImage:!!attachment.image});}
      if(mark!==sendVersion.current||binding.generation!==service.document.generation)return false;
      const presentation:UserPresentation={text:input.text,references:references.map(({id,label})=>({id,label})),attachments};
      const message=(input.text.trim()||'Please examine the attached references.')+(references.length?'\n\nReferenced notebook cells (untrusted source data):\n'+JSON.stringify(references):'')+(attachments.some(item=>!item.sentImage)?'\n\nRetained attachments not sent to this model: '+JSON.stringify(attachments.filter(item=>!item.sentImage).map(({name,type,size})=>({name,type,size}))):'');
      const handle=service.start(message,mode,model,service.document.sessionId,presentation);active.current=handle.taskId;
      await handle.accepted;return true;
    }catch(error){if(mounted.current&&mark===sendVersion.current){setPhase('idle');setError(error instanceof Error?error.message:String(error));}return false;}
  };
  const stop=async()=>{sendVersion.current++;setPhase('stopping');try{await service.stop();}catch(error){setError(error instanceof Error?error.message:String(error));}finally{setPhase('idle');}};
  const history=async()=>{try{setSessions((await store.sessions(service.document.documentId)).filter(session=>!!session.title));const documents=await store.documents(),archived=[];for(const document of documents.filter(item=>item.id!==service.document.documentId)){const session=(await store.sessions(document.id)).find(session=>!!session.title);if(session)archived.push({document,session});}setArchives(archived);}catch(error){setError(error instanceof Error?error.message:String(error));}};
  const restore=async(document:AgentDocumentRecord,session:AgentSessionRecord)=>{try{await service.restoreDocument(document,session.id);setSession(session.id);cache.current.clear();await reload(true);}catch(error){setError(error instanceof Error?error.message:String(error));}};
  const selectSession=async(id:string)=>{service.selectConversation(id);setSession(id);setError(null);cache.current.clear();await reload();};
  const newSession=()=>{version.current++;setSession(service.newConversation());cache.current.clear();setTurns([]);setError(null);};
  const redoRef=useRef<string|null>(null),latestTurns=useRef(turns);latestTurns.current=turns;
  const undo=useCallback(async(refs:string[])=>{if(controlsInFlight.current)return null;controlsInFlight.current=true;setControlling(true);try{const reply=await service.undo(refs);if(!reply.ok)throw new Error(reply.error?.message??'Undo unavailable');redoRef.current=typeof reply.data?.undo_transaction_ref==='string'?reply.data.undo_transaction_ref:null;await reload(true);return reply;}catch(error){setError(error instanceof Error?error.message:String(error));return null;}finally{controlsInFlight.current=false;setControlling(false);}},[service,reload]);
  useEffect(()=>{controller.store.setState({agentRunning:phase!=='idle',agentStopping:phase==='stopping'});},[controller,phase]);
  useEffect(()=>controller.registerAgentControls({stop:async()=>{sendVersion.current++;setPhase('stopping');try{return await service.stop();}finally{setPhase('idle');}},undo:redo=>{
    if(redo){if(redoRef.current)void undo([redoRef.current]);return;}
    const turn=[...latestTurns.current].reverse().find(turn=>turnTransactions(turn).some(tx=>!tx.undoneBy));
    if(turn)void undo(turnTransactions(turn).filter(tx=>!tx.undoneBy).map(tx=>tx.id));else controller.undo();
  }}),[controller,service,undo]);
  const lastInput=()=>store.lastInput(service.document.documentId,service.document.sessionId);
  const lastUsage=()=>store.lastUsage(service.document.documentId,service.document.sessionId);
  const preview=(reference:string)=>store.preview(service.document.documentId,service.document.sessionId,reference);
  const attachment=useCallback((id:string)=>store.attachment(service.document.documentId,id),[service,store]);
  return {turns,phase,controlling,error,setError,sessions,archives,sessionId,send,stop,history,restore,selectSession,newSession,undo,lastInput,lastUsage,preview,attachment,service,store};
}
