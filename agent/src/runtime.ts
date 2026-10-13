/** Pi is a replaceable loop. Every persistent business record crosses the native canonical port. */
import { Agent, type AgentTool, type StreamFn, type BeforeToolCallContext, type AfterToolCallContext } from '@earendil-works/pi-agent-core';
import type { Api, Model, AssistantMessage } from '@earendil-works/pi-ai';
import type { AgentCanonicalMessage } from './generated/protocol.ts';

export interface AgentRuntimePort {
  /** Supplied by the native adapter; it must never select a stock provider transport. */
  stream: StreamFn;
  /** Preserve original provider-decoded canonical bytes and IDs; no Pi state persistence. */
  canonical(message: AssistantMessage): AgentCanonicalMessage;
  persist(message: AgentCanonicalMessage, signal?: AbortSignal): Promise<void>;
  prepare(signal?: AbortSignal): Promise<void>;
  beforeTool(context: BeforeToolCallContext, signal?: AbortSignal): Promise<void>;
  afterTool(context: AfterToolCallContext, signal?: AbortSignal): Promise<void>;
}
const toolsAllowed = new Set(['read_notebook','search_functions','get_function_docs','preview_source','apply_notebook_patch','run_cells','inspect_result','evaluate_scratch','read_attachment','prepare_attachment','get_operation_status','undo_transaction']);

/** Internal native-adapter construction only. No plugin loader or user-provided JS is exposed. */
export class AgentRuntime {
  readonly #agent: Agent;
  readonly #task: string;
  readonly #port: AgentRuntimePort;
  readonly #receipts = new WeakMap<AssistantMessage, Promise<void>>();
  #failed = false;
  constructor(task: string, model: Model<Api>, tools: AgentTool[], port: AgentRuntimePort) {
    if (!port.stream || tools.length > 12 || tools.some(t => !toolsAllowed.has(t.name)) || new Set(tools.map(t=>t.name)).size !== tools.length) throw new Error('INVALID_AGENT_ADAPTER');
    this.#task = task; this.#port = port;
    this.#agent = new Agent({
      initialState: {model, tools}, sessionId: task, toolExecution: 'sequential',
      steeringMode: 'one-at-a-time', followUpMode: 'one-at-a-time',
      streamFn: port.stream, getApiKey: () => undefined, maxRetryDelayMs: 0,
      prepareRequest: async (_request, signal) => {
        if (this.#failed) throw new Error('CANONICAL_ACK_FAILED');
        signal?.throwIfAborted(); await port.prepare(signal); signal?.throwIfAborted();
      },
      beforeToolCall: async (context, signal) => {
        await this.#persist(context.assistantMessage, signal);
        signal?.throwIfAborted(); await port.beforeTool(context, signal); signal?.throwIfAborted();
      },
      afterToolCall: async (context, signal) => {await port.afterTool(context, signal);},
      finishTurn: async (context, signal) => {await this.#persist(context.message, signal);},
    });
  }
  async #persist(message: AssistantMessage, signal?: AbortSignal): Promise<void> {
    let pending = this.#receipts.get(message);
    if (!pending) {
      const canonical = this.#port.canonical(message);
      if (canonical.role !== 'assistant' || canonical.task_id !== this.#task) throw new Error('STALE_CANONICAL_SCOPE');
      pending = this.#port.persist(canonical, signal).catch(error => {this.#failed = true;this.#agent.abort();throw error;});
      this.#receipts.set(message, pending);
    }
    await pending; signal?.throwIfAborted();
  }
  #input(message: AgentCanonicalMessage) {
    if (message.role !== 'user' || message.task_id !== this.#task || message.parts.some(p=>p.kind!=='text')) throw new Error('UNSUPPORTED_AGENT_INPUT');
    return {role:'user' as const,content:message.parts.map(p=>({type:'text' as const,text:p.kind==='text'?p.text:''})),timestamp:Date.now()};
  }
  prompt(message: AgentCanonicalMessage): Promise<void> {return this.#agent.prompt(this.#input(message));}
  steer(message: AgentCanonicalMessage): void {this.#agent.steer(this.#input(message));}
  followUp(message: AgentCanonicalMessage): void {this.#agent.followUp(this.#input(message));}
  abort(): void {this.#agent.abort();}
  idle(): Promise<void> {return this.#agent.waitForIdle();}
}
