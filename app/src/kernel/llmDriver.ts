import type { Request, Response, Event } from './client';
import type { HttpRequest } from './generated/HttpRequest';
interface Job { id: string; timeout: number; controller: AbortController; next: HttpRequest | null; stopped: boolean }
/** Drives real bytes through the shared Session, following its LlmHttp events. */
export class LlmDriver {
  private jobs = new Map<string, Job>();
  constructor(private request: (request: Request) => Promise<Response>, private fetcher: typeof fetch = globalThis.fetch.bind(globalThis)) {}
  start(id: string, http: HttpRequest, timeout: number) {
    if (this.jobs.has(id)) return;
    const job: Job = { id, timeout, controller: new AbortController(), next: http, stopped: false };
    this.jobs.set(id, job);
    void this.drive(job);
  }
  event(event: Event) {
    if (event.type === 'llm_http') {
      const job = this.jobs.get(event.request_id);
      if (job && !job.stopped) job.next = event.http;
    } else if (event.type === 'llm_done' || event.type === 'llm_error') this.cancel(event.request_id);
  }
  cancel(id: string) {
    const job = this.jobs.get(id);
    if (job) { job.stopped = true; job.controller.abort(); this.jobs.delete(id); }
  }
  dispose() { for (const id of this.jobs.keys()) this.cancel(id); }
  private async drive(job: Job) {
    while (job.next && !job.stopped) {
      const http = job.next; job.next = null;
      job.controller = new AbortController();
      let timedOut = false; let status = 0; let sentEnd = false;
      const timer = setTimeout(() => { timedOut = true; job.controller.abort(); }, job.timeout);
      const end = async (error: string | null) => {
        if (sentEnd || job.stopped) return;
        sentEnd = true;
        await this.request({ type: 'llm_http_end', request_id: job.id, status, error });
      };
      try {
        const res = await this.fetcher(http.url, { method: http.method, headers: Object.fromEntries(http.headers), body: http.body, signal: job.controller.signal, redirect: 'error' });
        status = res.status;
        if (res.body) {
          const reader = res.body.getReader(); const decoder = new TextDecoder('utf-8', { fatal: true }); let size = 0;
          try {
            while (!job.stopped) {
              const { value, done } = await reader.read(); if (done) break;
              size += value.byteLength;
              if (size > 1_048_576) throw new Error('limit');
              const chunk = decoder.decode(value, { stream: true });
              if (chunk) await this.request({ type: 'llm_http_chunk', request_id: job.id, chunk, status });
            }
            const chunk = decoder.decode();
            if (chunk && !job.stopped) await this.request({ type: 'llm_http_chunk', request_id: job.id, chunk, status });
          } finally { await reader.cancel().catch(() => {}); reader.releaseLock(); }
        }
        await end(null);
      } catch {
        await end(timedOut ? 'HTTP request timed out' : 'HTTP request or UTF-8 response failed; check network, CORS and profile settings').catch(() => {});
      } finally { clearTimeout(timer); }
      if (!job.next) { this.cancel(job.id); break; }
    }
  }
}
