import {useRef,useState} from 'react';
import type {KernelConfig} from '../../kernel/generated/KernelConfig';
import type {NotebookController} from '../../state/notebookStore';
import type {Messages} from '../../i18n';
import {GeneralSettings} from './GeneralSettings';
import {ModelSettings} from './ModelSettings';
import {useResponsiveDialog} from '../useResponsiveDialog';
import {WorkbenchIcon} from '../WorkbenchIcon';
const labels={general:['通用','General'],editing:['编辑与计算','Editing & calculation'],models:['模型','Models'],assistant:['助手','Assistant'],data:['数据','Data']} as const;
export type SettingsGroup=keyof typeof labels;
export function SettingsDialog({config,controller,t,theme,onTheme,onClose,initialTab='general'}:{config:KernelConfig;controller:NotebookController;t:Messages;theme:string;onTheme:(theme:string)=>void;onClose:()=>void;initialTab?:'general'|'models'}){
  const [tab,setTab]=useState<SettingsGroup>(initialTab),root=useRef<HTMLDialogElement>(null);useResponsiveDialog(root,true,true);const zh=t.assistant==='助手',label=(id:SettingsGroup)=>labels[id][zh?0:1];
  return <dialog ref={root} className="preferences settings-dialog v2-settings" aria-label={t.settings} onCancel={event=>{event.preventDefault();onClose();}}>
    <header><h2>{t.settings}</h2><button aria-label={t.close} onClick={onClose}><WorkbenchIcon name="close"/></button></header>
    <div className="settings-layout"><nav role="tablist" aria-label={t.settings} aria-orientation="vertical">{(Object.keys(labels) as SettingsGroup[]).map(id=><button key={id} id={'settings-tab-'+id} role="tab" aria-controls={'settings-pane-'+id} aria-selected={tab===id} tabIndex={tab===id?0:-1} onClick={()=>setTab(id)} onKeyDown={event=>{const ids=Object.keys(labels) as SettingsGroup[],index=ids.indexOf(id);if(['ArrowDown','ArrowRight','ArrowUp','ArrowLeft','Home','End'].includes(event.key)){event.preventDefault();const next=ids[event.key==='Home'?0:event.key==='End'?ids.length-1:(index+(['ArrowDown','ArrowRight'].includes(event.key)?1:ids.length-1))%ids.length]!;setTab(next);document.getElementById('settings-tab-'+next)?.focus();}}}>{label(id)}</button>)}</nav>
      <div className="settings-main">
        {(['general','editing'] as const).map(id=><section className="settings-pane" key={id} id={'settings-pane-'+id} role="tabpanel" aria-labelledby={'settings-tab-'+id} hidden={tab!==id}><h3>{label(id)}</h3><GeneralSettings config={config} controller={controller} t={t} theme={theme} onTheme={onTheme} section={id}/></section>)}
        <div hidden={tab!=='models'&&tab!=='assistant'}><ModelSettings config={config} controller={controller} t={t} featuresOnly={tab==='assistant'}/></div>
        <section className="settings-pane" id="settings-pane-data" role="tabpanel" aria-labelledby="settings-tab-data" hidden={tab!=='data'}><h3>{label('data')}</h3><dl className="settings-data"><dt>{zh?'笔记本文件':'Notebook files'}</dt><dd>{zh?'使用文件菜单保存为 .omnb；文件只包含笔记本源码。':'Save .omnb files from the file menu. They contain notebook source only.'}</dd><dt>{zh?'助手会话与附件':'Conversations and attachments'}</dt><dd>{zh?'保存在此浏览器的 IndexedDB 中，可从助手的历史对话恢复最后记录的源码与会话；它们不包含在 .omnb 文件中。':'Stored in this browser’s IndexedDB. Assistant history can restore its last recorded source and conversation; they are not included in .omnb files.'}</dd><dt>{zh?'模型设置与密钥':'Model settings and keys'}</dt><dd>{zh?'设置保存在此浏览器的本地存储中。只有勾选“记住密钥”的供应商会持久保存密钥；其余密钥仅在本次页面会话内可用。':'Settings are saved in this browser’s local storage. Keys are persisted only for providers with “Remember key” selected; other keys last for this page session.'}</dd></dl></section>
      </div>
    </div>
  </dialog>;
}
