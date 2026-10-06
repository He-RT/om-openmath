import {useEffect,useRef,useState} from 'react';
import type {OutputItem} from '../../kernel/generated/OutputItem';
import type {KernelClient} from '../../kernel/client';
import type {Messages,Locale} from '../../i18n';
import {ArtifactButton} from '../output/ArtifactButton';
import {createRenderer} from './render';
import {defaultCamera,fittedCamera,normalizePoint,project,zoom,type Camera} from './camera';
export interface CameraMemory {current:Camera;fitted?:boolean}
export function SceneView({item,kernel,t,language,active=true,cameraMemory}: {item:Extract<OutputItem,{type:'scene3_d'}>;kernel:KernelClient;t:Messages;language:Locale;active?:boolean;cameraMemory?:CameraMemory|undefined}){
 const zh=language==='zh-CN',text=(a:string,b:string)=>zh?a:b;
 const canvas=useRef<HTMLCanvasElement>(null),renderer=useRef<ReturnType<typeof createRenderer>|null>(null);
 const [camera,setCamera]=useState<Camera>(()=>{const initial=cameraMemory?.fitted?cameraMemory.current:item.data?fittedCamera(item.data):defaultCamera;if(cameraMemory){cameraMemory.current=initial;cameraMemory.fitted=true;}return initial;}),[error,setError]=useState<string|null>(null),[showData,setShowData]=useState(false),[width,setWidth]=useState(640);
 const [restore,setRestore]=useState(0);const current=useRef(camera);current.current=camera;
 const source=item.request.expressions.join(', '),data=item.data;
 const resetCamera=data?fittedCamera(data):defaultCamera;
 const axisNames=item.request.kind==='parametric'?['x','y','z']:[item.request.axes[0]?.name??'x',item.request.axes[1]?.name??'y',item.request.axes[2]?.name??'z'];
 const update=(next:Camera)=>{if(!active)return;setCamera(next);if(cameraMemory)cameraMemory.current=next;};
 useEffect(()=>{
  const element=canvas.current;if(!element||!data)return;
  const state={alive:true};
  try{renderer.current=createRenderer(element,data);renderer.current.draw(current.current);setError(null);}catch(failure){setError(failure instanceof Error?failure.message:'WebGL2 unavailable');}
  const resize=()=>{if(state.alive){setWidth(element.getBoundingClientRect().width);renderer.current?.draw(current.current);}};
  const observer=typeof ResizeObserver!=='undefined'?new ResizeObserver(resize):null;observer?.observe(element);
  const lost=(event:Event)=>{event.preventDefault();renderer.current?.dispose();renderer.current=null;setError(text('三维图形上下文已丢失，可复位/导出OBJ','3D graphics context lost; reset or export OBJ'));};
  const regained=()=>setRestore(v=>v+1);element.addEventListener('webglcontextlost',lost);element.addEventListener('webglcontextrestored',regained);resize();
  return()=>{state.alive=false;observer?.disconnect();element.removeEventListener('webglcontextlost',lost);element.removeEventListener('webglcontextrestored',regained);renderer.current?.dispose();renderer.current=null;};
 },[data,restore]);
 useEffect(()=>{renderer.current?.draw(camera);},[camera]);
 const wheel=useRef<(event:WheelEvent)=>void>(()=>{});wheel.current=event=>{if(!active)return;event.preventDefault();update(zoom(current.current,Math.exp(Math.max(-1,Math.min(1,event.deltaY*.002)))));};
 useEffect(()=>{const c=canvas.current;const handler=(e:WheelEvent)=>wheel.current(e);c?.addEventListener('wheel',handler,{passive:false});return()=>c?.removeEventListener('wheel',handler);},[]);
 const drag=useRef<{point:[number,number];camera:Camera;pan:boolean}|null>(null);
 const rotate=(yaw:number,pitch:number)=>update({...camera,yaw:camera.yaw+yaw,pitch:Math.max(-1.5,Math.min(1.5,camera.pitch+pitch))});
 const count=data?.meshes.reduce((sum,m)=>sum+m.triangles.length,0)??0;
 const labels=data?[[data.bounds[1][0],data.bounds[0][1],data.bounds[0][2]],[data.bounds[0][0],data.bounds[1][1],data.bounds[0][2]],[data.bounds[0][0],data.bounds[0][1],data.bounds[1][2]]].map((p,i)=>({name:axisNames[i]!,p:project(normalizePoint(p as [number,number,number],data.bounds),camera,Math.max(1,width)/420)})):[];
 return <figure className="scene-view" aria-label={text('真实三维场景','Actual 3D scene')}>
  <div className="scene-toolbar"><strong>{text('三维场景','3D scene')}</strong><span>{count} {text('个实际三角形','actual triangles')}</span>
   <button disabled={!active||!!error} onClick={()=>rotate(-.15,0)}>{text('左转','Rotate left')}</button><button disabled={!active||!!error} onClick={()=>rotate(.15,0)}>{text('右转','Rotate right')}</button>
   <button disabled={!active||!!error} onClick={()=>rotate(0,.15)}>{text('向上查看','Tilt up')}</button><button disabled={!active||!!error} onClick={()=>rotate(0,-.15)}>{text('向下查看','Tilt down')}</button>
   <button disabled={!active||!!error} onClick={()=>update(zoom(camera,.8))}>{t.zoomIn}</button><button disabled={!active||!!error} onClick={()=>update(zoom(camera,1.25))}>{t.zoomOut}</button>
   <button disabled={!active} onClick={()=>{update({...resetCamera});if(error)setRestore(v=>v+1);}}>{error?t.retry:t.resetView}</button>
  </div>
  {data&&<div className="scene-canvas-wrap" style={error?{display:'none'}:undefined}><canvas ref={canvas} height="420" aria-label={text('三维网格：方向键旋转，Shift加方向键平移，正负键缩放，Home复位','3D mesh: arrows rotate, Shift+arrows pan, plus/minus zoom, Home resets')} role="img" tabIndex={active?0:-1}
   onPointerDown={e=>{if(!active)return;e.currentTarget.setPointerCapture?.(e.pointerId);drag.current={point:[e.clientX,e.clientY],camera,pan:e.shiftKey};}}
   onPointerMove={e=>{const start=drag.current;if(!start||!active)return;const dx=e.clientX-start.point[0],dy=e.clientY-start.point[1];update(start.pan?{...start.camera,panX:start.camera.panX+dx/120,panY:start.camera.panY-dy/120}:{...start.camera,yaw:start.camera.yaw+dx*.006,pitch:Math.max(-1.5,Math.min(1.5,start.camera.pitch+dy*.006))});}}
   onPointerUp={()=>{drag.current=null;}} onPointerCancel={()=>{drag.current=null;}} onDoubleClick={()=>update({...resetCamera})}
   onKeyDown={e=>{if(!active)return;if(['ArrowLeft','ArrowRight','ArrowUp','ArrowDown','+','=','-','Home'].includes(e.key)){e.preventDefault();if(e.key==='Home')update({...resetCamera});else if(e.key==='+'||e.key==='=')update(zoom(camera,.8));else if(e.key==='-')update(zoom(camera,1.25));else if(e.shiftKey)update({...camera,panX:camera.panX+(e.key==='ArrowLeft'?-.1:e.key==='ArrowRight'?.1:0),panY:camera.panY+(e.key==='ArrowUp'?.1:e.key==='ArrowDown'?-.1:0)});else rotate(e.key==='ArrowLeft'?-.1:e.key==='ArrowRight'?.1:0,e.key==='ArrowUp'?-.1:e.key==='ArrowDown'?.1:0);}}}/>
   {!error&&labels.map(label=>label.p&&<span className="scene-axis-label" key={label.name} style={{left:`${label.p[0]*100}%`,top:`${label.p[1]*100}%`}}>{label.name}</span>)}
  </div>}
  {(error||item.unavailable)&&<p role="alert">{item.unavailable??text('此环境暂不能显示WebGL2三维场景；原式与OBJ仍可用。','This environment cannot display the WebGL2 scene; source and OBJ remain available.')} {error}</p>}
  {data&&<><p className="scene-summary">{text('有限机器采样，曲面边界未经认证。','Finite machine samples; no certified surface boundary.')} {data.skipped>0&&`${text('跳过域外/不连续样本：','Skipped undefined/discontinuous samples: ')}${data.skipped}`}</p>
   <p className="scene-bounds">{data.bounds[0].map((lo,i)=><span key={i}>{axisNames[i]}: {lo.toPrecision(6)} … {data.bounds[1][i]!.toPrecision(6)}</span>)}</p>
   <ArtifactButton kernel={kernel} t={t} title="OpenMath-scene" label={text('导出 OBJ','Export OBJ')} version={data} enabled={active} request={()=>({type:'export_scene3_d',data,title:'OpenMath'})}/>
   <details onToggle={e=>setShowData(e.currentTarget.open)}><summary>{text('内核网格数据','Kernel mesh data')}</summary>{showData&&<pre>{JSON.stringify(data,null,2)}</pre>}</details>
  </>}
  <details><summary>{text('原始三维表达式','Original 3D expression')}</summary><pre>{source}</pre></details>
 </figure>;
}
