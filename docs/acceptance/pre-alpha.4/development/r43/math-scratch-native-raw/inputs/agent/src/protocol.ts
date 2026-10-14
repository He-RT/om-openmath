// Private pipe framing only. No Pi state, HTTP credentials or tool executor lives here.
import { readFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import type { AgentIPCFrame } from './generated/protocol.ts';
export type { AgentIPCFrame, AgentCanonicalMessage, AgentMessagePart } from './generated/protocol.ts';
type Json = null | boolean | number | string | Json[] | { [key: string]: Json };
export const MAX_FRAME_BYTES = 1024 * 1024;
export const MAX_JSON_DEPTH = 64;
export const MAX_JSON_NODES = 32768;
const schemaBytes = readFileSync(new URL('../../docs/design/native-agent.schema.json', import.meta.url));
const schema = JSON.parse(schemaBytes.toString('utf8'));
const toolSchema = JSON.parse(readFileSync(new URL('../../docs/design/agent-tools.schema.json', import.meta.url),'utf8'));
export const schemaHash = createHash('sha256').update(schemaBytes).digest('hex');
function fail(code: string): never { throw new Error(code); }
function unicode(text: string): boolean {
  for (let i = 0; i < text.length; i++) {
    const n = text.charCodeAt(i);
    if (n >= 0xd800 && n <= 0xdbff) {
      const next = text.charCodeAt(++i);
      if (!(next >= 0xdc00 && next <= 0xdfff)) return false;
    } else if (n >= 0xdc00 && n <= 0xdfff) return false;
  }
  return true;
}
/** Complete UTF8 JSON, with unique member names and independent syntax bounds. */
export function strictJson(bytes: Uint8Array): Json {
  if (!bytes.length || bytes.length > MAX_FRAME_BYTES) fail('IPC_FRAME_SIZE');
  let text: string;
  try { text = new TextDecoder('utf-8', { fatal: true, ignoreBOM: true }).decode(bytes); }
  catch { return fail('IPC_INVALID_UTF8'); }
  let cursor = 0, nodes = 0;
  const space = () => { while (cursor < text.length && /[ \t\r\n]/.test(text[cursor])) cursor++; };
  const string = (): string => {
    const start = cursor++;
    while (cursor < text.length) {
      const c = text[cursor++];
      if (c === '\\') { cursor++; continue; }
      if (c === '"') {
        let value: string;
        try { value = JSON.parse(text.slice(start, cursor)); } catch { return fail('IPC_INVALID_JSON'); }
        if (!unicode(value)) fail('IPC_INVALID_JSON');
        return value;
      }
      if (c.charCodeAt(0) < 32) fail('IPC_INVALID_JSON');
    }
    return fail('IPC_INVALID_JSON');
  };
  const value = (depth: number): Json => {
    if (depth > MAX_JSON_DEPTH || ++nodes > MAX_JSON_NODES) fail('IPC_INVALID_JSON');
    space();
    if (text[cursor] === '"') return string();
    if (text[cursor] === '{') {
      cursor++; space(); const output: { [key: string]: Json } = Object.create(null);
      if (text[cursor] === '}') { cursor++; return output; }
      while (cursor < text.length) {
        if (text[cursor] !== '"') fail('IPC_INVALID_JSON');
        const key = string(); if (Object.hasOwn(output, key)) fail('IPC_INVALID_JSON');
        space(); if (text[cursor++] !== ':') fail('IPC_INVALID_JSON');
        output[key] = value(depth + 1); space();
        const delimiter = text[cursor++]; if (delimiter === '}') return output;
        if (delimiter !== ',') fail('IPC_INVALID_JSON'); space();
      }
      return fail('IPC_INVALID_JSON');
    }
    if (text[cursor] === '[') {
      cursor++; space(); const output: Json[] = [];
      if (text[cursor] === ']') { cursor++; return output; }
      while (cursor < text.length) {
        output.push(value(depth + 1)); space(); const delimiter = text[cursor++];
        if (delimiter === ']') return output; if (delimiter !== ',') fail('IPC_INVALID_JSON');
      }
      return fail('IPC_INVALID_JSON');
    }
    for (const [literal, result] of [['null', null], ['true', true], ['false', false]] as const) {
      if (text.startsWith(literal, cursor)) { cursor += literal.length; return result; }
    }
    const numeric = /^-?(?:0|[1-9][0-9]*)(?:\.[0-9]+)?(?:[eE][+-]?[0-9]+)?/.exec(text.slice(cursor));
    if (!numeric) return fail('IPC_INVALID_JSON');
    cursor += numeric[0].length; const number = Number(numeric[0]);
    if (!Number.isFinite(number)) fail('IPC_INVALID_JSON'); return number;
  };
  const result = value(0); space(); if (cursor !== text.length) fail('IPC_INVALID_JSON'); return result;
}
function matches(value: Json, rule: any, depth = 0, root = schema): boolean {
  if (depth > 128) return false;
  if (rule.$ref) {
    if (!rule.$ref.startsWith('#/$defs/')) return false;
    const target = root.$defs[rule.$ref.slice(8)]; return target !== undefined && matches(value, target, depth + 1,root);
  }
  if (Object.hasOwn(rule, 'const') && value !== rule.const) return false;
  if (rule.enum && !rule.enum.includes(value)) return false;
  if (rule.oneOf && rule.oneOf.filter((r: any) => matches(value, r, depth + 1,root)).length !== 1) return false;
  if (rule.anyOf && !rule.anyOf.some((r: any) => matches(value, r, depth + 1,root))) return false;
  if (rule.allOf && !rule.allOf.every((r:any)=>matches(value,r,depth+1,root))) return false;
  if (rule.not && matches(value,rule.not,depth+1,root)) return false;
  if (rule.if) {const branch=matches(value,rule.if,depth+1,root)?rule.then:rule.else;if(branch&&!matches(value,branch,depth+1,root))return false;}
  if (rule.type) {
    const type = value === null ? 'null' : Array.isArray(value) ? 'array' : typeof value;
    const permitted=(candidate:string)=>candidate==='integer'?typeof value==='number'&&Number.isSafeInteger(value):type===candidate;
    if(Array.isArray(rule.type)?!rule.type.some(permitted):!permitted(rule.type))return false;
  }
  if (typeof value === 'string') {
    const count = [...value].length;
    if (!unicode(value) || rule.minLength !== undefined && count < rule.minLength || rule.maxLength !== undefined && count > rule.maxLength) return false;
    if (rule.pattern && !new RegExp(rule.pattern, 'u').test(value)) return false;
  }
  if (typeof value === 'number' && (!Number.isFinite(value) || rule.minimum !== undefined && value < rule.minimum || rule.maximum !== undefined && value > rule.maximum)) return false;
  if (Array.isArray(value)) {
    if (rule.minItems !== undefined && value.length < rule.minItems || rule.maxItems !== undefined && value.length > rule.maxItems) return false;
    if (rule.items && !value.every(v => matches(v, rule.items, depth + 1,root))) return false;
  } else if (value !== null && typeof value === 'object') {
    if (rule.required?.some((key: string) => !Object.hasOwn(value, key))) return false;
    for (const [key, entry] of Object.entries(value)) {
      if (rule.properties && Object.hasOwn(rule.properties, key)) { if (!matches(entry, rule.properties[key], depth + 1,root)) return false; }
      else if (rule.additionalProperties === false) return false;
    }
  }
  return true;
}
/** Data only, after native durability; no result may become a tool grant or source command. */
export function decodeReadonlyResult(name:string,bytes:Uint8Array):{ok:boolean,status:'completed'|'failed'} {
  if(name==='apply_notebook_patch')fail('IPC_TOOL_RESULT_LIMIT');
  return decodeToolResult(name,bytes) as {ok:boolean,status:'completed'|'failed'};
}
/** All native tool data stays closed; effects additionally carry a real operation identity. */
export function decodeToolResult(name:string,bytes:Uint8Array):{ok:boolean,status:string,operation_ref:string|null} {
  if(!['read_notebook','search_functions','get_function_docs','preview_source','apply_notebook_patch','inspect_result','evaluate_scratch'].includes(name)||bytes.length>262144)fail('IPC_TOOL_RESULT_LIMIT');
  const value=strictJson(bytes),rule=toolSchema.$defs[name+'_result'];
  const phases=name==='apply_notebook_patch'?['completed','failed','cancelled','unknown']:['completed','failed'];
  if(!matches(value,rule,0,toolSchema)||value===null||typeof value!=='object'||Array.isArray(value)||!phases.includes(String(value.status))||(name!=='apply_notebook_patch'&&value.operation_ref!==null))fail('IPC_INVALID_TOOL_RESULT');
  return value as unknown as {ok:boolean,status:string,operation_ref:string|null};
}
export function decodeFrame(bytes: Uint8Array): AgentIPCFrame {
  const value = strictJson(bytes);
  if (!matches(value, schema.$defs.AgentIPCFrame)) fail('IPC_INVALID_SHAPE');
  return value as AgentIPCFrame;
}
/** Native model codec supplies canonical data, not Pi state or executable provider payloads. */
export function decodeCanonical(bytes:Uint8Array): import('./generated/protocol.ts').AgentCanonicalMessage {
  const value=strictJson(bytes);
  if (!matches(value,schema.$defs.AgentCanonicalMessage)) fail('IPC_INVALID_CANONICAL');
  return value as unknown as import('./generated/protocol.ts').AgentCanonicalMessage;
}
export function encodeFrame(value: AgentIPCFrame): Uint8Array {
  if (!matches(value as Json, schema.$defs.AgentIPCFrame)) fail('IPC_INVALID_SHAPE');
  const bytes = new TextEncoder().encode(JSON.stringify(value)); decodeFrame(bytes);
  if (bytes.length > MAX_FRAME_BYTES) fail('IPC_FRAME_SIZE');
  const line = new Uint8Array(bytes.length + 1); line.set(bytes); line[bytes.length] = 10; return line;
}
/** Private stream owner, never a second message/task authority. */
export class FrameDecoder {
  #buffer = new Uint8Array(16384);
  #count = 0;
  #failed = false;
  push(chunk: Uint8Array): AgentIPCFrame[] {return this.pushRaw(chunk).map(v=>v.frame);}
  pushRaw(chunk:Uint8Array): {raw:Uint8Array;frame:AgentIPCFrame}[] {
    if (this.#failed) fail('IPC_CONNECTION_FAILED');
    const frames: {raw:Uint8Array;frame:AgentIPCFrame}[] = [];
    try {
      for (const byte of chunk) {
        if (byte === 10) {
          if (this.#count && this.#buffer[this.#count - 1] === 13) this.#count--;
          const raw=this.#buffer.slice(0,this.#count);frames.push({raw,frame:decodeFrame(raw)}); this.#count = 0;
          if (frames.length > 32) fail('IPC_BATCH_SIZE');
        } else {
          if (this.#count >= MAX_FRAME_BYTES) fail('IPC_FRAME_SIZE');
          if (this.#count === this.#buffer.length) {
            const larger = new Uint8Array(Math.min(MAX_FRAME_BYTES, this.#count * 2)); larger.set(this.#buffer); this.#buffer = larger;
          }
          this.#buffer[this.#count++] = byte;
        }
      }
      return frames;
    } catch (error) { this.#failed = true; this.#count = 0; throw error; }
  }
  finish(): void {
    if (this.#failed) fail('IPC_CONNECTION_FAILED');
    if (this.#count) { this.#failed = true; this.#count = 0; fail('IPC_INCOMPLETE_EOF'); }
  }
}
