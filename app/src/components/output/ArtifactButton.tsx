import {useEffect,useRef,useState} from 'react';
import type {KernelClient,Request} from '../../kernel/client';
import type {Messages} from '../../i18n';
import type {Artifact} from '../../kernel/generated/Artifact';
import {saveArtifact} from '../../state/files';
/** Export callbacks are bound to the currently successful value/viewport, not the next pending source. */
export function ArtifactButton({kernel,request,createArtifact,title,label,version,enabled,t,onStatus}: {kernel:KernelClient;request?:()=>Request|undefined;createArtifact?:()=>Artifact|Promise<Artifact>;title:string;label:string;version:unknown;enabled:boolean;t:Messages;onStatus?:((message:string|null)=>void)|undefined}){
  const [busy,setBusy]=useState(false),[status,setStatus]=useState<string|null>(null);
  const owner=useRef({alive:true,revision:0});
  useEffect(()=>{const state=owner.current;state.alive=true;state.revision++;setBusy(false);setStatus(null);onStatus?.(null);return()=>{state.alive=false;state.revision++;};},[version,enabled,onStatus]);
  const publish=(message:string)=>{setStatus(message);onStatus?.(`${label}: ${message}`);};
  return <span className="artifact-action"><button disabled={!enabled||busy} onClick={()=>{
    const operation=request?.();if(!enabled||busy||(!operation&&!createArtifact))return;
    const state=owner.current,revision=++state.revision;setBusy(true);setStatus(null);onStatus?.(`${label}: ${t.exporting}`);
    void (async()=>{
      if(createArtifact)return createArtifact();
      const response=await kernel.request(operation!);
      if(response.type!=='artifact')throw new Error(response.type==='error'?response.message:t.exportFailure);
      return response.artifact;
    })().then(async artifact=>{
      if(!state.alive||state.revision!==revision)return;
      const saved=await saveArtifact(artifact,title,kernel.kind);
      if(state.alive&&state.revision===revision&&saved)publish(kernel.kind==='tauri'?t.exportSaved:t.exportDownload);
      else if(state.alive&&state.revision===revision)onStatus?.(null);
    }).catch(error=>{if(state.alive&&state.revision===revision)publish(error instanceof Error?error.message:t.exportFailure);})
      .finally(()=>{if(state.alive&&state.revision===revision)setBusy(false);});
  }}>{busy?t.exporting:label}</button>{status&&!onStatus&&<span role="status">{status}</span>}</span>;
}
