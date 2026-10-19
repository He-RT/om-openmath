import { createAssistantMessageEventStream, type AssistantMessage, type JsonObject } from '@earendil-works/pi-ai';
import type { StreamFn } from '@earendil-works/pi-agent-core';
import { ModelCodec } from '../kernel/wasm/om_wasm.js';
import { LlmDriver } from '../kernel/llmDriver';
import type { ModelResult, PreparedModelRound } from './modelTypes';

// Pi requires numeric usage fields internally. Product usage comes only from ModelResult,
// where missing provider values stay null; these scheduling placeholders are never facts.
const internalUsage = () => ({ input: 0, output: 0, cacheRead: 0, cacheWrite: 0, totalTokens: 0,
  cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0, total: 0 } });

/** Pi schedules turns; the existing HTTP driver and shared Rust codec own provider bytes. */
export class BrowserModelAdapter {
  private prepared: PreparedModelRound | undefined;
  private results = new WeakMap<AssistantMessage, ModelResult>();
  constructor(private onResult: (result: ModelResult) => void, private fetcher?: typeof fetch,private onDispatch?: (round:PreparedModelRound)=>void,private onText?: (round:PreparedModelRound,text:string)=>void) {}
  prepare(round: PreparedModelRound) {
    if (this.prepared) throw new Error('Previous model round was not dispatched');
    this.prepared = structuredClone(round);
  }
  result(message: AssistantMessage): ModelResult {
    const result = this.results.get(message);
    if (!result) throw new Error('No canonical model result');
    return result;
  }
  readonly stream: StreamFn = (model, _context, options) => {
    const stream = createAssistantMessageEventStream(), round = this.prepared;
    this.prepared = undefined;
    const partial: AssistantMessage = { role: 'assistant', api: model.api, provider: model.provider,
      model: model.id, content: [], usage: internalUsage(), stopReason: 'stop', timestamp: Date.now() };
    void (async () => {
      let codec: ModelCodec | undefined, driver: LlmDriver | undefined;
      const signal = options?.signal;
      let abort: (() => void) | undefined;
      try {
        if (!round) throw new Error('Model round has not been prepared');
        signal?.throwIfAborted();
        codec = new ModelCodec(JSON.stringify(round.input), round.messageId, round.taskId, round.turnId);
        const decoder = codec;
        let status: number | undefined, text = '';
        const result = await new Promise<ModelResult>((resolve, reject) => {
          driver = new LlmDriver(async request => {
            if ((request.type !== 'llm_http_chunk' && request.type !== 'llm_http_end') || request.request_id !== round.requestId) return { type: 'ok' };
            try {
              signal?.throwIfAborted();
              if (request.type === 'llm_http_end' && request.error) throw new Error(request.error);
              if (status === undefined) { status = request.status ?? 0; decoder.status(status); }
              if (request.type === 'llm_http_chunk') {
                const delta = decoder.chunk(request.chunk);
                if (delta) {
                  if (!text) { partial.content = [{ type: 'text', text: '' }]; stream.push({ type: 'start', partial }); stream.push({ type: 'text_start', contentIndex: 0, partial }); }
                  text += delta; partial.content = [{ type: 'text', text }];
                  stream.push({ type: 'text_delta', contentIndex: 0, delta, partial });
                  this.onText?.(round,delta);
                }
              } else {
                resolve(JSON.parse(decoder.finish()) as ModelResult);
              }
            } catch (error) { driver?.cancel(round.requestId); reject(error); }
            return { type: 'ok' };
          }, this.fetcher);
          abort = () => { decoder.cancel(); driver?.cancel(round.requestId); reject(new DOMException('Model request stopped', 'AbortError')); };
          signal?.addEventListener('abort', abort, { once: true });
          this.onDispatch?.(round);
          driver.start(round.requestId, { method: 'POST', url: round.url, headers: round.headers, body: decoder.body(), stream: round.input.stream }, round.timeoutMs);
        });
        signal?.throwIfAborted();
        this.onResult(result);
        if (result.finish === 'incomplete') throw new Error('Incomplete model response');
        const content: AssistantMessage['content'] = result.message.parts.map(part => {
          if (part.kind === 'text') return { type: 'text', text: part.text };
          if (part.kind === 'tool_call') return { type: 'toolCall', id: part.tool_call_id, name: part.name, arguments: JSON.parse(part.arguments_json) as JsonObject };
          throw new Error('Unsupported model content');
        });
        const message: AssistantMessage = { ...partial, content, stopReason: result.finish === 'tool_calls' ? 'toolUse' : 'stop' };
        this.results.set(message, result);
        stream.push({ type: 'done', reason: message.stopReason as 'toolUse' | 'stop', message });
      } catch (error) {
        const reason = signal?.aborted ? 'aborted' : 'error';
        const message: AssistantMessage = { ...partial, stopReason: reason, errorMessage: error instanceof Error ? error.message : String(error) };
        stream.push({ type: 'error', reason, error: message });
      } finally {
        if (abort) signal?.removeEventListener('abort', abort);
        driver?.dispose(); codec?.free();
      }
    })();
    return stream;
  };
}
