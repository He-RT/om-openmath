export type Vec3=[number,number,number];
export interface Camera { yaw:number; pitch:number; distance:number; panX:number; panY:number }
export const defaultCamera:Camera={yaw:-0.65,pitch:-0.45,distance:6.5,panX:0,panY:0};
export function normalizePoint(point:Vec3,bounds:[Vec3,Vec3]):Vec3{
 const span=Math.max(...bounds[0].map((lo,i)=>bounds[1][i]!-lo));
 const scale=span>0&&Number.isFinite(span)?span/2:1;
 return point.map((v,i)=>(v-(bounds[0][i]!+(bounds[1][i]!-bounds[0][i]!)/2))/scale) as Vec3;
}
export function matrices(camera:Camera,aspect:number){
 const y=Math.cos(camera.yaw),s=Math.sin(camera.yaw),p=Math.cos(camera.pitch),t=Math.sin(camera.pitch);
 const view=[y,t*s,-p*s,0,0,p,t,0,s,-t*y,p*y,0,camera.panX,camera.panY,-camera.distance*Math.max(1,1/aspect),1];
 const f=1/Math.tan(Math.PI/8),near=.01,far=100;
 const projection=[f/aspect,0,0,0,0,f,0,0,0,0,(far+near)/(near-far),-1,0,0,2*far*near/(near-far),0];
 const mvp=new Float32Array(16);
 for(let c=0;c<4;c++)for(let r=0;r<4;r++)for(let k=0;k<4;k++)mvp[c*4+r]!+=projection[k*4+r]!*view[c*4+k]!;
 return {mvp,normal:new Float32Array([view[0]!,view[1]!,view[2]!,view[4]!,view[5]!,view[6]!,view[8]!,view[9]!,view[10]!]),view};
}
export function project(point:Vec3,camera:Camera,aspect:number):[number,number,number]|null{
 const m=matrices(camera,aspect).mvp;
 const clip=[0,0,0,0];for(let r=0;r<4;r++)clip[r]=m[r]!*point[0]+m[4+r]!*point[1]+m[8+r]!*point[2]+m[12+r]!;
 if(clip[3]!<=0)return null;
 return [(clip[0]!/clip[3]!+1)/2,(1-clip[1]!/clip[3]!)/2,clip[2]!/clip[3]!];
}
export function zoom(camera:Camera,factor:number):Camera{return Number.isFinite(factor)&&factor>0?{...camera,distance:Math.max(1.5,Math.min(30,camera.distance*factor))}:camera;}
/** Fit an already sampled shape in display space; source functions are never evaluated by the camera. */
export function fittedCamera(data:import('../../kernel/generated/Scene3DData').Scene3DData):Camera{
 let radius=0;
 const consider=(point:Vec3)=>{const p=normalizePoint(point,data.bounds);radius=Math.max(radius,Math.hypot(...p));};
 for(const mesh of data.meshes)for(const p of mesh.positions)consider(p);
 for(const line of data.lines)for(const p of line.positions)consider(p);
 for(const p of data.points)consider(p.position);
 return {...defaultCamera,distance:Math.max(1.5,Math.min(30,radius/Math.sin(Math.PI/8)*1.12))};
}
