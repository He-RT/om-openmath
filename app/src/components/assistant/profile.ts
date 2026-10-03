import type { LlmConfig } from "../../kernel/generated/LlmConfig";
import { completionProfile } from "../editor/ghostProfile";
export type AiFeature = "translate" | "chat" | "fix" | "explain" | "complete";
const scopes = new WeakMap<object, number>();
let nextScope = 0;
export function configScope(config: object | null | undefined) {
  if (!config) return 0;
  let id = scopes.get(config);
  if (id === undefined) {
    id = ++nextScope;
    scopes.set(config, id);
  }
  return id;
}
export function featureProfile(
  config: LlmConfig | null | undefined,
  feature: AiFeature,
) {
  if (!config) return null;
  const profile = completionProfile({ ...config, complete: config[feature] });
  return profile &&
    (feature === "complete" ||
      profile.kind === "openai_chat" ||
      profile.kind === "anthropic")
    ? profile
    : null;
}
