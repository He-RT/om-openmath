/// <reference lib="webworker" />
import init, { Kernel } from './wasm/om_wasm.js';
let kernel: Kernel | undefined;
let initialize: Promise<void> | undefined;
self.onmessage = (event: MessageEvent<{ type: 'init'; config?: string } | { type: 'request'; envelope: string; id: number }>) => {
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
        self.postMessage(JSON.parse(kernel.request(data.envelope)) as unknown);
      } catch { self.postMessage({ type: 'error', id: data.id, message: 'Kernel request failed' }); }
    })();
  }
};
