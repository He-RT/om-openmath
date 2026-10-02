import { EventHub, type KernelClient, type Request, type Response, type Event } from '../kernel/client';
/** Explicit deterministic test transport; it never claims to execute the CAS. */
export class MockKernel implements KernelClient {
  readonly kind = 'wasm' as const; readonly ready = Promise.resolve(); readonly requests: Request[] = [];
  private hub = new EventHub(); private closed = false;
  constructor(private handle: (request: Request) => Response | Promise<Response> = () => ({ type: 'error', message: 'No mock response configured' })) {}
  async request(request: Request) { if (this.closed) throw new Error('Mock kernel disposed'); this.requests.push(request); return this.handle(request); }
  onEvent(callback: (event: Event) => void) { return this.hub.onEvent(callback); }
  emit(event: Event) { this.hub.emit(event); }
  async interrupt() { await this.request({ type: 'interrupt' }); }
  dispose() { this.closed = true; this.hub.clear(); }
}
