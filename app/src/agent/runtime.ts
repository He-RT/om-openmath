import {Agent,type AgentTool} from '@earendil-works/pi-agent-core';
import type {Api,Model,AssistantMessage} from '@earendil-works/pi-ai';
import {BrowserModelAdapter} from './modelAdapter';
import {AgentStore,type AgentEventPayload} from './store';
import {ToolHost} from './toolHost';
import type {ToolPort} from './toolPort';
import type {BrowserAgentTask,AgentRuntimeEvent} from './runtimeTypes';
import type {CanonicalRecord,AgentCanonicalMessage,ModelInput} from './modelTypes';
import type {AgentMessagePart} from '../../../agent/src/generated/protocol';
import {agentSystemPrompt} from './prompts';

/** Pi owns scheduling; canonical records and host tool receipts own all business facts. */
export class BrowserAgentRuntime{
  private abort=new AbortController();private agent:Agent|undefined;
  stop(){this.abort.abort();this.agent?.abort();}
  async run(task:BrowserAgentTask,port:ToolPort,emit:(event:AgentRuntimeEvent)=>void,store=new AgentStore()){
    const started=performance.now(),signal=this.abort.signal,host=new ToolHost(port,store,{documentId:task.documentId,generation:task.generation,sessionId:task.sessionId,taskId:task.taskId,mode:task.mode,signal});
    let requests=0,tools=0,budget=false,failure:string|null=null;
    const limitRequests=task.maxModelRequests??32,limitTools=task.maxToolCalls??80,duration=task.maxDurationMs??300000;
    const timer=setTimeout(()=>{budget=true;this.stop();},duration);
    const history:CanonicalRecord[]=[],results:ModelInput['results']=[],persisted=new Set<string>();
    let evidence=Promise.resolve();
    const recordEvidence=(payload:AgentEventPayload)=>{evidence=evidence.then(()=>store.append(task.documentId,task.sessionId,payload)).then(()=>{});void evidence.catch(()=>{});};
    const adapter=new BrowserModelAdapter(result=>recordEvidence({kind:'model_result',result}),undefined,round=>{requests++;recordEvidence({kind:'dispatch',taskId:task.taskId,turnId:round.turnId,requestId:round.requestId,time:Date.now()});});
    const check=()=>{signal.throwIfAborted();};
    const persist=async(message:AssistantMessage)=>{
      if(message.stopReason==='error'||message.stopReason==='aborted')return;
      const result=adapter.result(message);await evidence;
      if(persisted.has(result.message.message_id))return;
      const record={canonical:result.message,replay:result.replay};
      await store.append(task.documentId,task.sessionId,{kind:'message',record,modelId:task.model.apiModel,mode:task.mode});
      history.push(record);persisted.add(result.message.message_id);
      emit({type:'assistant',taskId:task.taskId,message:result.message,usage:result.usage});
    };
    const originalCalls=new Map<string,{part:Extract<AgentMessagePart,{kind:'tool_call'}>;turnId:string|null}>();
    try{
      if(task.user.role!=='user'||task.user.task_id!==task.taskId||task.user.message_id.length>256)throw new Error('Invalid agent input scope');
      await host.initialize();
      const stored:CanonicalRecord[]=[];const texts=new Map<string,{text:string;isError:boolean}>();let after=0;
      do{
        const page=await store.events(task.documentId,task.sessionId,after,100);
        for(const event of page.items){if(event.payload.kind==='message')stored.push(event.payload.record);else if(event.payload.kind==='tool'&&event.payload.resultJson){const reply=JSON.parse(event.payload.resultJson) as import('./toolTypes').ToolReply;const isError=!reply.ok||event.payload.name==='preview_source'&&reply.data?.valid===false||event.payload.name==='evaluate_scratch'&&reply.data?.evaluation_status==='error';texts.set(event.payload.callId,{text:event.payload.resultJson,isError});}}
        if(stored.length>1024)throw new Error('Conversation exceeds the current context limit');
        if(page.next===null)break;after=page.next;
      }while(after>0);
      const paired=new Set(stored.flatMap(record=>record.canonical.role==='tool'?record.canonical.parts.filter((part):part is Extract<AgentMessagePart,{kind:'tool_result'}>=>part.kind==='tool_result').map(part=>part.tool_call_id):[]));
      for(const record of stored){
        if(record.canonical.role==='system')continue;
        if(record.canonical.role==='assistant'&&record.canonical.parts.some(part=>part.kind==='tool_call'&&!paired.has(part.tool_call_id)))continue;
        if(record.canonical.role==='tool'){
          for(const part of record.canonical.parts){if(part.kind!=='tool_result')continue;const result=texts.get(part.tool_call_id);if(!result)throw new Error('Stored tool result is unavailable');results.push({message_id:record.canonical.message_id,tool_call_id:part.tool_call_id,text:result.text,is_error:result.isError});}
        }
        history.push(record);
      }
      if(history.some(record=>record.canonical.message_id===task.user.message_id))throw new Error('This user input has already been accepted');
      const system:AgentCanonicalMessage={record_type:'canonical_agent_message',message_version:1,message_id:'system-'+crypto.randomUUID(),role:'system',task_id:task.taskId,agent_turn_id:null,parts:[{kind:'text',part_id:'system-text',text:agentSystemPrompt(task.version,task.mode)}]};
      history.unshift({canonical:system,replay:null});
      await store.append(task.documentId,task.sessionId,{kind:'message',record:{canonical:task.user,replay:null},modelId:task.model.apiModel,mode:task.mode});
      history.push({canonical:task.user,replay:null});await store.append(task.documentId,task.sessionId,{kind:'turn',taskId:task.taskId,mode:task.mode,phase:'started',time:Date.now()});
      emit({type:'accepted',taskId:task.taskId,message:task.user});
      const declarations=task.model.supportsTools?host.declarations():[];
      const agentTools:AgentTool[]=declarations.map(tool=>({name:tool.name,label:tool.label,description:tool.description,
        // Full closed schemas are sent in ModelInput; Pi's internal gate only preserves
        // the original object so the host can record malformed-argument feedback too.
        parameters:{type:'object',properties:{},additionalProperties:true} as AgentTool['parameters'],
        execute:async(callId)=>{
          check();if(tools>=limitTools){budget=true;throw new Error('Agent tool budget reached');}tools++;const original=originalCalls.get(callId);if(!original||original.part.name!==tool.name)throw new Error('Tool proposal is not durably acknowledged');
          const reply=await host.invoke(tool.name,original.part.arguments_json,callId);
          const isError=!reply.ok||tool.name==='preview_source'&&reply.data?.valid===false||tool.name==='evaluate_scratch'&&reply.data?.evaluation_status==='error';
          const outcome=reply.status;
          const message:AgentCanonicalMessage={record_type:'canonical_agent_message',message_version:1,message_id:'tool-'+crypto.randomUUID(),role:'tool',task_id:task.taskId,agent_turn_id:original.turnId,parts:[{kind:'tool_result',part_id:'result-'+crypto.randomUUID(),tool_call_id:callId,operation_ref:reply.operation_ref,result_ref:reply.request_ref,outcome}]};
          await store.append(task.documentId,task.sessionId,{kind:'message',record:{canonical:message,replay:null}});
          history.push({canonical:message,replay:null});results.push({message_id:message.message_id,tool_call_id:callId,text:JSON.stringify(reply),is_error:isError});
          emit({type:'tool',taskId:task.taskId,callId,name:tool.name,reply});
          return {content:[{type:'text',text:JSON.stringify(reply)}],details:{reply},isError,terminate:outcome==='unknown'||outcome==='cancelled'};
        }}));
      const model:Model<Api>={id:task.model.apiModel,name:task.model.displayName,api:'openai-completions',provider:'openmath-host',baseUrl:'',reasoning:false,input:['text'],contextWindow:task.model.contextWindow??0,maxTokens:task.model.maxOutputTokens,cost:{input:0,output:0,cacheRead:0,cacheWrite:0}};
      this.agent=new Agent({initialState:{model,tools:agentTools},toolExecution:'sequential',getApiKey:()=>undefined,maxRetryDelayMs:0,streamFn:adapter.stream,
        prepareRequest:async()=>{
          check();if(requests>=limitRequests){budget=true;throw new Error('Agent model budget reached');}await evidence;const turnId='turn-'+crypto.randomUUID(),requestId='request-'+crypto.randomUUID();
          const input:ModelInput={binding:task.model.binding,api_model:task.model.apiModel,messages:structuredClone(history),results:structuredClone(results),tools:declarations.map(({name,description,parameters})=>({name,description,parameters})),max_output_tokens:task.model.maxOutputTokens,temperature:null,stream:true};
          await store.append(task.documentId,task.sessionId,{kind:'input',taskId:task.taskId,turnId,input});
          signal.throwIfAborted();adapter.prepare({requestId,messageId:'assistant-'+crypto.randomUUID(),taskId:task.taskId,turnId,input,url:task.model.url,headers:task.model.headers,timeoutMs:task.model.timeoutMs});
        },beforeToolCall:async(context)=>{
          signal.throwIfAborted();await persist(context.assistantMessage);
          const result=adapter.result(context.assistantMessage);
          const part=result.message.parts.find((part):part is Extract<AgentMessagePart,{kind:'tool_call'}>=>part.kind==='tool_call'&&part.tool_call_id===context.toolCall.id);
          if(!part)throw new Error('Original tool call is unavailable');originalCalls.set(part.tool_call_id,{part,turnId:result.message.agent_turn_id});
        },finishTurn:async(context)=>{await persist(context.message);}});
      if(signal.aborted)this.agent.abort();
      await this.agent.prompt({role:'user',content:task.user.parts.filter((part):part is Extract<AgentMessagePart,{kind:'text'}>=>part.kind==='text').map(part=>({type:'text',text:part.text})),timestamp:Date.now()});
      const last=this.agent.state.messages.at(-1);
      if(last?.role==='assistant'&&(last.stopReason==='error'||last.stopReason==='aborted'))failure=last.errorMessage??'Agent model request failed';
    }catch(error){failure=error instanceof Error?error.message:String(error);}
    finally{
      clearTimeout(timer);let reason:Extract<AgentRuntimeEvent,{type:'settled'}>['reason']=budget?'budget':signal.aborted?'stopped':failure?'failed':'ended';
      try{await store.append(task.documentId,task.sessionId,{kind:'turn',taskId:task.taskId,mode:task.mode,phase:reason,time:Date.now()});}catch{failure='Could not persist final agent status';reason='failed';}
      host.dispose();emit({type:'settled',taskId:task.taskId,reason,requests,tools,durationMs:performance.now()-started,error:failure});
    }
  }
}
