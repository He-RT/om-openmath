import {useEffect,useLayoutEffect,useRef,useState} from 'react';
import type {ProfileConfig} from '../../kernel/generated/ProfileConfig';
import type {UiCell} from '../../state/notebookStore';
import type {Messages} from '../../i18n';
import {modelMetadata} from '../../agent/profileModel';
import {supportedImage} from '../../agent/attachments';
import {ToolbarMenu} from '../ToolbarMenu';
import {WorkbenchIcon} from '../WorkbenchIcon';
import {agentText as text,cellLabel} from './agentText';

export interface ComposerFile {id:string;file:File;url?:string}
export interface ComposerInput {text:string;files:ComposerFile[];references:string[]}
export function shouldSend(event:{key:string;shiftKey:boolean;metaKey:boolean;ctrlKey:boolean;isComposing:boolean;keyCode:number},composing:boolean,shortcut:string){
  return event.key==='Enter'&&!event.shiftKey&&!composing&&!event.isComposing&&event.keyCode!==229&&(shortcut==='command'?(event.metaKey||event.ctrlKey):!event.metaKey&&!event.ctrlKey);
}
export function Composer({models,model,onModel,mode,onMode,cells,t,running,stopping,disabled,onSend,onStop,onSettings,onContext,shortcut='enter',incomingReferences,onConsumeReferences}:{
  models:ProfileConfig[];model:string;onModel:(id:string)=>void;mode:'execute'|'discuss';onMode:(mode:'execute'|'discuss')=>void;
  cells:UiCell[];t:Messages;running:boolean;stopping:boolean;disabled:boolean;onSend:(input:ComposerInput)=>Promise<boolean>;
  onStop:()=>void;onSettings:()=>void;onContext:(input:ComposerInput)=>void;shortcut?:string;
  incomingReferences?:string[]|undefined;onConsumeReferences?:()=>void;
}){
  const [draft,setDraft]=useState(''),[files,setFiles]=useState<ComposerFile[]>([]),[references,setReferences]=useState<string[]>([]),[search,setSearch]=useState(''),[pending,setPending]=useState(false);
  const editor=useRef<HTMLTextAreaElement>(null),root=useRef<HTMLDivElement>(null),fileInput=useRef<HTMLInputElement>(null),composing=useRef(false),version=useRef(0),ownedUrls=useRef(new Set<string>());
  const selected=models.find(item=>item.name===model),meta=selected?modelMetadata(selected.name):{},hasTools=selected?.supports_tools??false;
  useEffect(()=>{if(!incomingReferences?.length)return;version.current++;setReferences(previous=>[...new Set([...previous,...incomingReferences])]);onConsumeReferences?.();editor.current?.focus();},[incomingReferences,onConsumeReferences]);
  useLayoutEffect(()=>{
    const resize=()=>{const element=editor.current;if(!element)return;const panel=root.current?.closest('.assistant-panel');element.style.height='0px';element.style.height=`${Math.max(44,Math.min(element.scrollHeight,(panel?.clientHeight??600)*.35))}px`;};
    resize();const observer=new ResizeObserver(resize);if(root.current){observer.observe(root.current);const panel=root.current.closest('.assistant-panel');if(panel)observer.observe(panel);}return()=>observer.disconnect();
  },[draft]);
  useLayoutEffect(()=>()=>{for(const url of ownedUrls.current)URL.revokeObjectURL(url);},[]);
  const addFiles=(incoming:File[])=>{version.current++;setFiles(previous=>[...previous,...incoming.map(file=>{const url=file.type.startsWith('image/')?URL.createObjectURL(file):undefined;if(url)ownedUrls.current.add(url);return {id:'media-'+crypto.randomUUID(),file,...(url?{url}:{})};})]);};
  const send=async()=>{
    if(running||pending||disabled||!selected||mode==='execute'&&!hasTools||!draft.trim()&&files.length===0&&references.length===0||composing.current)return;
    const mark=version.current;setPending(true);
    try{if(await onSend({text:draft,files:[...files],references:[...references]})&&mark===version.current){setDraft('');setFiles([]);setReferences([]);version.current++;}}finally{setPending(false);}
  };
  const groups=new Map<string,ProfileConfig[]>();for(const item of models){const metadata=modelMetadata(item.name),provider=metadata.providerName??new URL(item.base_url).host;if(`${provider} ${item.model} ${metadata.displayName??''}`.toLowerCase().includes(search.toLowerCase()))groups.set(provider,[...(groups.get(provider)??[]),item]);}
  return <div className="agent-composer-area" ref={root}>
    <div className="agent-composer" onDragOver={event=>{if(event.dataTransfer.types.includes('Files'))event.preventDefault();}} onDrop={event=>{if(event.dataTransfer.files.length){event.preventDefault();addFiles([...event.dataTransfer.files]);}}}>
      {(files.length>0||references.length>0)&&<div className="agent-attachments" aria-label={text(t,'附件和引用','Attachments and references')}>
        {files.map(item=><div className="agent-file" key={item.id}>{item.url?<img src={item.url} alt=""/>:<span className="file-symbol" aria-hidden="true">▤</span>}<div><strong>{item.file.name}</strong><small>{Math.ceil(item.file.size/1024)} KB · {item.file.type||text(t,'文件','File')}</small>{!(supportedImage(item.file.type)&&meta.supportsImages)&&<small>{text(t,'暂不能发送给模型','Cannot send to this model yet')}</small>}</div><button aria-label={text(t,`移除 ${item.file.name}`,`Remove ${item.file.name}`)} onClick={()=>{version.current++;setFiles(files.filter(file=>file.id!==item.id));}}>×</button></div>)}
        {references.map(id=><span className="agent-reference" key={id}>{cellLabel(t,cells,id)}<button aria-label={text(t,'移除引用','Remove reference')} onClick={()=>{version.current++;setReferences(references.filter(ref=>ref!==id));}}>×</button></span>)}
      </div>}
      <textarea ref={editor} aria-label={t.chatInput} placeholder={t.chatPlaceholder} value={draft} rows={2}
        onChange={event=>{version.current++;setDraft(event.target.value);}} onCompositionStart={()=>{composing.current=true;}} onCompositionEnd={()=>{composing.current=false;}}
        onPaste={event=>{const incoming=[...event.clipboardData.files];if(incoming.length){event.preventDefault();addFiles(incoming);}}}
        onKeyDown={event=>{if(shouldSend({...event,isComposing:event.nativeEvent.isComposing,keyCode:event.nativeEvent.keyCode},composing.current,shortcut)){event.preventDefault();void send();}}}/>
      <div className="agent-composer-controls">
        <ToolbarMenu label={text(t,'添加附件或引用','Add attachment or reference')} icon={<WorkbenchIcon name="plus"/>}>
          <button onClick={()=>{if(fileInput.current){fileInput.current.accept='';fileInput.current.click();}}}>{text(t,'添加文件…','Add files…')}</button>
          <button onClick={()=>{if(fileInput.current){fileInput.current.accept='image/*';fileInput.current.click();}}}>{text(t,'添加图片…','Add images…')}</button>
          <div className="menu-group-label">{text(t,'引用单元格','Reference a cell')}</div>
          {cells.map(cell=><button key={cell.id} disabled={references.includes(cell.id)} onClick={()=>{version.current++;setReferences([...references,cell.id]);}}>{cellLabel(t,cells,cell.id)} · {cell.source.slice(0,28)}</button>)}
        </ToolbarMenu>
        <ToolbarMenu label={text(t,'助手模式','Assistant mode')} className="agent-mode-menu" menuRole="menu" icon={<>{mode==='execute'?text(t,'执行','Execute'):text(t,'讨论','Discuss')}<WorkbenchIcon name="chevron"/></>}>
          {(['execute','discuss'] as const).map(value=><button role="menuitemradio" aria-checked={mode===value} key={value} onClick={()=>onMode(value)}><strong>{value==='execute'?text(t,'执行','Execute'):text(t,'讨论','Discuss')}</strong><small>{value==='execute'?text(t,'可以修改和运行笔记本，每轮修改可撤销','Edit and run the notebook; changes are undoable'):text(t,'只读取，计算在临时环境中进行','Read only; calculations use a scratch environment')}</small></button>)}
        </ToolbarMenu>
        <span className="composer-spacer"/>
        <button className="agent-context" aria-label={text(t,'上下文用量','Context usage')} title={meta.contextWindow?text(t,'上下文预估','Context estimate'):text(t,'上限未知','Limit unknown')} onClick={()=>onContext({text:draft,files,references})}><span className="context-ring" data-known={!!meta.contextWindow}/></button>
        <ToolbarMenu label={text(t,'选择模型','Select model')} className="agent-model-menu" menuRole="menu" icon={<><span>{meta.displayName??selected?.model??text(t,'选择模型','Select model')}</span><WorkbenchIcon name="chevron"/></>}>
          <input aria-label={text(t,'搜索模型','Search models')} placeholder={text(t,'搜索模型','Search models')} value={search} onChange={event=>setSearch(event.target.value)} onKeyDown={event=>{if(event.key==='Escape'){event.preventDefault();const menu=event.currentTarget.closest('details');if(menu){menu.open=false;menu.querySelector('summary')?.focus();}}if(event.key==='ArrowDown'){event.preventDefault();event.currentTarget.closest('details')?.querySelector<HTMLButtonElement>('button:not(:disabled)')?.focus();}}}/>
          {[...groups].map(([provider,items])=><div key={provider}><div className="menu-group-label">{provider}</div>{items.map(item=>{const metadata=modelMetadata(item.name),unavailable=mode==='execute'&&!item.supports_tools;return <button key={item.name} role="menuitemradio" aria-checked={item.name===model} disabled={unavailable} onClick={()=>onModel(item.name)}><strong>{metadata.displayName??item.model}</strong><small>{unavailable?text(t,'不支持工具调用，讨论模式下可用','No tool calling; available in Discuss'):item.supports_tools?text(t,'工具调用','Tool calling'):text(t,'文字对话','Text conversation')}{metadata.supportsImages&&text(t,' · 图片输入',' · Images')}</small></button>;})}</div>)}
          {groups.size===0&&<p className="muted">{text(t,'没有匹配的已配置模型','No configured models match')}</p>}
          <button role="menuitem" onClick={onSettings}>{text(t,'管理模型…','Manage models…')} <kbd>⌘,</kbd></button>
        </ToolbarMenu>
        {running?<button className="agent-send agent-stop" aria-label={stopping?text(t,'正在停止…','Stopping…'):t.stop} disabled={stopping} onClick={onStop}><WorkbenchIcon name="stop"/></button>:<button className="agent-send" aria-label={t.send} disabled={pending||disabled||!selected||mode==='execute'&&!hasTools||!draft.trim()&&!files.length&&!references.length} onClick={()=>void send()}><WorkbenchIcon name="send"/></button>}
      </div>
      <input hidden multiple type="file" ref={fileInput} onChange={event=>{addFiles([...(event.target.files??[])]);event.target.value='';}}/>
    </div>
    {(running||mode==='discuss')&&<p className="composer-note">{running?text(t,'模式和模型从下一轮开始使用','Mode and model apply to the next turn'):text(t,'讨论模式不会修改或运行笔记本','Discuss mode does not edit or run the notebook')}</p>}
  </div>;
}
