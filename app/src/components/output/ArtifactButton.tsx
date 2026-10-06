import {useEffect,useRef,useState} from 'react';
import type {KernelClient,Request} from '../../kernel/client';
import type {Messages} from '../../i18n';
import {saveArtifact} from '../../state/files';
/** Export callbacks are bound to the currently successful value/viewport, not the next pending source. */
export function ArtifactButton({kernel,request,title,label,version,enabled,t}: {kernel:KernelClient;request:()=>Request|undefined;title:string;label:string;version:unknown;enabled:boolean;t:Messages}){
  const [busy,setBusy]=useState(false),[status,setStatus]=useState<string|null>(null);
  const owner=useRef({alive:true,revision:0});
  useEffect(()=>{const state=owner.current;state.alive=true;state.revision++;setBusy(false);setStatus(null);return()=>{state.alive=false;state.revision++;};},[version,enabled]);
  return <span className="artifact-action"><button disabled={!enabled||busy} onClick={()=>{
    const operation=request();if(!enabled||busy||!operation)return;
    const state=owner.current,revision=++state.revision;setBusy(true);setStatus(null);
    void kernel.request(operation).then(async response=>{
      if(!state.alive||state.revision!==revision)return;
      if(response.type!=='artifact')throw new Error(response.type==='error'?response.message:t.exportFailure);
      const saved=await saveArtifact(response.artifact,title,kernel.kind);
      if(state.alive&&state.revision===revision&&saved)setStatus(kernel.kind==='tauri'?t.exportSaved:t.exportDownload);
    }).catch(error=>{if(state.alive&&state.revision===revision)setStatus(error instanceof Error?error.message:t.exportFailure);})
      .finally(()=>{if(state.alive&&state.revision===revision)setBusy(false);});
  }}>{busy?t.exporting:label}</button>{status&&<span role="status">{status}</span>}</span>;
}
