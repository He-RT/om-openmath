import {buildInfo} from '../../buildInfo';
import {useCallback,useEffect,useRef,useState} from 'react';
import type {NotebookController,UiState} from '../../state/notebookStore';
import type {Dialect} from '../../kernel/generated/Dialect';
import type {Messages} from '../../i18n';
import type {ProfileConfig} from '../../kernel/generated/ProfileConfig';
import {configuredModels,modelMetadata} from '../../agent/profileModel';
import type {AgentEventPayload} from '../../agent/store';
import type {ModelResult} from '../../agent/modelTypes';
import {agentSystemPrompt} from '../../agent/prompts';
import {hasConsent,PrivacyPrompt,destination} from './PrivacyPrompt';
import {useAgentConversation} from './useAgentConversation';
import {Composer,type ComposerInput} from './Composer';
import {Transcript} from './Transcript';
import {WorkbenchIcon} from '../WorkbenchIcon';
import {useResponsiveDialog} from '../useResponsiveDialog';
import {agentText as text} from './agentText';
import {assistantPreferences,settingsChanged} from '../../state/modelRegistry';
function AssistantDialog({title,onClose,children,closeLabel}:{title:string;onClose:()=>void;children:React.ReactNode;closeLabel:string}){
  const root=useRef<HTMLDialogElement>(null);useResponsiveDialog(root,true,true);
  return <dialog className="agent-dialog" ref={root} onCancel={event=>{event.preventDefault();onClose();}} aria-label={title}><header><h2>{title}</h2><button aria-label={closeLabel} onClick={onClose}><WorkbenchIcon name="close"/></button></header>{children}</dialog>;
}
export function AssistantPanel({state,controller,t,onSettings}:{state:UiState;controller:NotebookController;t:Messages;onSettings:()=>void}){
  const agent=useAgentConversation(controller,state.documentGeneration),models=state.config?.llm.enabled?configuredModels(state.config.llm.profiles):[];
  const [preferences,setPreferences]=useState(assistantPreferences),[model,setModel]=useState(()=>assistantPreferences().defaultModel),[mode,setMode]=useState<'execute'|'discuss'>(()=>assistantPreferences().defaultMode),[historyOpen,setHistory]=useState(false),[contextOpen,setContext]=useState(false),[fullInput,setFullInput]=useState(false),[contextDraft,setContextDraft]=useState<ComposerInput>({text:'',files:[],references:[]});
  const [input,setInput]=useState<Extract<AgentEventPayload,{kind:'input'}>|undefined>(),[usage,setUsage]=useState<ModelResult['usage']|undefined>();
  const [privacy,setPrivacy]=useState<{profile:ProfileConfig;resolve:(value:boolean)=>void}|null>(null),pending=useRef<((value:boolean)=>void)|null>(null);
  const selected=models.find(item=>item.name===model)??models.find(item=>item.name===state.config?.llm.chat)??models[0],selectedName=selected?.name??'',running=agent.phase!=='idle';
  useEffect(()=>()=>{pending.current?.(false);pending.current=null;},[state.documentGeneration]);
  useEffect(()=>{const update=()=>setPreferences(assistantPreferences());window.addEventListener(settingsChanged,update);window.addEventListener('storage',update);return()=>{window.removeEventListener(settingsChanged,update);window.removeEventListener('storage',update);};},[]);
  const newConversation=()=>{const next=assistantPreferences();setPreferences(next);setMode(next.defaultMode);setModel(next.defaultModel);agent.newSession();};
  const insert=useCallback((source:string,dialect:Dialect)=>{const id=controller.add('Math',source,controller.store.getState().active??undefined);controller.edit(id,{dialect});controller.select(id,true);},[controller]);
  const locate=useCallback((id:string)=>{controller.select(id);const element=document.querySelector<HTMLElement>(`.notebook-cell[data-cell-id="${CSS.escape(id)}"]`);element?.scrollIntoView({block:'center',behavior:matchMedia('(prefers-reduced-motion: reduce)').matches?'instant':'smooth'});if(!matchMedia('(prefers-reduced-motion: reduce)').matches){element?.classList.remove('agent-located');requestAnimationFrame(()=>element?.classList.add('agent-located'));}},[controller]);
  const send=async(candidate:ComposerInput)=>{
    if(!selected)return false;
    const profile=selected,generation=state.documentGeneration,scope='agent:notebook:'+mode;
    if(!hasConsent(controller.kernel,profile,scope)){
      const consent=await new Promise<boolean>(resolve=>{pending.current=resolve;setPrivacy({profile,resolve});});pending.current=null;setPrivacy(null);if(!consent)return false;
    }
    const current=controller.store.getState();
    if(current.documentGeneration!==generation||destination(current.config?.llm.profiles.find(item=>item.name===profile.name)??profile)!==destination(profile)){agent.setError(text(t,'模型或文稿已改变，输入已保留','Model or document changed; input retained'));return false;}
    return agent.send(candidate,mode,profile.name,t);
  };
  const openContext=(draft:ComposerInput)=>{setContextDraft(draft);setContext(true);setInput(undefined);setUsage(undefined);if(agent.turns.length)void Promise.all([agent.lastInput(),agent.lastUsage()]).then(([input,usage])=>{setInput(input);setUsage(usage);}).catch(error=>agent.setError(String(error)));};
  const metadata=selected?modelMetadata(selected.name):{},systemEstimate=Math.ceil((agentSystemPrompt(buildInfo.version,mode).length+(input?JSON.stringify(input.input.tools).length:0))/1.6),historyEstimate=Math.ceil((JSON.stringify(agent.turns.map(turn=>({user:turn.user,parts:turn.parts}))).length+contextDraft.text.length+contextDraft.references.reduce((n,id)=>n+(state.cells.find(cell=>cell.id===id)?.source.length??0),0))/1.6);
  return <section className="assistant-panel" aria-label={t.assistant}>
    <header className="assistant-header"><h2>{agent.turns[0]?.presentation?.text.slice(0,20)||agent.turns[0]?.user.parts.filter(part=>part.kind==='text').map(part=>part.text).join('').slice(0,20)||text(t,'新对话','New conversation')}</h2><div>
      <button aria-label={text(t,'历史对话','Conversation history')} disabled={running||agent.controlling} onClick={()=>{setHistory(true);void agent.history();}}><WorkbenchIcon name="history"/></button>
      <button aria-label={t.newConversation} disabled={running||agent.controlling} onClick={newConversation}><WorkbenchIcon name="new"/></button>
    </div></header>
    <div className="agent-conversation-body">
      {agent.turns.length===0&&agent.phase==='idle'&&<div className="agent-empty"><p>{text(t,'可以一起求解、绘图，或修改当前笔记本。','Solve, plot, or edit the current notebook together.')}</p>{!selected&&<><p>{t.configureAI}</p><button onClick={onSettings}>{text(t,'添加模型…','Add a model…')}</button></>}</div>}
      <Transcript turns={agent.turns} cells={state.cells} dirty={state.dirty||!state.savedSource} t={t} onUndo={agent.undo} onLocate={locate} onInsert={insert} loadPreview={agent.preview} loadAttachment={agent.attachment} running={running} work={state.agentWork} stopping={agent.phase==='stopping'} onStop={()=>void agent.stop()}/>
    </div>
    {agent.error&&<div className="agent-request-error" role="alert">{agent.error}<button aria-label={t.close} onClick={()=>agent.setError(null)}>×</button></div>}
    <Composer models={models} model={selectedName} onModel={setModel} mode={mode} onMode={setMode} cells={state.cells} t={t} running={running} stopping={agent.phase==='stopping'} disabled={state.busy||agent.controlling} onSend={send} onStop={()=>void agent.stop()} onSettings={onSettings} onContext={openContext} shortcut={preferences.sendShortcut} incomingReferences={state.assistantReferences} onConsumeReferences={()=>controller.store.setState({assistantReferences:undefined})}/>
    {privacy&&<PrivacyPrompt kernel={controller.kernel} profile={privacy.profile} scope={'agent:notebook:'+mode} t={t} disclosure={text(t,'将向下列地址发送当前对话、引用与可用附件，以及助手读取的笔记本内容和工具结果。执行模式可修改、运行笔记本，修改可撤销。','The conversation, references, supported attachments, notebook content read by the assistant, and tool results will be sent to this endpoint. Execute mode can edit and run the notebook; changes are undoable.')} onClose={()=>privacy.resolve(false)} onSend={()=>privacy.resolve(true)}/>}
    {historyOpen&&<AssistantDialog closeLabel={t.close} title={text(t,'历史对话','Conversation history')} onClose={()=>setHistory(false)}><div className="agent-history">{agent.sessions.map(session=><button aria-current={session.id===agent.sessionId} key={session.id} onClick={()=>{setHistory(false);void agent.selectSession(session.id);}}><strong>{session.title||text(t,'新对话','New conversation')}</strong><time>{new Date(session.createdAt).toLocaleString()}</time></button>)}{agent.archives.length>0&&<><h3>{text(t,'保存在此浏览器的笔记本','Notebooks saved in this browser')}</h3><p>{text(t,'恢复助手最后记录的源码与对话；未记录的手工编辑不在此快照内。','Restore the last source and conversation recorded by the assistant. Later manual edits are not in this snapshot.')}</p>{agent.archives.map(({document,session})=><button key={document.id} onClick={()=>{if(state.cells.length&&!confirm(text(t,'恢复会替换当前笔记本，请先保存需要的内容。继续恢复？','Restore replaces the current notebook. Save any content you need first. Restore?')))return;setHistory(false);void agent.restore(document,session);}}><strong>{document.file.title||session.title}</strong><time>{new Date(document.updatedAt).toLocaleString()}</time><span>{text(t,'恢复笔记本与对话','Restore notebook and conversation')}</span></button>)}</>}{!agent.sessions.length&&<p>{text(t,'此笔记本还没有历史对话','No conversations for this notebook yet')}</p>}</div></AssistantDialog>}
    {contextOpen&&<AssistantDialog closeLabel={t.close} title={text(t,'上下文用量','Context usage')} onClose={()=>{setContext(false);setFullInput(false);}}><dl className="agent-context-details"><dt>{text(t,'系统与工具说明','System and tools')}</dt><dd>{text(t,`系统与已知工具预估 ${systemEstimate} tokens${input?'':'；工具目录将在首轮准备时加入'}`,`System and known tools estimated ${systemEstimate} tokens${input?'':'; tool catalogue added when the first turn is prepared'}`)}</dd><dt>{text(t,'对话与工具结果','Conversation and tool results')}</dt><dd>{input?text(t,`对话与当前草稿预估 ${historyEstimate} tokens（不含图片）`,`Conversation and current draft estimated ${historyEstimate} tokens (excluding images)`):text(t,'尚无请求记录','No request recorded')}</dd><dt>{text(t,'本条附件','Attachments')}</dt><dd>{text(t,`${contextDraft.files.length} 个文件；图片 tokens 由供应商计量，发送前未知`,`${contextDraft.files.length} files; provider image tokens are unknown before sending`)}</dd><dt>{text(t,'上限及来源','Limit and source')}</dt><dd>{metadata.contextWindow?text(t,`${metadata.contextWindow} tokens · 模型设置`,`${metadata.contextWindow} tokens · Model settings`):text(t,'上限未知','Limit unknown')}</dd><dt>{text(t,'上一轮请求的实际用量','Actual usage of the last request')}</dt><dd>{usage?text(t,`输入 ${usage.input??'未知'} / 输出 ${usage.output??'未知'} tokens`,`Input ${usage.input??'unknown'} / output ${usage.output??'unknown'} tokens`):text(t,'未知','Unknown')}</dd></dl><button disabled={!input} onClick={()=>setFullInput(value=>!value)}>{text(t,'查看本轮完整输入','View full turn input')}</button>{fullInput&&input&&<pre className="agent-full-input" tabIndex={0}>{JSON.stringify(input.input,null,2)}</pre>}</AssistantDialog>}
  </section>;
}
