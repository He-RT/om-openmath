import {memo,useEffect,useRef,useState} from 'react';
import {marked} from 'marked';
import type {ConversationTurn} from '../../agent/conversation';
import type {UiCell} from '../../state/notebookStore';
import type {Messages} from '../../i18n';
import type {AgentEventPayload} from '../../agent/store';
import {Markdown} from '../TextCell';
import {ActionLog} from './ActionLog';
import {TurnOutcome} from './TurnOutcome';
import type {Dialect} from '../../kernel/generated/Dialect';
import {agentText as text} from './agentText';
import type {AttachmentCard,StoredAttachment} from '../../agent/attachments';
function SentFile({file,load,t}:{file:AttachmentCard;load:(id:string)=>Promise<StoredAttachment|undefined>;t:Messages}){
  const [media,setMedia]=useState<{original:string;image?:string}|null>(null);
  useEffect(()=>{let current=true;const urls:string[]=[];void load(file.id).then(record=>{if(!current||!record)return;const original=URL.createObjectURL(record.original);urls.push(original);const image=record.image?URL.createObjectURL(record.image.blob):undefined;if(image)urls.push(image);setMedia({original,...(image?{image}:{})});});return()=>{current=false;for(const url of urls)URL.revokeObjectURL(url);};},[file.id,load]);
  return <div className="agent-file">{media?.image&&<img src={media.image} alt=""/>}<div>{media?<a href={media.original} download={file.name}>{file.name}</a>:<strong>{file.name}</strong>}<small>{Math.ceil(file.size/1024)} KB · {file.type||'File'}</small>{!file.sentImage&&<small>{text(t,'未发送给模型，原件已保留','Not sent to the model; original retained')}</small>}</div></div>;
}
const Block=memo(function Block({source,t,onInsert}:{source:string;t:Messages;onInsert:(source:string,dialect:Dialect)=>void}){return <Markdown source={source} t={t} onInsertCode={onInsert}/>;});
function MessageMarkdown({source,t,onInsert}:{source:string;t:Messages;onInsert:(source:string,dialect:Dialect)=>void}){return <>{marked.lexer(source).map((token,index)=><Block key={index} source={token.raw} t={t} onInsert={onInsert}/>)}</>;}
export function Transcript({turns,cells,dirty,t,onUndo,onLocate,onInsert,loadPreview,loadAttachment,running}:{turns:ConversationTurn[];cells:UiCell[];dirty:boolean;t:Messages;onUndo:(refs:string[])=>Promise<unknown>;onLocate:(id:string)=>void;onInsert:(source:string,dialect:Dialect)=>void;loadPreview:(id:string)=>Promise<Extract<AgentEventPayload,{kind:'preview'}>|undefined>;loadAttachment:(id:string)=>Promise<StoredAttachment|undefined>;running:boolean}){
  const root=useRef<HTMLDivElement>(null),following=useRef(true),[newMessages,setNewMessages]=useState(false);
  useEffect(()=>{const element=root.current;if(!element)return;if(following.current){element.scrollTop=element.scrollHeight;setNewMessages(false);}else setNewMessages(true);},[turns]);
  return <div className="agent-transcript" ref={root} onScroll={()=>{const element=root.current;if(element){following.current=element.scrollHeight-element.scrollTop-element.clientHeight<36;if(following.current)setNewMessages(false);}}}>
    {turns.map(turn=><article className="agent-turn" key={turn.id} data-task-id={turn.id}>
      <div className="agent-user"><div className="user-bubble">{turn.presentation&&<><div className="agent-attachments">{turn.presentation.attachments.map(file=><SentFile key={file.id} file={file} load={loadAttachment} t={t}/>)}</div>{turn.presentation.references.map(ref=><button className="sent-reference" key={ref.id} onClick={()=>onLocate(ref.id)}>[{ref.label}]</button>)}</>}{turn.presentation?.text??turn.user.parts.filter(part=>part.kind==='text').map(part=>part.text).join('\n')}</div></div>
      <div className="agent-reply"><div className="agent-signature"><strong>{t.assistant}</strong><span>{turn.modelId} · {turn.mode==='execute'?text(t,'执行','Execute'):text(t,'讨论','Discuss')}</span></div>
        {turn.parts.map(part=>part.kind==='text'?<div className={part.partial?'agent-text partial':'agent-text'} key={part.id}><MessageMarkdown source={part.text} t={t} onInsert={onInsert}/></div>:<ActionLog key={part.id} actions={part.actions} running={turn.running} cells={cells} t={t} onUndo={onUndo} onLocate={onLocate} loadPreview={loadPreview}/>)}
        {turn.running&&<p className="agent-live" role="status"><span className="agent-spinner" aria-hidden="true"/>{text(t,'正在处理笔记本…','Working with the notebook…')}</p>}
        <TurnOutcome turn={turn} cells={cells} dirty={dirty} t={t} onUndo={onUndo} onLocate={onLocate}/>
      </div>
    </article>)}
    {running&&turns.length===0&&<p className="agent-live" role="status">{text(t,'正在准备本轮输入…','Preparing this turn…')}</p>}
    {newMessages&&<button className="agent-new-messages" onClick={()=>{const element=root.current;if(element)element.scrollTop=element.scrollHeight;following.current=true;setNewMessages(false);}}>{text(t,'新消息 ⌄','New messages ⌄')}</button>}
  </div>;
}
