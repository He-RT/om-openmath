/// <reference lib="webworker" />
import init, { Kernel } from './wasm/om_wasm.js';
let kernel: Kernel | undefined;
let initialize: Promise<void> | undefined;
self.onmessage = (event: MessageEvent<{ type: 'init'; config?: string } | { type: 'request'; envelope: string; id: number } | {type:'agent-math';source:string;id:number} | { type: 'commit-source'; expected: string; desired: string; id: number }>) => {
  const data = event.data;
  if (data.type === 'init') {
    initialize = (async () => {
      await init(); kernel = new Kernel(data.config);
      self.postMessage({ type: 'ready' });
    })().catch(() => { self.postMessage({ type: 'error', message: 'Kernel initialization failed' }); });
  } else {
    void (async () => {
      await initialize;
      try {
        if (!kernel) throw new Error('not initialized');
        self.postMessage(data.type === 'commit-source'
          ? { response: { id: data.id, body: JSON.parse(kernel.commit_source_file(data.expected, data.desired)) as unknown }, events: [] }
          : data.type==='agent-math'?{response:{id:data.id,body:{type:'agent_math',value:JSON.parse(kernel.agent_math(data.source)) as unknown}},events:[]}
          : JSON.parse(kernel.request(data.envelope)) as unknown);
      } catch (failure) {
        self.postMessage(data.type === 'commit-source'||data.type==='agent-math'
          ? { response: { id: data.id, body: { type: 'error', message: typeof failure === 'string' ? failure : 'Kernel source commit failed' } }, events: [] }
          : { type: 'error', id: data.id, message: 'Kernel request failed' });
      }
    })();
  }
};
