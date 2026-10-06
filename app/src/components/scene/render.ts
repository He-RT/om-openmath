import type {Scene3DData} from '../../kernel/generated/Scene3DData';
import {matrices,normalizePoint,type Camera,type Vec3} from './camera';
const vertex=`#version 300 es
precision highp float;
layout(location=0) in vec3 position;
layout(location=1) in vec3 normal;
layout(location=2) in vec4 color;
uniform mat4 mvp;
uniform mat3 normal_matrix;
uniform float point_size;
out vec3 n;
out vec4 c;
void main(){gl_Position=mvp*vec4(position,1.0);gl_PointSize=point_size;n=normal_matrix*normal;c=color;}`;
const fragment=`#version 300 es
precision highp float;
in vec3 n;in vec4 c;
uniform float lit;
uniform float point_mode;
out vec4 output_color;
void main(){if(point_mode>0.5&&distance(gl_PointCoord,vec2(0.5))>0.5)discard;float light=1.0;if(lit>0.5)light=0.35+0.65*abs(dot(normalize(n),normalize(vec3(0.4,0.7,1.0))));output_color=vec4(c.rgb*light,c.a);}`;
interface Batch {vao:WebGLVertexArrayObject;buffer:WebGLBuffer;indices:WebGLBuffer|null;count:number;mode:number;triangles:number[][]|null;positions:Vec3[];transparent:boolean;size:number}
/** GPU renders immutable kernel geometry. Only display/camera normalization and transparent ordering happen here. */
export function createRenderer(canvas:HTMLCanvasElement,data:Scene3DData){
 const gl=canvas.getContext('webgl2',{antialias:true,alpha:false,preserveDrawingBuffer:true});
 if(!gl)throw new Error('WebGL2 unavailable');
 const shader=(type:number,source:string)=>{const s=gl.createShader(type);if(!s)throw new Error('Shader unavailable');gl.shaderSource(s,source);gl.compileShader(s);if(!gl.getShaderParameter(s,gl.COMPILE_STATUS)){gl.deleteShader(s);throw new Error('WebGL2 shader compilation failed');}return s;};
 const program=gl.createProgram();if(!program)throw new Error('WebGL2 program unavailable');
 let vs:WebGLShader|undefined,fs:WebGLShader|undefined;
 try {vs=shader(gl.VERTEX_SHADER,vertex);fs=shader(gl.FRAGMENT_SHADER,fragment);gl.attachShader(program,vs);gl.attachShader(program,fs);gl.linkProgram(program);}
 catch(error){gl.deleteProgram(program);throw error;}finally{if(vs)gl.deleteShader(vs);if(fs)gl.deleteShader(fs);}
 if(!gl.getProgramParameter(program,gl.LINK_STATUS)){gl.deleteProgram(program);throw new Error('WebGL2 shader link failed');}
 const uniforms={mvp:gl.getUniformLocation(program,'mvp'),normal:gl.getUniformLocation(program,'normal_matrix'),lit:gl.getUniformLocation(program,'lit'),point:gl.getUniformLocation(program,'point_mode'),size:gl.getUniformLocation(program,'point_size')};
 const batches:Batch[]=[];
 const buffers:WebGLBuffer[]=[],vaos:WebGLVertexArrayObject[]=[];
 const dispose=()=>{for(const b of buffers)gl.deleteBuffer(b);for(const v of vaos)gl.deleteVertexArray(v);gl.deleteProgram(program);};
 const batch=(world:Vec3[],normals:Vec3[],colors:[number,number,number,number][],triangles:number[][]|null,mode:number,size:number)=>{
  if(world.length!==normals.length||world.length!==colors.length)throw new Error('Invalid kernel geometry');
  const positions=world.map(p=>normalizePoint(p,data.bounds));const values=new Float32Array(world.length*10);
  for(let i=0;i<positions.length;i++){const p=positions[i]!,n=normals[i]!,c=colors[i]!;if([...p,...n,...c].some(v=>!Number.isFinite(v)))throw new Error('Nonfinite kernel geometry');values.set([...p,...n,...c],i*10);}
  const vao=gl.createVertexArray(),buffer=gl.createBuffer();if(vao)vaos.push(vao);if(buffer)buffers.push(buffer);if(!vao||!buffer)throw new Error('WebGL2 buffer allocation failed');gl.bindVertexArray(vao);gl.bindBuffer(gl.ARRAY_BUFFER,buffer);gl.bufferData(gl.ARRAY_BUFFER,values,gl.STATIC_DRAW);
  for(const [index,count,offset]of [[0,3,0],[1,3,12],[2,4,24]]){gl.enableVertexAttribArray(index!);gl.vertexAttribPointer(index!,count!,gl.FLOAT,false,40,offset!);}
  const indices=triangles?gl.createBuffer():null;if(indices)buffers.push(indices);
  if(triangles){if(!indices)throw new Error('WebGL2 index buffer unavailable');if(triangles.flat().some(i=>!Number.isSafeInteger(i)||i<0||i>=world.length))throw new Error('Invalid triangle index');gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER,indices);gl.bufferData(gl.ELEMENT_ARRAY_BUFFER,new Uint32Array(triangles.flat()),gl.DYNAMIC_DRAW);}
  batches.push({vao,buffer,indices,count:triangles?triangles.length*3:world.length,mode,triangles,positions,transparent:colors.some(c=>c[3]<1),size});
 };
 try {
 for(const mesh of data.meshes)batch(mesh.positions,mesh.normals,mesh.colors,mesh.triangles,gl.TRIANGLES,1);
 for(const line of data.lines)batch(line.positions,line.positions.map(()=>[0,0,0]),line.colors,null,gl.LINE_STRIP,line.width);
 for(const p of data.points)batch([p.position],[[0,0,0]],[p.color],null,gl.POINTS,Math.max(1,Math.min(64,p.radius*2)));
 // Actual coordinate endpoints form display axes; these are camera aids, not newly sampled functions.
 const lo=data.bounds[0],hi=data.bounds[1];
 const axisPoints:Vec3[]=[lo,[hi[0],lo[1],lo[2]],lo,[lo[0],hi[1],lo[2]],lo,[lo[0],lo[1],hi[2]]];
 batch(axisPoints,axisPoints.map(()=>[0,0,0]),[[.65,.25,.25,1],[.65,.25,.25,1],[.2,.5,.3,1],[.2,.5,.3,1],[.2,.35,.7,1],[.2,.35,.7,1]],null,gl.LINES,1);
 } catch(error){dispose();throw error;}
 const draw=(camera:Camera)=>{
  const rect=canvas.getBoundingClientRect(),ratio=Math.min(2,window.devicePixelRatio||1),width=Math.max(1,Math.round(rect.width*ratio)),height=Math.max(1,Math.round(rect.height*ratio));
  if(canvas.width!==width||canvas.height!==height){canvas.width=width;canvas.height=height;}
  gl.viewport(0,0,width,height);gl.clearColor(.98,.99,.98,1);gl.clear(gl.COLOR_BUFFER_BIT|gl.DEPTH_BUFFER_BIT);gl.enable(gl.DEPTH_TEST);gl.depthFunc(gl.LEQUAL);gl.disable(gl.CULL_FACE);gl.useProgram(program);
  const m=matrices(camera,width/height);gl.uniformMatrix4fv(uniforms.mvp,false,m.mvp);gl.uniformMatrix3fv(uniforms.normal,false,m.normal);
  const depth=(b:Batch)=>b.positions.reduce((sum,p)=>sum+m.view[2]!*p[0]+m.view[6]!*p[1]+m.view[10]!*p[2],0)/Math.max(1,b.positions.length);
  const lineWidthRange=gl.getParameter(gl.ALIASED_LINE_WIDTH_RANGE) as Float32Array;
  const sorted=[...batches.filter(b=>!b.transparent),...batches.filter(b=>b.transparent).sort((a,b)=>depth(a)-depth(b))];
  for(const b of sorted){if(b.mode===gl.LINE_STRIP||b.mode===gl.LINES)gl.lineWidth(Math.max(lineWidthRange[0]!,Math.min(lineWidthRange[1]!,b.size*ratio)));gl.bindVertexArray(b.vao);gl.uniform1f(uniforms.lit,b.mode===gl.TRIANGLES?1:0);gl.uniform1f(uniforms.point,b.mode===gl.POINTS?1:0);gl.uniform1f(uniforms.size,b.size*ratio);
   if(b.transparent){gl.enable(gl.BLEND);gl.blendFunc(gl.SRC_ALPHA,gl.ONE_MINUS_SRC_ALPHA);gl.depthMask(false);if(b.triangles){const order=[...b.triangles].sort((a,c)=>{
    const z=(f:number[])=>f.reduce((sum,i)=>{const p=b.positions[i]!;return sum+m.view[2]!*p[0]+m.view[6]!*p[1]+m.view[10]!*p[2];},0);return z(a)-z(c);});gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER,b.indices);gl.bufferData(gl.ELEMENT_ARRAY_BUFFER,new Uint32Array(order.flat()),gl.DYNAMIC_DRAW);}}
   else{gl.disable(gl.BLEND);gl.depthMask(true);}
   if(b.indices)gl.drawElements(b.mode,b.count,gl.UNSIGNED_INT,0);else gl.drawArrays(b.mode,0,b.count);
  }gl.depthMask(true);gl.bindVertexArray(null);
 };
 return {draw,dispose};
}
