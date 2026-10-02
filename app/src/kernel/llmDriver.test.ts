import { expect, test, vi } from 'vitest';
import { LlmDriver } from './llmDriver';
import type { Request } from './client';
import type { HttpRequest } from './generated/HttpRequest';
const http: HttpRequest = { method: 'POST', url: 'https://fixture.invalid/profile', headers: [['Authorization', 'Bearer private-key']], body: '{}', stream: true };
test('actual UTF8 bytes, status and event-driven follow-up produce one end for each round', async () => {
  const requests: Request[] = []; let rounds = 0;
  const fetcher: typeof fetch = vi.fn(async (_url, options) => {
    expect(options?.redirect).toBe('error'); rounds++;
    const bytes = new TextEncoder().encode(rounds === 1 ? 'α中' : 'second');
    return new Response(new ReadableStream({ start(controller) { for (const b of bytes) controller.enqueue(new Uint8Array([b])); controller.close(); } }), { status: 201 });
  });
  const driver = new LlmDriver(async request => {
    requests.push(request);
    if (request.type === 'llm_http_end') { if (rounds === 1) driver.event({ type: 'llm_http', request_id: 'job', http }); else driver.event({ type: 'llm_done', request_id: 'job' }); }
    return { type: 'ok' };
  }, fetcher);
  driver.start('job', http, 1000);
  await vi.waitFor(() => expect(requests.filter(r => r.type === 'llm_http_end')).toHaveLength(2));
  expect(requests.filter(r => r.type === 'llm_http_chunk').map(r => r.chunk).join('')).toBe('α中second');
  expect(requests.filter(r => r.type === 'llm_http_chunk').every(r => r.status === 201)).toBe(true);
  driver.dispose();
});
test('transport exceptions do not expose URLs, credentials or private exception messages', async () => {
  const requests: Request[] = [];
  const driver = new LlmDriver(async r => { requests.push(r); return { type: 'ok' }; }, vi.fn(async () => { throw new Error('private-key reflected https://private-url.invalid'); }));
  driver.start('job', http, 1000);
  await vi.waitFor(() => expect(requests).toHaveLength(1));
  expect(JSON.stringify(requests)).not.toContain('private-key'); expect(JSON.stringify(requests)).not.toContain('private-url');
  driver.dispose();
});
test('explicit cancellation aborts fetch and prevents late end or continuation work', async () => {
  let signal: AbortSignal | undefined; const requests: Request[] = [];
  const fetcher: typeof fetch = vi.fn((_url, options) => new Promise<Response>((_resolve, reject) => { signal = options?.signal ?? undefined; signal?.addEventListener('abort', () => reject(new Error('cancel'))); }));
  const driver = new LlmDriver(async r => { requests.push(r); return { type: 'ok' }; }, fetcher);
  driver.start('job', http, 1000); driver.cancel('job');
  await vi.waitFor(() => expect(signal?.aborted).toBe(true));
  expect(requests).toHaveLength(0);
  driver.dispose();
});
