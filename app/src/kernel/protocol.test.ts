import { describe, expect, it } from 'vitest';
import type { Envelope } from './generated/Envelope';
import type { Event } from './generated/Event';
import type { Request } from './generated/Request';
import type { Response } from './generated/Response';
import type { Verification } from './generated/Verification';

describe('generated kernel wire contract', () => {
  it('carries numeric envelope IDs and the prescribed request tags', () => {
    const requests = [
      { type: 'evaluate', cell_id: 'α', source: 'solve(x^2=2,x)', dialect: 'Modern' },
      { type: 'llm_http_end', request_id: 'r1', status: 0, error: 'network failure' },
      { type: 'llm_chat', request_id: 'r2', messages: [{
        role: 'assistant', content: '', tool_call_id: null,
        tool_calls: [{ id: 'call_1', name: 'evaluate', arguments: '{"code":"1+1"}' }],
      }] },
    ] satisfies Request[];
    const envelope: Envelope<Request> = { id: 42, body: requests[0]! };
    expect(JSON.parse(JSON.stringify(envelope))).toEqual(envelope);
    expect(typeof envelope.id).toBe('number');
  });

  it('narrows the flattened preview response and externally tagged verification', () => {
    const envelope: Envelope<Response> = {
      id: 42,
      body: { type: 'preview', latex: 'x^2', diagnostics: [], dialect: 'Modern',
        tokens: [[{ start: 0, end: 1 }, 'Identifier']], actions: [] },
    };
    const reply: Response = JSON.parse(JSON.stringify(envelope.body)) as Response;
    if (reply.type === 'preview') {
      expect(reply.latex).toBe('x^2');
      expect(reply.tokens[0]?.[0].end).toBe(1);
    } else {
      throw new Error('fixture must be a preview');
    }
    const verified: Verification = { Numeric: { digits: 200 } };
    expect(JSON.stringify(verified)).toBe('{"Numeric":{"digits":200}}');
    const event = { id: 0, body: { type: 'cell_status', cell_id: 'b', status: 'Stale' } } satisfies Envelope<Event>;
    expect(event.id).toBe(0);
  });

  it('rejects wrong field casing and incompatible JSON types at compile time', () => {
    // @ts-expect-error request dialects retain the parser's capitalized spelling.
    const wrongDialect: Request = { type: 'evaluate', cell_id: 'a', source: '1', dialect: 'modern' };
    // @ts-expect-error snake_case is the protocol discriminant contract.
    const wrongTag: Request = { type: 'runAll' };
    // @ts-expect-error JSON transport uses numbers, not bigint.
    const wrongEnvelope: Envelope<Event> = { id: 0n, body: { type: 'llm_done', request_id: 'r' } };
    expect([wrongDialect, wrongTag, wrongEnvelope]).toHaveLength(3);
  });
});
