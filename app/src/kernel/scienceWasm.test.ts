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
  return Number(item.input_form.replace(/`.*$/, ''));
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
