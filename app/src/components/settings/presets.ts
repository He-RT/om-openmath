import type { ProfileConfig } from "../../kernel/generated/ProfileConfig";
import type { LlmConfig } from "../../kernel/generated/LlmConfig";
export interface Preset {
  id: string;
  label: string;
  kind: ProfileConfig["kind"];
  base_url: string;
  model: string;
  api_key_env: string | null;
  tools: boolean;
  json: boolean;
  extra_body?: Record<string, unknown>;
}
/** Verified examples only; model names remain user-editable configuration data. */
export const presets: Preset[] = [
  {
    id: "openai",
    label: "OpenAI",
    kind: "openai_chat",
    base_url: "https://api.openai.com/v1",
    model: "gpt-4.1-mini",
    api_key_env: "OPENAI_API_KEY",
    tools: true,
    json: true,
  },
  {
    id: "deepseek",
    label: "DeepSeek",
    kind: "openai_chat",
    base_url: "https://api.deepseek.com/v1",
    model: "deepseek-flash",
    api_key_env: "DEEPSEEK_API_KEY",
    tools: true,
    json: true,
    extra_body: { thinking: { type: "disabled" } },
  },
  {
    id: "deepseek-fim",
    label: "DeepSeek FIM",
    kind: "openai_fim",
    base_url: "https://api.deepseek.com/beta",
    model: "deepseek-flash",
    api_key_env: "DEEPSEEK_API_KEY",
    tools: false,
    json: false,
  },
  {
    id: "qwen",
    label: "Qwen",
    kind: "openai_chat",
    base_url: "https://dashscope.aliyuncs.com/compatible-mode/v1",
    model: "qwen-plus",
    api_key_env: "DASHSCOPE_API_KEY",
    tools: true,
    json: true,
  },
  {
    id: "openrouter",
    label: "OpenRouter",
    kind: "openai_chat",
    base_url: "https://openrouter.ai/api/v1",
    model: "openai/gpt-4.1-mini",
    api_key_env: "OPENROUTER_API_KEY",
    tools: true,
    json: true,
  },
  {
    id: "ollama",
    label: "Ollama",
    kind: "openai_chat",
    base_url: "http://localhost:11434/v1",
    model: "qwen2.5-coder:7b",
    api_key_env: null,
    tools: true,
    json: true,
  },
  {
    id: "ollama-fim",
    label: "Ollama FIM",
    kind: "ollama_fim",
    base_url: "http://localhost:11434",
    model: "qwen2.5-coder:7b",
    api_key_env: null,
    tools: false,
    json: false,
  },
  {
    id: "lmstudio",
    label: "LM Studio",
    kind: "openai_chat",
    base_url: "http://localhost:1234/v1",
    model: "local-model",
    api_key_env: null,
    tools: true,
    json: true,
  },
  {
    id: "anthropic",
    label: "Anthropic",
    kind: "anthropic",
    base_url: "https://api.anthropic.com",
    model: "claude-sonnet-5-5",
    api_key_env: "ANTHROPIC_API_KEY",
    tools: true,
    json: false,
  },
  {
    id: "custom",
    label: "OpenAI compatible / MiMo",
    kind: "openai_chat",
    base_url: "",
    model: "",
    api_key_env: null,
    tools: true,
    json: true,
  },
];
export function newProfile(name = "model"): ProfileConfig {
  return {
    name,
    kind: "openai_chat",
    base_url: "",
    model: "",
    requires_api_key: true,
    api_key: null,
    api_key_env: null,
    temperature: 0.2,
    max_tokens: 1024,
    supports_tools: true,
    supports_json_mode: true,
    timeout_ms: 60000,
    extra_headers: {},
  };
}
export function applyPreset(
  profile: ProfileConfig,
  preset: Preset,
): ProfileConfig {
  return {
    ...profile,
    kind: preset.kind,
    requires_api_key: !["ollama", "ollama-fim", "lmstudio"].includes(preset.id),
    base_url: preset.base_url,
    model: preset.model,
    api_key: null,
    api_key_env: preset.api_key_env,
    extra_headers: {},
    extra_body: preset.extra_body ?? {},
    supports_tools: preset.tools,
    supports_json_mode: preset.json,
    max_tokens: preset.kind.endsWith("fim") ? 64 : 1024,
  };
}
export const features = [
  "translate",
  "explain",
  "chat",
  "complete",
  "fix",
] as const;
export function uniqueName(base: string, profiles: ProfileConfig[]) {
  let name = base,
    index = 2;
  while (profiles.some((p) => p.name === name)) name = `${base}-${index++}`;
  return name;
}
export function cloneProfile(
  profile: ProfileConfig,
  profiles: ProfileConfig[],
): ProfileConfig {
  return {
    ...structuredClone(profile),
    name: uniqueName(`${profile.name}-copy`, profiles),
    api_key: profile.api_key === "***" ? null : profile.api_key,
  };
}
export function renameProfile(
  llm: LlmConfig,
  old: string,
  name: string,
): LlmConfig {
  const next = {
    ...llm,
    profiles: llm.profiles.map((p) =>
      p.name === old
        ? { ...p, name, api_key: p.api_key === "***" ? null : p.api_key }
        : p,
    ),
  };
  for (const f of features) if (next[f] === old) next[f] = name;
  return next;
}
export function removeProfile(llm: LlmConfig, name: string): LlmConfig {
  const next = {
    ...llm,
    profiles: llm.profiles.filter((p) => p.name !== name),
  };
  for (const f of features) if (next[f] === name) next[f] = "";
  return next;
}
export function validationError(llm: LlmConfig): string | null {
  const names = new Set<string>();
  for (const p of llm.profiles) {
    if (!p.name.trim() || names.has(p.name))
      return "Profile names must be nonempty and unique";
    names.add(p.name);
    try {
      const url = new URL(p.base_url);
      if (
        !["http:", "https:"].includes(url.protocol) ||
        url.username ||
        url.password ||
        url.search ||
        url.hash
      )
        return "Invalid Base URL";
    } catch {
      return "Invalid Base URL";
    }
    if (
      !p.model.trim() ||
      !Number.isFinite(p.temperature) ||
      p.temperature < 0 ||
      p.temperature > 2 ||
      !Number.isInteger(p.max_tokens) ||
      p.max_tokens < 1 ||
      p.max_tokens > 4294967295 ||
      !Number.isInteger(p.timeout_ms) ||
      p.timeout_ms < 1
    )
      return "Invalid model or numeric settings";
  }
  for (const f of features) {
    if (!llm[f]) continue;
    const p = llm.profiles.find((p) => p.name === llm[f]);
    if (!p) return "Unknown feature profile";
    if (
      (f !== "complete" && !["openai_chat", "anthropic"].includes(p.kind)) ||
      (f === "chat" && !p.supports_tools)
    )
      return "Incompatible feature profile";
  }
  return null;
}
