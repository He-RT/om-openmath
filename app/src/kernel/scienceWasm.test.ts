import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { beforeAll, expect, it } from 'vitest';
import { initSync, Kernel } from './wasm/om_wasm.js';
import type { Request } from './generated/Request';
import type { Response } from './generated/Response';
import type { Envelope } from './generated/Envelope';
import type { CellOutput } from './generated/CellOutput';

beforeAll(() => {
  initSync({ module: new Uint8Array(readFileSync(resolve(process.cwd(), 'src/kernel/wasm/om_wasm_bg.wasm'))) });
});
function request(kernel: Kernel, body: Request): Response {
  const packet = JSON.parse(kernel.request(JSON.stringify({ id: 7, body }))) as { response: Envelope<Response> };
  expect(packet.response.id).toBe(7);
  return packet.response.body;
}
function evaluate(kernel: Kernel, id: string, source: string): CellOutput {
  const reply = request(kernel, { type: 'evaluate', cell_id: id, source, dialect: 'Modern' });
  if (reply.type !== 'evaluated') throw new Error('expected real evaluated reply');
  expect(reply.output.messages).toEqual([]);
  return reply.output;
}
function number(output: CellOutput): number {
  const item = output.items.filter(item => item.type === 'expr').at(-1);
  if (item?.type !== 'expr') throw new Error('missing actual numeric result');
  // Wolfram machine markers describe precision; values are checked against independent JS math.
  return Number(item.input_form.replace(/`(?:[-+]?\d+(?:\.\d*)?)?/g, '').replace(/\*\^/g, 'e'));
}
it('actual WASM continuous ODE survives parameter re-evaluation and source-only saving', () => {
  const kernel = new Kernel();
  try {
    evaluate(kernel, 'rate', 'let rate=1');
    evaluate(kernel, 'solution', 'let solution=ode(fn(t,y)=>rate*y,initial:1,t:0..1)');
    expect(number(evaluate(kernel, 'point', 'solution.solution(0.37)'))).toBeCloseTo(Math.exp(0.37), 7);
    evaluate(kernel, 'rate', 'let rate=2');
    expect(number(evaluate(kernel, 'point', 'solution.solution(0.37)'))).toBeCloseTo(Math.exp(0.74), 7);
    const saved = request(kernel, { type: 'save_notebook' });
    if (saved.type !== 'notebook') throw new Error('expected genuine notebook reply');
    expect(saved.file.version).toBe(1);
    expect(JSON.stringify(saved.file)).not.toContain('InterpolationData');
    expect(saved.file.cells.some(cell => cell.source.includes('fn(t,y)'))).toBe(true);
  } finally { kernel.free(); }
});
it('actual WASM Hermite, event endpoint and sampling use kernel data', () => {
  const kernel = new Kernel();
  try {
    expect(number(evaluate(kernel, 'cubic', 'interpolate([[0,0,0],[1,1,3]],method:"hermite")(0.37)'))).toBeCloseTo(0.37 ** 3, 13);
    evaluate(kernel, 'event', 'let stopped=ode(fn(t,y)=>y,initial:1,t:0..2,event:fn(t,y)=>y-2)');
    expect(number(evaluate(kernel, 'time', 'stopped.domain[2]'))).toBeCloseTo(Math.log(2), 7);
    const sample = evaluate(kernel, 'samples', 'sample(fn(x)=>x^2,x:0..1,count:5)');
    expect(sample.items.some(item => item.type === 'expr' && item.input_form.startsWith('DataTable['))).toBe(true);
  } finally { kernel.free(); }
});
it('actual WASM keeps exact global certificates distinct from local candidates', () => {
  const kernel = new Kernel();
  const input = (output: CellOutput) => {
    const item = output.items.filter(item => item.type === 'expr').at(-1);
    if (item?.type !== 'expr') throw new Error('missing actual expression');
    return item.input_form;
  };
  try {
    evaluate(kernel, 'optimum', 'let optimum=optimize(3*x^2+2*x*y+2*y^2-4*x+6*y+9,[x,y],scope:"global")');
    expect(input(evaluate(kernel, 'value', 'optimum.value'))).toBe('-2/5');
    expect(input(evaluate(kernel, 'certificate', 'optimum.guarantee'))).toBe('"certified_global"');
    expect(input(evaluate(kernel, 'residual', 'optimum.certificate.kkt_residual')).replace(/\s/g, '')).toBe('{0,0}');
    evaluate(kernel, 'local', 'let local=optimize(-x^2,x,initial:0)');
    expect(input(evaluate(kernel, 'kind', 'local.guarantee'))).toBe('"numerical_stationary_candidate"');
    expect(input(evaluate(kernel, 'proof', 'local.certificate'))).toBe('Null');
  } finally { kernel.free(); }
});
it('actual WASM fits callable QR and nonlinear models and reports real residuals', () => {
  const kernel = new Kernel();
  try {
    evaluate(kernel, 'line', 'let line=fit([[0,1],[1,3],[2,5]],model:a*x+b,parameters:[a,b])');
    expect(number(evaluate(kernel, 'prediction', 'line.model(3)'))).toBeCloseTo(7, 12);
    evaluate(kernel, 'curve', 'let curve=fit([[0,2],[1,numeric(2*exp(0.3))],[2,numeric(2*exp(0.6))],[3,numeric(2*exp(0.9))]],model:a*exp(b*x),parameters:{a:1,b:0},method:"nonlinear")');
    expect(number(evaluate(kernel, 'rate', 'curve.parameters.b'))).toBeCloseTo(0.3, 7);
    expect(number(evaluate(kernel, 'value', 'curve.model(0.5)'))).toBeCloseTo(2 * Math.exp(0.15), 7);
    expect(number(evaluate(kernel, 'residual', 'curve.residual_norm'))).toBeLessThan(1e-7);
  } finally { kernel.free(); }
});
it('actual WASM extended 2D samples carry true values, counts and original log coordinates', () => {
  const kernel=new Kernel();
  const plot=(source:string)=>{
    const item=evaluate(kernel,'plot',source).items[0];
    if(item?.type!=='plot')throw new Error('missing actual plot');
    return item;
  };
  try {
    const circle=plot('parametric_plot([cos(t),sin(t)],t:0..2*pi)');
    for(const point of circle.data.curves[0]!.segments.flat())expect(point[0]**2+point[1]**2).toBeCloseTo(1,12);
    const histogram=plot('histogram([1,1,2,3,3],bins:3)');
    expect(histogram.data.geometry?.tiles.map(t=>t.value)).toEqual([2,1,2]);
    const field=plot('field_plot([-y,x],x:-2..2,y:-2..2)');
    expect(field.data.geometry?.arrows).toHaveLength(400);
    for(const a of field.data.geometry!.arrows)expect(a.value).toEqual([-a.start[1],a.start[0]]);
    const density=plot('plot(x*y,x:-2..2,y:-2..2,view:"density")');
    expect(density.data.geometry?.tiles).toHaveLength(9216);
    const t=density.data.geometry!.tiles[150]!;
    expect(t.value).toBeCloseTo((t.bounds[0][0]+t.bounds[1][0])/2*(t.bounds[0][1]+t.bounds[1][1])/2,12);
    const log=plot('plot(x^2,x:1..1000,scale:"log_log")');
    expect(log.data.scale).toBe('log_log');
    expect(log.data.curves[0]!.segments.flat().at(-1)).toEqual([1000,1000000]);
  }finally{kernel.free();}
});
it('actual WASM immutable exploration context restores exact state without replay or notebook mutation',()=>{
  const main=new Kernel(),task=new Kernel();
  try{
    evaluate(main,'definitions','let a=99;let rate=2;let f(x)=rate*x');
    const output=evaluate(main,'explore','explore([[a,f(a)],[rate,a^2]],controls:{a:0..4})');
    const item=output.items[0];if(item?.type!=='explore')throw new Error('missing actual exploration');
    const ctx=request(main,{type:'get_explore_context',query:{cell_id:'explore',out_index:item.out_index,view_id:item.view_id}});
    if(ctx.type!=='explore_context')throw new Error('no readonly context');
    evaluate(main,'definitions','let a=100;let rate=20;let f(x)=rate*x');
    const answer=request(task,{type:'run_explore_context',context:ctx.context,values:{a:3},revision:7});
    if(answer.type!=='explored'||answer.result.item.type!=='expr')throw new Error('no genuine result');
    expect(answer.result.item.input_form).toBe('{{3., 6.}, {2, 9.}}');
    expect(answer.result.revision).toBe(7);
    expect(number(evaluate(main,'main-value','a'))).toBe(100);
    expect(number(evaluate(main,'main-rate','rate'))).toBe(20);
    const file=request(task,{type:'save_notebook'});if(file.type!=='notebook')throw new Error('no file');expect(file.file.cells).toHaveLength(0);
  }finally{main.free();task.free();}
});
it('actual WASM creates original-coordinate 3D samples and mobile fallback suppresses unseen meshes',()=>{
 const kernel=new Kernel();
 try{
  const output=evaluate(kernel,'scene','parametric_plot([cos(u)*sin(v),sin(u)*sin(v),cos(v)],u:0..2*pi,v:0..pi,mesh_points:12)');
  const item=output.items[0];if(item?.type!=='scene3_d'||!item.data)throw new Error('No actual mesh');
  for(const p of item.data.meshes[0]!.positions)expect(p[0]**2+p[1]**2+p[2]**2).toBeCloseTo(1,11);
  const obj=request(kernel,{type:'export_scene3_d',data:item.data,title:'actual unit sphere'});if(obj.type!=='artifact')throw new Error('No actual OBJ');expect(Buffer.from(obj.artifact.base64,'base64').toString()).toContain('\nf ');
  request(kernel,{type:'set_host_platform',platform:'ios'});
  const mobile=evaluate(kernel,'scene','plot(x+y,x:0..1,y:0..1)');const view=mobile.items[0];if(view?.type!=='scene3_d')throw new Error('No explicit mobile fallback');expect(view.data).toBeNull();expect(view.unavailable).toContain('尚未适配');
 }finally{kernel.free();}
});
