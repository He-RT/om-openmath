import type { KernelClient } from './client';
export type { KernelClient, Request, Response, Event, Unsubscribe } from './client';
export async function createKernelClient(): Promise<KernelClient> {
  const client = '__TAURI_INTERNALS__' in window
    ? new (await import('./tauriClient')).TauriClient()
    : new (await import('./wasmClient')).WasmClient();
  try { await client.ready; } catch (error) { client.dispose(); throw error; }
  return client;
}
