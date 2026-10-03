import type { LlmConfig } from "../../kernel/generated/LlmConfig";
export function completionProfile(config: LlmConfig | null | undefined) {
  if (!config?.enabled || !config.complete) return null;
  const profiles = config.profiles.filter((p) => p.name === config.complete);
  if (profiles.length !== 1) return null;
  const profile = profiles[0];
  if (!profile?.base_url || !profile.model) return null;
  let local: boolean;
  try {
    local = ["localhost", "127.0.0.1", "[::1]"].includes(
      new URL(profile.base_url).hostname,
    );
  } catch {
    return null;
  }
  return profile.requires_api_key === false ||
    profile.kind === "ollama_fim" ||
    local ||
    profile.api_key !== null ||
    Object.keys(profile.extra_headers).length > 0
    ? profile
    : null;
}
