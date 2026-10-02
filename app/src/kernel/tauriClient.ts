import { Channel, invoke } from '@tauri-apps/api/core';
import { EventHub, type KernelClient, type Request, type Response, type Event } from './client';
import type { Envelope } from './generated/Envelope';
interface NativeChannel { onmessage: (message: string) => void }
export interface NativeTransport { invoke<T>(command: string, args?: Record<string, unknown>): Promise<T>; channel(): NativeChannel }
const native: NativeTransport = { invoke, channel: () => new Channel<string>() };
export class TauriClient implements KernelClient {
  readonly kind = 'tauri' as const;
  readonly ready: Promise<void>;
  private hub = new EventHub(); private nextId = 1; private closed = false;
  private channel: NativeChannel;
  constructor(private transport: NativeTransport = native) {
    this.channel = transport.channel();
    this.channel.onmessage = value => {
      if (this.closed) return;
      try { const event = JSON.parse(value) as Envelope<Event>; if (event.id === 0 && event.body.type !== 'llm_http') this.hub.emit(event.body); } catch { /* Invalid native transport packets are ignored. */ }
    };
    this.ready = transport.invoke<void>('kernel_subscribe', { channel: this.channel });
  }
  onEvent(callback: (event: Event) => void) { return this.hub.onEvent(callback); }
  async request(request: Request): Promise<Response> {
    await this.ready; if (this.closed) throw new Error('Kernel client disposed');
    const id = this.nextId++; if (!Number.isSafeInteger(id)) throw new Error('Kernel request ID exhausted');
    const raw = await this.transport.invoke<string>('kernel_request', { envelope: JSON.stringify({ id, body: request }) });
    if (this.closed) throw new Error('Kernel client disposed');
    const response = JSON.parse(raw) as Envelope<Response>;
    if (response.id !== id) throw new Error('Kernel reply ID does not match');
    return response.body;
  }
  async interrupt() { await this.ready; if (!this.closed) await this.transport.invoke<void>('kernel_interrupt'); }
  dispose() { this.closed = true; this.channel.onmessage = () => {}; this.hub.clear(); }
}
