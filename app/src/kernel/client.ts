import type { Request } from './generated/Request';
import type { Response } from './generated/Response';
import type { Event } from './generated/Event';
export type { Request, Response, Event };
export type Unsubscribe = () => void;
export interface KernelClient {
  readonly kind: 'wasm' | 'tauri';
  readonly ready: Promise<void>;
  request(request: Request): Promise<Response>;
  onEvent(callback: (event: Event) => void): Unsubscribe;
  interrupt(): Promise<void>;
  dispose(): void;
}
export class EventHub {
  private listeners = new Set<(event: Event) => void>();
  onEvent(callback: (event: Event) => void): Unsubscribe {
    this.listeners.add(callback);
    return () => this.listeners.delete(callback);
  }
  emit(event: Event) {
    for (const callback of [...this.listeners]) {
      try { callback(event); } catch { /* One view must not interrupt delivery to other views. */ }
    }
  }
  clear() { this.listeners.clear(); }
}
