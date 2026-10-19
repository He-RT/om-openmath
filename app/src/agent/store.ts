import type {NotebookFile} from '../kernel/generated/NotebookFile';
import type {CanonicalRecord,ModelInput,ModelResult,AgentCanonicalMessage} from './modelTypes';
import {sameNotebookSource} from '../state/sourceTransaction';
import type {StoredAttachment,UserPresentation} from './attachments';
export interface AgentDocumentRecord {id:string;generation:number;revision:number;file:NotebookFile;updatedAt:number}
export interface AgentSessionRecord {id:string;documentId:string;title:string;createdAt:number;nextSequence:number}
export type AgentEventPayload =
  | {kind:'message';record:CanonicalRecord;modelId?:string;mode?:'execute'|'discuss'}
  | {kind:'tool';callId:string;name:string;operationRef:string|null;argumentsJson:string;resultJson:string|null}
  | {kind:'turn';taskId:string;mode:'execute'|'discuss';phase:'started'|'ended'|'stopped'|'failed'|'budget';time:number}
  | {kind:'input';taskId:string;turnId:string;input:Omit<ModelInput,'images'>}
  | {kind:'dispatch';taskId:string;turnId:string;requestId:string;time:number}
  | {kind:'delta';taskId:string;turnId:string;messageId:string;text:string}
  | {kind:'preview';reference:string;before:NotebookFile;after:NotebookFile;changedCellIds:string[]}
  | {kind:'settled';taskId:string;mode:'execute'|'discuss';reason:'ended'|'stopped'|'failed'|'budget';requests:number|null;tools:number|null;durationMs:number;error:string|null}
  | {kind:'model_result';result:ModelResult}
  | {kind:'user_input';taskId:string;presentation:UserPresentation};
export interface AgentStoredEvent {id:string;documentId:string;sessionId:string;sequence:number;time:number;payload:AgentEventPayload}
export interface AgentOperationRecord {
  id:string;documentId:string;sessionId:string;taskId:string;tool:string;inputHash:string;
  state:'prepared'|'running'|'completed'|'failed'|'cancelled'|'unknown';
  effectCommitted:boolean|null;transactionRef:string|null;resultJson:string|null;resultRefs:string[];updatedAt:number;
  intent?:{transaction:AgentTransactionRecord;planHash:string};
}
export interface AgentTransactionRecord {id:string;documentId:string;sessionId:string;taskId:string;before:NotebookFile;after:NotebookFile;changedCellIds:string[];createdAt:number;undoneBy:string|null;undoes?:string[]}
const terminal=(state:AgentOperationRecord['state'])=>state==='completed'||state==='failed'||state==='cancelled';
const olderDocument=(old:AgentDocumentRecord|undefined,next:AgentDocumentRecord)=>!!old&&(old.generation>next.generation||old.generation===next.generation&&(old.revision>next.revision||old.revision===next.revision&&!sameNotebookSource(old.file,next.file)));
/** Separate browser journal. No notebook export or model credentials are stored by this API. */
export class AgentStore {
  private database:Promise<IDBDatabase>|undefined;
  private closed=false;
  constructor(private name='openmath.agent.v1'){}
  private open():Promise<IDBDatabase>{
    if(this.closed)return Promise.reject(new Error('Agent store is closed'));
    return this.database??=new Promise((resolve,reject)=>{
      const request=indexedDB.open(this.name,2);
      request.onupgradeneeded=()=>{
        const db=request.result;
        if(!db.objectStoreNames.contains('documents'))db.createObjectStore('documents',{keyPath:'id'});
        for(const name of ['sessions','operations','transactions'])if(!db.objectStoreNames.contains(name)){const store=db.createObjectStore(name,{keyPath:'id'});store.createIndex('document','documentId');if(name==='operations'){store.createIndex('pending',['documentId','state']);store.createIndex('transaction','transactionRef');}}
        const events=db.objectStoreNames.contains('events')?request.transaction!.objectStore('events'):db.createObjectStore('events',{keyPath:['sessionId','sequence']});
        if(!events.indexNames.contains('document'))events.createIndex('document','documentId');if(!events.indexNames.contains('display'))events.createIndex('display',['sessionId','payload.kind','sequence']);
        if(!db.objectStoreNames.contains('attachments')){const media=db.createObjectStore('attachments',{keyPath:'id'});media.createIndex('document','documentId');}
      };
      request.onsuccess=()=>{request.result.onversionchange=()=>request.result.close();resolve(request.result);};
      request.onerror=()=>reject(new Error('Could not open agent storage'));
      request.onblocked=()=>reject(new Error('Agent storage is busy in another tab'));
    });
  }
  private async transaction<T>(names:string[],mode:IDBTransactionMode,body:(tx:IDBTransaction,set:(value:T)=>void,fail:(error:Error)=>void)=>void):Promise<T>{
    const db=await this.open();
    return new Promise<T>((resolve,reject)=>{
      const tx=db.transaction(names,mode,mode==='readwrite'?{durability:'strict'}:undefined);
      let value:T, failure:Error|undefined;
      const fail=(error:Error)=>{failure=error;tx.abort();};
      tx.oncomplete=()=>resolve(value);tx.onabort=()=>reject(failure??new Error('Agent storage transaction failed'));
      tx.onerror=()=>{failure??=new Error('Agent storage transaction failed');};
      try{body(tx,result=>{value=result;},fail);}catch(error){fail(error instanceof Error?error:new Error('Agent storage write failed'));}
    });
  }
  async putDocument(document:AgentDocumentRecord):Promise<void>{
    const frozen=structuredClone(document);
    await this.transaction<void>(['documents'],'readwrite',(tx,set,fail)=>{
      const store=tx.objectStore('documents'),request=store.get(frozen.id);
      request.onsuccess=()=>{if(olderDocument(request.result as AgentDocumentRecord|undefined,frozen)){fail(new Error('STALE_SNAPSHOT'));return;}store.put(frozen);set();};
    });
  }
  document(id:string):Promise<AgentDocumentRecord|undefined>{
    return this.transaction(['documents'],'readonly',(tx,set)=>{const request=tx.objectStore('documents').get(id);request.onsuccess=()=>set(request.result as AgentDocumentRecord|undefined);});
  }
  documents():Promise<AgentDocumentRecord[]>{return this.transaction(['documents'],'readonly',(tx,set)=>{const request=tx.objectStore('documents').getAll();request.onsuccess=()=>set((request.result as AgentDocumentRecord[]).sort((a,b)=>b.updatedAt-a.updatedAt));});}
  async createSession(documentId:string,id:string,title:string):Promise<AgentSessionRecord>{
    const record:AgentSessionRecord={id,documentId,title,createdAt:Date.now(),nextSequence:0};
    return this.transaction(['sessions'],'readwrite',(tx,set,fail)=>{
      const store=tx.objectStore('sessions'),existing=store.get(id);
      existing.onsuccess=()=>{
        const old=existing.result as AgentSessionRecord|undefined;
        if(old&&old.documentId!==documentId){fail(new Error('Session belongs to another document'));return;}
        if(!old)store.add(record);set(old??record);
      };
    });
  }
  sessions(documentId:string):Promise<AgentSessionRecord[]>{
    return this.transaction(['sessions'],'readonly',(tx,set)=>{const request=tx.objectStore('sessions').index('document').getAll(documentId);request.onsuccess=()=>set((request.result as AgentSessionRecord[]).sort((a,b)=>b.createdAt-a.createdAt));});
  }
  setSessionTitle(documentId:string,id:string,title:string):Promise<void>{
    return this.transaction(['sessions'],'readwrite',(tx,set,fail)=>{const store=tx.objectStore('sessions'),request=store.get(id);request.onsuccess=()=>{const session=request.result as AgentSessionRecord|undefined;if(!session||session.documentId!==documentId){fail(new Error('Session belongs to another document'));return;}if(!session.title)store.put({...session,title:Array.from(title).slice(0,20).join('')});set();};});
  }
  acceptInput(documentId:string,sessionId:string,user:AgentCanonicalMessage,modelId:string,mode:'execute'|'discuss',presentation?:UserPresentation):Promise<void>{
    const payloads:AgentEventPayload[]=[];if(presentation)payloads.push({kind:'user_input',taskId:user.task_id!,presentation:structuredClone(presentation)});
    payloads.push({kind:'message',record:{canonical:structuredClone(user),replay:null},modelId,mode},{kind:'turn',taskId:user.task_id!,mode,phase:'started',time:Date.now()});
    return this.transaction(['sessions','events'],'readwrite',(tx,set,fail)=>{const sessions=tx.objectStore('sessions'),request=sessions.get(sessionId);request.onsuccess=()=>{
      const session=request.result as AgentSessionRecord|undefined;if(!session||session.documentId!==documentId||user.role!=='user'||!user.task_id){fail(new Error('Invalid user input scope'));return;}
      const nextSequence=session.nextSequence+payloads.length;if(!Number.isSafeInteger(nextSequence)){fail(new Error('Session sequence exceeds limit'));return;}
      payloads.forEach((payload,index)=>tx.objectStore('events').add({id:crypto.randomUUID(),documentId,sessionId,sequence:session.nextSequence+index+1,time:Date.now(),payload}));
      const title=presentation?.text||user.parts.filter(part=>part.kind==='text').map(part=>part.text).join('');sessions.put({...session,nextSequence,title:session.title||Array.from(title).slice(0,20).join('')});set();
    };});
  }
  append(documentId:string,sessionId:string,payload:AgentEventPayload):Promise<AgentStoredEvent>{
    const frozen=structuredClone(payload);
    // Whitelist body fields for the input snapshot. Prepared HTTP headers/URL never enter it.
    if(frozen.kind==='input'){
      const {binding,api_model,messages,results,tools,max_output_tokens,temperature,stream}=frozen.input;
      frozen.input={binding,api_model,messages,results,tools,max_output_tokens,temperature,stream};
    }
    return this.transaction(['sessions','events'],'readwrite',(tx,set,fail)=>{
      const sessions=tx.objectStore('sessions'),request=sessions.get(sessionId);
      request.onsuccess=()=>{
        const session=request.result as AgentSessionRecord|undefined;
        if(!session||session.documentId!==documentId){fail(new Error('Session belongs to another document'));return;}
        const sequence=session.nextSequence+1;
        if(!Number.isSafeInteger(sequence)){fail(new Error('Session sequence exceeds limit'));return;}
        const event:AgentStoredEvent={id:crypto.randomUUID(),documentId,sessionId,sequence,time:Date.now(),payload:frozen};
        tx.objectStore('events').add(event);sessions.put({...session,nextSequence:sequence});set(event);
      };
    });
  }
  events(documentId:string,sessionId:string,after=0,limit=100):Promise<{items:AgentStoredEvent[];next:number|null}>{
    if(!Number.isSafeInteger(after)||after<0||!Number.isInteger(limit)||limit<1||limit>500)return Promise.reject(new Error('Invalid event page'));
    return this.transaction(['sessions','events'],'readonly',(tx,set,fail)=>{
      const request=tx.objectStore('sessions').get(sessionId);
      request.onsuccess=()=>{
        const session=request.result as AgentSessionRecord|undefined;
        if(!session||session.documentId!==documentId){fail(new Error('Session belongs to another document'));return;}
        if(after>=session.nextSequence){set({items:[],next:null});return;}
        const items:AgentStoredEvent[]=[],cursor=tx.objectStore('events').openCursor(IDBKeyRange.bound([sessionId,after+1],[sessionId,Number.MAX_SAFE_INTEGER]));
        cursor.onsuccess=()=>{
          const row=cursor.result;
          if(row&&items.length<limit){items.push(row.value as AgentStoredEvent);row.continue();return;}
          set({items,next:row&&items.length?items.at(-1)!.sequence:null});
        };
      };
    });
  }
  transcript(documentId:string,sessionId:string,after=0,limit=200):Promise<{items:AgentStoredEvent[];next:number|null}>{
    if(!Number.isSafeInteger(after)||after<0||!Number.isInteger(limit)||limit<1||limit>500)return Promise.reject(new Error('Invalid event page'));
    return this.transaction(['sessions','events'],'readonly',(tx,set,fail)=>{const request=tx.objectStore('sessions').get(sessionId);request.onsuccess=()=>{
      const session=request.result as AgentSessionRecord|undefined;if(!session||session.documentId!==documentId){fail(new Error('Session belongs to another document'));return;}
      const store=tx.objectStore('events'),kinds=['message','tool','turn','delta','settled','user_input'],items:AgentStoredEvent[]=[];
      if(store.indexNames.contains('display')){let remaining=kinds.length;for(const kind of kinds){const page=store.index('display').getAll(IDBKeyRange.bound([sessionId,kind,after+1],[sessionId,kind,Number.MAX_SAFE_INTEGER]),limit+1);page.onsuccess=()=>{items.push(...page.result as AgentStoredEvent[]);if(--remaining===0){items.sort((a,b)=>a.sequence-b.sequence);set({items:items.slice(0,limit),next:items.length>limit?items[limit-1]!.sequence:null});}};}}
      else{const cursor=store.openCursor(IDBKeyRange.bound([sessionId,after+1],[sessionId,Number.MAX_SAFE_INTEGER]));cursor.onsuccess=()=>{const row=cursor.result;if(!row){set({items,next:null});return;}const event=row.value as AgentStoredEvent;if(kinds.includes(event.payload.kind))items.push(event);if(items.length>limit){set({items:items.slice(0,limit),next:items[limit-1]!.sequence});return;}row.continue();};}
    };});
  }
  lastInput(documentId:string,sessionId:string):Promise<Extract<AgentEventPayload,{kind:'input'}>|undefined>{
    return this.transaction(['sessions','events'],'readonly',(tx,set,fail)=>{const request=tx.objectStore('sessions').get(sessionId);request.onsuccess=()=>{const session=request.result as AgentSessionRecord|undefined;if(!session||session.documentId!==documentId){fail(new Error('Session belongs to another document'));return;}const cursor=tx.objectStore('events').openCursor(IDBKeyRange.bound([sessionId,0],[sessionId,Number.MAX_SAFE_INTEGER]),'prev');cursor.onsuccess=()=>{const row=cursor.result;if(!row){set(undefined);return;}const event=row.value as AgentStoredEvent;if(event.payload.kind==='input'){set(event.payload);return;}row.continue();};};});
  }
  preview(documentId:string,sessionId:string,reference:string):Promise<Extract<AgentEventPayload,{kind:'preview'}>|undefined>{
    return this.transaction(['sessions','events'],'readonly',(tx,set,fail)=>{const request=tx.objectStore('sessions').get(sessionId);request.onsuccess=()=>{const session=request.result as AgentSessionRecord|undefined;if(!session||session.documentId!==documentId){fail(new Error('Session belongs to another document'));return;}const cursor=tx.objectStore('events').openCursor(IDBKeyRange.bound([sessionId,0],[sessionId,Number.MAX_SAFE_INTEGER]),'prev');cursor.onsuccess=()=>{const row=cursor.result;if(!row){set(undefined);return;}const event=row.value as AgentStoredEvent;if(event.payload.kind==='preview'&&event.payload.reference===reference){set(event.payload);return;}row.continue();};};});
  }
  lastUsage(documentId:string,sessionId:string):Promise<ModelResult['usage']|undefined>{
    return this.transaction(['sessions','events'],'readonly',(tx,set,fail)=>{const request=tx.objectStore('sessions').get(sessionId);request.onsuccess=()=>{const session=request.result as AgentSessionRecord|undefined;if(!session||session.documentId!==documentId){fail(new Error('Session belongs to another document'));return;}const cursor=tx.objectStore('events').index('display').openCursor(IDBKeyRange.bound([sessionId,'model_result',0],[sessionId,'model_result',Number.MAX_SAFE_INTEGER]),'prev');cursor.onsuccess=()=>{const value=cursor.result?.value as AgentStoredEvent|undefined;set(value?.payload.kind==='model_result'?value.payload.result.usage:undefined);};};});
  }
  saveAttachment(attachment:StoredAttachment):Promise<void>{return this.transaction(['attachments'],'readwrite',(tx,set,fail)=>{const store=tx.objectStore('attachments'),request=store.get(attachment.id);request.onsuccess=()=>{const old=request.result as StoredAttachment|undefined;if(old&&old.documentId!==attachment.documentId){fail(new Error('Attachment belongs to another document'));return;}store.put(attachment);set();};});}
  attachment(documentId:string,id:string):Promise<StoredAttachment|undefined>{return this.transaction(['attachments'],'readonly',(tx,set)=>{const request=tx.objectStore('attachments').get(id);request.onsuccess=()=>{const value=request.result as StoredAttachment|undefined;set(value?.documentId===documentId?value:undefined);};});}
  beginOperation(record:AgentOperationRecord):Promise<{created:boolean;record:AgentOperationRecord}>{
    const frozen=structuredClone(record);
    return this.transaction(['operations'],'readwrite',(tx,set,fail)=>{
      const store=tx.objectStore('operations'),request=store.get(frozen.id);
      request.onsuccess=()=>{
        const old=request.result as AgentOperationRecord|undefined;
        if(old&&(old.documentId!==frozen.documentId||old.sessionId!==frozen.sessionId||old.taskId!==frozen.taskId||old.tool!==frozen.tool||old.inputHash!==frozen.inputHash)){fail(new Error('Operation identity conflict'));return;}
        if(!old)store.add(frozen);set({created:!old,record:old??frozen});
      };
    });
  }
  settleOperation(record:AgentOperationRecord,transaction?:AgentTransactionRecord,document?:AgentDocumentRecord):Promise<AgentOperationRecord>{
    const frozen=structuredClone({record,transaction,document});
    const accepted=frozen.record;
    return this.transaction(['operations','transactions','documents'],'readwrite',(tx,set,fail)=>{
      const store=tx.objectStore('operations'),request=store.get(accepted.id);
      request.onsuccess=()=>{
        const old=request.result as AgentOperationRecord|undefined;
        if(!old||old.documentId!==accepted.documentId||old.sessionId!==accepted.sessionId||old.taskId!==accepted.taskId||old.tool!==accepted.tool||old.inputHash!==accepted.inputHash){fail(new Error('Operation identity conflict'));return;}
        if(terminal(old.state)){set(old);return;}
        const persist=()=>{
          if(frozen.transaction){if(frozen.transaction.documentId!==accepted.documentId||frozen.transaction.sessionId!==accepted.sessionId||frozen.transaction.taskId!==accepted.taskId||frozen.transaction.id!==accepted.transactionRef){fail(new Error('Transaction belongs to another operation'));return;}tx.objectStore('transactions').add(frozen.transaction);}
          if(frozen.document)tx.objectStore('documents').put(frozen.document);
          store.put(accepted);set(accepted);
        };
        const save=()=>{
          const inverse=frozen.transaction??accepted.intent?.transaction,refs=inverse?.undoes;
          if(!inverse||!refs?.length){persist();return;}
          let remaining=refs.length;
          const done=()=>{if(--remaining===0)persist();};
          for(const id of refs){const request=tx.objectStore('transactions').get(id);request.onsuccess=()=>{
            const original=request.result as AgentTransactionRecord|undefined;
            if(!original||original.documentId!==accepted.documentId){fail(new Error('UNDO_CONFLICT'));return;}
            if(original.undoneBy&&original.undoneBy!==inverse.id){fail(new Error('UNDO_CONFLICT'));return;}
            tx.objectStore('transactions').put({...original,undoneBy:inverse.id});
            if(!original.undoes?.length){done();return;}
            let active=original.undoes.length;
            for(const previous of original.undoes){const request=tx.objectStore('transactions').get(previous);request.onsuccess=()=>{const record=request.result as AgentTransactionRecord|undefined;if(!record||record.documentId!==accepted.documentId||record.undoneBy!==original.id){fail(new Error('UNDO_CONFLICT'));return;}tx.objectStore('transactions').put({...record,undoneBy:null});if(--active===0)done();};}
          };}
        };
        if(frozen.document){
          if(frozen.document.id!==accepted.documentId){fail(new Error('Source belongs to another document'));return;}
          const current=tx.objectStore('documents').get(frozen.document.id);
          current.onsuccess=()=>{if(olderDocument(current.result as AgentDocumentRecord|undefined,frozen.document!)){fail(new Error('STALE_SNAPSHOT'));return;}save();};
        }else save();
      };
    });
  }
  operation(documentId:string,id:string):Promise<AgentOperationRecord|undefined>{
    return this.transaction(['operations'],'readonly',(tx,set)=>{const request=tx.objectStore('operations').get(id);request.onsuccess=()=>{const record=request.result as AgentOperationRecord|undefined;set(record?.documentId===documentId?record:undefined);};});
  }
  pendingOperations(documentId:string):Promise<AgentOperationRecord[]>{
    return this.transaction(['operations'],'readonly',(tx,set)=>{
      const store=tx.objectStore('operations'),records:AgentOperationRecord[]=[];
      if(store.indexNames.contains('pending')){
        let remaining=3;for(const state of ['prepared','running','unknown']){const request=store.index('pending').getAll([documentId,state]);request.onsuccess=()=>{records.push(...request.result as AgentOperationRecord[]);if(--remaining===0)set(records.sort((a,b)=>a.updatedAt-b.updatedAt));};}
      }else{
        const request=store.index('document').openCursor(documentId);request.onsuccess=()=>{const cursor=request.result;if(!cursor){set(records);return;}const record=cursor.value as AgentOperationRecord;if(!terminal(record.state))records.push(record);cursor.continue();};
      }
    });
  }
  sourceTransaction(documentId:string,id:string):Promise<AgentTransactionRecord|undefined>{
    return this.transaction(['transactions','operations'],'readonly',(tx,set)=>{const request=tx.objectStore('transactions').get(id);request.onsuccess=()=>{
      const record=request.result as AgentTransactionRecord|undefined;
      if(record){set(record.documentId===documentId?record:undefined);return;}
      if(tx.objectStore('operations').indexNames.contains('transaction')){
        const operation=tx.objectStore('operations').index('transaction').get(id);operation.onsuccess=()=>{const record=operation.result as AgentOperationRecord|undefined;set(record?.documentId===documentId&&record.effectCommitted===true&&record.intent?.transaction.id===id?record.intent.transaction:undefined);};return;
      }
      const cursor=tx.objectStore('operations').index('document').openCursor(documentId);
      cursor.onsuccess=()=>{const row=cursor.result;if(!row){set(undefined);return;}const operation=row.value as AgentOperationRecord;if(operation.effectCommitted===true&&operation.intent?.transaction.id===id){set(operation.intent.transaction);return;}row.continue();};
    };});
  }
  close(){this.closed=true;void this.database?.then(db=>db.close());}
}
