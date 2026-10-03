import type { KernelConfig } from "../kernel/generated/KernelConfig";
import type { ProfileConfig } from "../kernel/generated/ProfileConfig";
export const storageKey = "openmath-settings-v1";
export interface SettingsStorage {
  getItem: (key: string) => string | null;
  setItem: (key: string, value: string) => void;
  removeItem: (key: string) => void;
}
interface Stored {
  version: 1;
  general: KernelConfig["general"];
  enabled: boolean;
  send_context: boolean;
  routes: Record<string, string>;
  profiles: ProfileConfig[];
  remembered?: string[];
}
function read(storage: SettingsStorage): Stored | null {
  const text = storage.getItem(storageKey);
  if (!text) return null;
  if (text.length > 1_048_576)
    throw new Error("Invalid stored browser settings");
  const value = JSON.parse(text) as Stored;
  if (
    value.version !== 1 ||
    !Array.isArray(value.profiles) ||
    !value.general ||
    !value.routes
  )
    throw new Error("Invalid stored browser settings");
  return value;
}
export function rememberedNames(
  storage: SettingsStorage = localStorage,
): Set<string> {
  try {
    const value = read(storage);
    return new Set(
      value?.remembered ??
        value?.profiles.filter((p) => p.api_key !== null).map((p) => p.name) ??
        [],
    );
  } catch {
    return new Set();
  }
}
/** Only explicitly opted-in complete provider records may contain secrets. */
export function rememberConfig(
  config: KernelConfig,
  names: Set<string>,
  storage: SettingsStorage = localStorage,
) {
  let previous: Stored | null = null;
  try {
    previous = read(storage);
  } catch {
    previous = null;
  }
  const profiles = config.llm.profiles.map((p) => {
    const saved = { ...p };
    if (!names.has(p.name)) {
      saved.api_key = null;
      saved.extra_headers = {};
      saved.extra_body = publicParameters(p.extra_body ?? {});
      return saved;
    }
    if (saved.api_key === "***") {
      const old = previous?.profiles.find(
        (old) =>
          old.name === p.name &&
          old.kind === p.kind &&
          old.base_url === p.base_url,
      );
      if (!old?.api_key || old.api_key === "***")
        throw new Error("Enter the key again to remember this provider");
      saved.api_key = old.api_key;
    }
    return saved;
  });
  const routes: Record<string, string> = {};
  for (const f of ["translate", "explain", "chat", "complete", "fix"] as const)
    routes[f] = config.llm[f];
  const value: Stored = {
    version: 1,
    general: config.general,
    enabled: config.llm.enabled,
    send_context: config.llm.send_context,
    routes,
    profiles,
    remembered: [...names],
  };
  storage.setItem(storageKey, JSON.stringify(value));
}
/** Merge remembered records into default profiles, never a notebook or source buffer. */
export function restoreConfig(
  base: KernelConfig,
  storage: SettingsStorage = localStorage,
): KernelConfig | null {
  let stored: Stored | null;
  try {
    stored = read(storage);
  } catch {
    return null;
  }
  if (!stored) return null;
  const config = structuredClone(base);
  config.general = { ...config.general, ...stored.general };
  config.llm.enabled = stored.enabled;
  config.llm.send_context = stored.send_context;
  config.llm.profiles = [];
  for (const p of stored.profiles) {
    if (!p || typeof p.name !== "string" || p.api_key === "***") return null;
    const index = config.llm.profiles.findIndex((old) => old.name === p.name);
    if (index < 0) config.llm.profiles.push(p);
    else config.llm.profiles[index] = p;
  }
  for (const f of ["translate", "explain", "chat", "complete", "fix"] as const)
    config.llm[f] = stored.routes[f] ?? "";
  return config;
}
function publicParameters(
  body: Record<string, unknown>,
): Record<string, unknown> {
  const result: Record<string, unknown> = {};
  for (const [key, value] of Object.entries(body)) {
    if (
      value === null ||
      typeof value === "boolean" ||
      typeof value === "number"
    )
      result[key] = value;
    else if (
      key === "reasoning_effort" &&
      ["none", "minimal", "low", "medium", "high", "xhigh"].includes(
        String(value),
      )
    )
      result[key] = value;
    else if (
      key === "thinking" &&
      value &&
      typeof value === "object" &&
      !Array.isArray(value)
    ) {
      const thinking = value as Record<string, unknown>;
      if (["enabled", "disabled", "adaptive"].includes(String(thinking.type)))
        result[key] = {
          type: thinking.type,
          ...(typeof thinking.budget_tokens === "number"
            ? { budget_tokens: thinking.budget_tokens }
            : {}),
        };
    }
  }
  return result;
}
