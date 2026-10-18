import type {NotebookFile} from '../kernel/generated/NotebookFile';
import type {CanonicalRecord,ModelInput,ModelResult} from './modelTypes';
import {sameNotebookSource} from '../state/sourceTransaction';
export interface AgentDocumentRecord {id:string;generation:number;revision:number;file:NotebookFile;updatedAt:number}
export interface AgentSessionRecord {id:string;documentId:string;title:string;createdAt:number;nextSequence:number}
export type AgentEventPayload =
  | {kind:'message';record:CanonicalRecord;modelId?:string;mode?:'execute'|'discuss'}
  | {kind:'tool';callId:string;name:string;operationRef:string|null;argumentsJson:string;resultJson:string|null}
  | {kind:'turn';taskId:string;mode:'execute'|'discuss';phase:'started'|'ended'|'stopped'|'failed'|'budget';time:number}
  | {kind:'input';taskId:string;turnId:string;input:Omit<ModelInput,'images'>}
  | {kind:'dispatch';taskId:string;turnId:string;requestId:string;time:number}
  | {kind:'model_result';result:ModelResult};
export interface AgentStoredEvent {id:string;documentId:string;sessionId:string;sequence:number;time:number;payload:AgentEventPayload}
export interface AgentOperationRecord {
  id:string;documentId:string;sessionId:string;taskId:string;tool:string;inputHash:string;
  state:'prepared'|'running'|'completed'|'failed'|'cancelled'|'unknown';
  effectCommitted:boolean|null;transactionRef:string|null;resultJson:string|null;resultRefs:string[];updatedAt:number;
  intent?:{transaction:AgentTransactionRecord;planHash:string};
}
export interface AgentTransactionRecord {id:string;documentId:string;sessionId:string;taskId:string;before:NotebookFile;after:NotebookFile;changedCellIds:string[];createdAt:number;undoneBy:string|null}
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
      const request=indexedDB.open(this.name,1);
      request.onupgradeneeded=()=>{
        const db=request.result;
        db.createObjectStore('documents',{keyPath:'id'});
        for(const name of ['sessions','operations','transactions']){const store=db.createObjectStore(name,{keyPath:'id'});store.createIndex('document','documentId');if(name==='operations'){store.createIndex('pending',['documentId','state']);store.createIndex('transaction','transactionRef');}}
        db.createObjectStore('events',{keyPath:['sessionId','sequence']}).createIndex('document','documentId');
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
        const save=()=>{
          if(frozen.transaction){if(frozen.transaction.documentId!==accepted.documentId||frozen.transaction.sessionId!==accepted.sessionId||frozen.transaction.taskId!==accepted.taskId||frozen.transaction.id!==accepted.transactionRef){fail(new Error('Transaction belongs to another operation'));return;}tx.objectStore('transactions').add(frozen.transaction);}
          if(frozen.document)tx.objectStore('documents').put(frozen.document);
          store.put(accepted);set(accepted);
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
