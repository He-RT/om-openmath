import { expect, test, vi } from 'vitest';
import { WasmClient, type WorkerPort } from './wasmClient';
import { TauriClient, type NativeTransport } from './tauriClient';
import type { Request, Response, Event } from './client';
import type { KernelConfig } from './generated/KernelConfig';
import type { NotebookFile } from './generated/NotebookFile';
const config: KernelConfig = { general: { language: 'en', dialect: 'modern', constants: 'math', reactive: true, auto_run_dependents: true, show_steps: true, auto_plot: false, eval_timeout_ms: 30000 }, llm: { enabled: false, translate: '', explain: '', complete: '', chat: '', fix: '', send_context: false, profiles: [] } };
class Port implements WorkerPort {
  onmessage: WorkerPort['onmessage'] = null; onerror: WorkerPort['onerror'] = null;
  sent: { id: number; body: Request }[] = []; terminated = false;
  commits: { id: number; expected: string; desired: string }[] = [];
  postMessage(message: unknown) {
    const m = message as { type: string; envelope?: string };
    if (m.type === 'init') queueMicrotask(() => this.onmessage?.(new MessageEvent('message', { data: { type: 'ready' } })));
    else if (m.type === 'commit-source') this.commits.push(message as { id: number; expected: string; desired: string });
    else {
      const envelope = JSON.parse(m.envelope ?? '') as { id: number; body: Request }; this.sent.push(envelope);
      if (envelope.body.type === 'get_config') queueMicrotask(() => this.reply(envelope.id, { type: 'config', config }));
      if (envelope.body.type === 'load_notebook') queueMicrotask(() => this.reply(envelope.id, { type: 'ok' }));
      if (envelope.body.type === 'restore_definitions') queueMicrotask(() => this.reply(envelope.id, { type: 'notebook_state', state: { file: { version: 1, title: '', cells: [] }, cells: [], definition_order: [], cycles: [] } }, [{ type: 'kernel_restarted', message: 'restarted' }]));
    }
  }
  reply(id: number, body: Response, events: Event[] = []) { this.onmessage?.(new MessageEvent('message', { data: { response: { id, body }, events: events.map(body => ({ id: 0, body })) } })); }
  terminate() { this.terminated = true; }
}
for(const accepted of [true,false])test(`source commit acknowledgement controls restart mirror even with concurrent reads: ${accepted}`,async()=>{
  const ports:Port[]=[];
  const client=new WasmClient(()=>{const port=new Port();ports.push(port);return port;});await client.ready;
  const before:NotebookFile={version:1,title:'before',cells:[]},after:NotebookFile={version:1,title:'after',cells:[{id:'new',source:'let a=5',kind:'Math',dialect:'Modern'}]};
  await client.request({type:'load_notebook',file:before});
  const commit=client.commitSourceFile(before,after);await vi.waitFor(()=>expect(ports[0]!.commits).toHaveLength(1));
  expect(JSON.parse(ports[0]!.commits[0]!.expected)).toEqual(before);
  expect(JSON.parse(ports[0]!.commits[0]!.desired)).toEqual(after);
  const reading=client.request({type:'get_notebook_state'});await vi.waitFor(()=>expect(ports[0]!.sent.at(-1)?.body.type).toBe('get_notebook_state'));
  const state=(file:NotebookFile)=>({file,cells:[],definition_order:[],cycles:[]});
  ports[0]!.reply(ports[0]!.sent.at(-1)!.id,{type:'notebook_state',state:state(before)});await reading;
  ports[0]!.reply(ports[0]!.commits[0]!.id,accepted?{type:'notebook_state',state:state(after)}:{type:'error',message:'STALE_SNAPSHOT'});
  await commit;await client.interrupt();
  const load=ports[1]!.sent.find(request=>request.body.type==='load_notebook')!.body;
  expect(load.type==='load_notebook'&&load.file).toEqual(accepted?after:before);
  expect(ports[0]!.sent.some(request=>request.body.type==='evaluate'||request.body.type==='run_all')).toBe(false);
  client.dispose();
});
test('wasm request IDs handle replies in reverse order, event unsubscribe and disposal', async () => {
  const port = new Port(); const client = new WasmClient(() => port); await client.ready;
  const events: Event[] = []; const off = client.onEvent(e => events.push(e));
  const a = client.request({ type: 'save_notebook' }); const b = client.request({ type: 'get_notebook_state' });
  await vi.waitFor(() => expect(port.sent.length).toBe(3));
  const last = port.sent.at(-1)?.id; const first = port.sent.at(-2)?.id;
  if (first === undefined || last === undefined) throw new Error('missing request');
  port.reply(last, { type: 'ok' }, [{ type: 'cell_status', cell_id: 'a', status: 'Stale' }]);
  port.reply(first, { type: 'notebook', file: { version: 1, title: 'actual correlation', cells: [] } });
  expect((await a).type).toBe('notebook'); expect((await b).type).toBe('ok'); expect(events).toHaveLength(1);
  off(); port.reply(0, { type: 'ok' }, [{ type: 'cell_status', cell_id: 'a', status: 'Done' }]); expect(events).toHaveLength(1);
  client.dispose(); expect(port.terminated).toBe(true); await expect(client.request({ type: 'run_all' })).rejects.toThrow('disposed');
});
test('interrupt captures the latest in-flight math source, terminates and restores without run_all', async () => {
  const ports: Port[] = []; const client = new WasmClient(() => { const port = new Port(); ports.push(port); return port; }); await client.ready;
  const request = client.request({ type: 'evaluate', cell_id: 'current', source: 'factorial(1000000)', dialect: 'Modern' });
  const rejected = expect(request).rejects.toThrow('interrupted');
  await vi.waitFor(() => expect(ports[0]?.sent.at(-1)?.body.type).toBe('evaluate'));
  await client.interrupt(); await rejected;
  expect(ports[0]?.terminated).toBe(true);
  const load = ports[1]?.sent.find(r => r.body.type === 'load_notebook')?.body;
  if (load?.type !== 'load_notebook') throw new Error('missing restore load');
  expect(load.file.cells[0]?.source).toBe('factorial(1000000)');
  expect(ports[1]?.sent.map(r => r.body.type)).toEqual(['load_notebook', 'restore_definitions']);
  client.dispose();
});
test('tauri subscribes once, checks IDs and delivers only public zero-ID events', async () => {
  const channel: { onmessage: (message: string) => void } = { onmessage: () => {} }; const commands: string[] = [];
  const transport: NativeTransport = { channel: () => channel, invoke: async <T,>(command: string, args?: Record<string, unknown>): Promise<T> => {
    commands.push(command);
    if (command === 'kernel_request') { const request = JSON.parse(String(args?.envelope)) as { id: number }; return JSON.stringify({ id: request.id, body: { type: 'ok' } }) as T; }
    return undefined as T;
  } };
  const client = new TauriClient(transport); const events: Event[] = []; const off = client.onEvent(e => events.push(e));
  await expect(client.request({ type: 'run_all' })).resolves.toEqual({ type: 'ok' });
  channel.onmessage(JSON.stringify({ id: 0, body: { type: 'cell_status', cell_id: 'a', status: 'Done' } }));
  expect(events).toHaveLength(1); off(); await client.interrupt(); client.dispose();
  expect(commands).toEqual(['kernel_subscribe', 'kernel_request', 'kernel_interrupt']);
});
test('disposing before Worker readiness rejects initialization instead of leaving pending waiters', async () => {
  const port = new Port(); port.postMessage = () => {};
  const client = new WasmClient(() => port);
  const initialized = expect(client.ready).rejects.toThrow('disposed');
  client.dispose(); await initialized; expect(port.terminated).toBe(true);
});
