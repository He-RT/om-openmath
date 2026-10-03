import type { Page } from "@playwright/test";
// Restore only synthetic test settings through the same path as a saved browser profile.
export async function fixtureProfile(page: Page, base_url: string, api_key: string, complete = false) {
  await page.addInitScript(({ base_url, api_key, complete }) => {
    localStorage.setItem("openmath-settings-v1", JSON.stringify({
      version: 1, general: {}, enabled: true, send_context: false,
      routes: { translate: "deepseek", explain: "deepseek", chat: "deepseek", fix: "deepseek", complete: complete ? "deepseek" : "" },
      remembered: ["deepseek"],
      profiles: [{ name: "deepseek", kind: "openai_chat", base_url, model: "deepseek-flash", api_key, api_key_env: null, temperature: 0.2, max_tokens: 1024, supports_tools: true, supports_json_mode: true, timeout_ms: 60000, extra_headers: {} }],
    }));
  }, { base_url, api_key, complete });
}
