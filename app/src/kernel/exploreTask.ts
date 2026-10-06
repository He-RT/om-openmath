import type { Request, Response } from './client';
export interface ExploreTaskPort {
  postMessage(message: unknown): void;
  terminate(): void;
  onmessage: ((event: MessageEvent<{response: Response}>) => void) | null;
  onerror: ((event: ErrorEvent) => void) | null;
}
/** A disposable mathematical worker. Cancelling one task never restarts the notebook kernel. */
export class ExploreTask {
  private active: { worker: ExploreTaskPort; reject: (error: Error) => void; timer: ReturnType<typeof setTimeout> } | null = null;
  private closed = false;
  private suspended = false;
  constructor(private factory: () => ExploreTaskPort = () => new Worker(new URL('./exploreWorker.ts', import.meta.url), {type:'module'})) {}
  request(request: Request): Promise<Response> {
    this.cancel();
    if(this.closed)return Promise.reject(new Error('Exploration disposed'));
    if(this.suspended)return Promise.reject(new Error('Exploration suspended'));
    const worker=this.factory();
    return new Promise((resolve,reject)=>{
      const timer=setTimeout(()=>{ if(this.active?.worker===worker)this.cancel('Exploration timed out'); },10000);
      this.active={worker,reject,timer};
      worker.onmessage=(event)=>{
        if(this.active?.worker!==worker)return;
        this.active=null;clearTimeout(timer);worker.terminate();resolve(event.data.response);
      };
      worker.onerror=()=>{if(this.active?.worker===worker)this.cancel('Exploration worker failed');};
      try {worker.postMessage({request});}catch {this.cancel('Exploration could not be started');}
    });
  }
  cancel(message='Exploration cancelled') {
    const job=this.active;this.active=null;
    if(job){clearTimeout(job.timer);job.worker.terminate();job.reject(new Error(message));}
  }
  suspend(){this.suspended=true;this.cancel();}
  resume(){this.suspended=false;}
  dispose(){this.closed=true;this.cancel();}
}
