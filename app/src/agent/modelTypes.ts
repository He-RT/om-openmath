import type { AgentCanonicalMessage } from '../../../agent/src/generated/protocol';
export type { AgentCanonicalMessage };
export interface ModelBinding {
  provider_ref: string; route_ref: string; model_ref: string; adapter_version: string;
  protocol: 'chat_completions' | 'anthropic_messages';
}
export interface ProviderReplay {
  binding: ModelBinding; response_id: string; provider_model: string; assistant: unknown;
}
export interface CanonicalRecord { canonical: AgentCanonicalMessage; replay: ProviderReplay | null }
export interface ModelInput {
  binding: ModelBinding; api_model: string; messages: CanonicalRecord[];
  results: { message_id: string; tool_call_id: string; text: string; is_error: boolean }[];
  images?: { message_id: string; part_id: string; media_ref: string; media_type: string; data: string }[];
  tools: { name: string; description: string; parameters: Record<string, unknown> }[];
  max_output_tokens: number; temperature: number | null; stream: boolean;
}
export interface ModelResult {
  message: AgentCanonicalMessage; replay: ProviderReplay | null;
  finish: 'complete' | 'tool_calls' | 'incomplete'; provider_reason: string;
  usage: { input: number | null; output: number | null; total: number | null; cache_read: number | null; cache_write: number | null };
  partial_calls: { index: number; id: string; name: string; arguments: string }[];
}
/** A host-prepared immutable round. URL and authentication stay outside canonical history/codec input. */
export interface PreparedModelRound {
  requestId: string; messageId: string; taskId: string; turnId: string;
  input: ModelInput; url: string; headers: [string, string][]; timeoutMs: number;
}
