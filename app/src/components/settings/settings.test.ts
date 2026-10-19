import { expect, test } from "vitest";
import {
  rememberConfig,
  restoreConfig,
  storageKey,
} from "../../state/browserSettings";
import {
  applyPreset,
  presets,
  cloneProfile,
  renameProfile,
  removeProfile,
} from "./presets";
import type { KernelConfig } from "../../kernel/generated/KernelConfig";
const config = {
  general: {
    language: "en",
    dialect: "modern",
    constants: "math",
    reactive: true,
    auto_run_dependents: true,
    show_steps: true,
    auto_plot: true,
    eval_timeout_ms: 30000,
  },
  llm: {
    enabled: true,
    translate: "p",
    chat: "p",
    fix: "p",
    explain: "p",
    complete: "p",
    send_context: false,
    profiles: [
      {
        name: "p",
        kind: "openai_chat",
        base_url: "https://old.invalid/v1",
        model: "old",
        api_key: "synthetic-key",
        api_key_env: null,
        temperature: 0.2,
        max_tokens: 128,
        supports_tools: true,
        supports_json_mode: true,
        timeout_ms: 1000,
        extra_headers: { Authorization: "synthetic-header" },
        extra_body: { thinking: { type: "disabled" } },
      },
    ],
  },
} as KernelConfig;
class Memory {
  data = new Map<string, string>();
  getItem(k: string) {
    return this.data.get(k) ?? null;
  }
  setItem(k: string, v: string) {
    this.data.set(k, v);
  }
  removeItem(k: string) {
    this.data.delete(k);
  }
}
test("only explicitly remembered provider data persists; no mask becomes a stored credential", () => {
  const storage = new Memory();
  rememberConfig(config, new Set(), storage);
  expect(storage.getItem(storageKey)).not.toContain("synthetic-key");
  expect(storage.getItem(storageKey)).not.toContain("synthetic-header");
  const metadata = restoreConfig(config, storage)!;
  expect(metadata.llm.profiles[0]?.base_url).toBe(
    config.llm.profiles[0]?.base_url,
  );
  expect(metadata.llm.profiles[0]?.api_key).toBeNull();
  expect(metadata.llm.profiles[0]?.extra_body).toEqual({
    thinking: { type: "disabled" },
  });
  rememberConfig(config, new Set(["p"]), storage);
  expect(storage.getItem(storageKey)).toContain("synthetic-key");
  expect(restoreConfig(config, storage)?.llm.profiles[0]?.api_key).toBe(
    "synthetic-key",
  );
  expect(() =>
    rememberConfig(
      {
        ...config,
        llm: {
          ...config.llm,
          profiles: [
            { ...config.llm.profiles[0]!, name: "unknown", api_key: "***" },
          ],
        },
      },
      new Set(["unknown"]),
      storage,
    ),
  ).toThrow("Enter the key again");
});
test("preset switches and cloning cannot carry opaque provider credentials across identity or endpoint", () => {
  const old = { ...config.llm.profiles[0]!, api_key: "***" };
  const next = applyPreset(
    old,
    presets.find((p) => p.id === "openai")!,
  );
  expect(next.api_key).toBeNull();
  expect(next.extra_headers).toEqual({});
  expect(next.base_url).toBe("https://api.openai.com/v1");
  const cloned = cloneProfile(old, config.llm.profiles);
  expect(cloned.name).not.toBe("p");
  expect(cloned.api_key).toBeNull();
  const renamed = renameProfile(config.llm, "p", "new");
  expect(renamed.translate).toBe("new");
  expect(renamed.profiles[0]?.name).toBe("new");
  expect(removeProfile(config.llm, "p").profiles).toEqual([]);
  expect(removeProfile(config.llm, "p").translate).toBe("");
});

test('provider registry shares only an opaque credential reference, keeps independent model capabilities, and rejects protected input overrides',async()=>{
  const {registryDraft,newModel,buildModelSettings,defaultAssistantPreferences,modelSettingsError}=await import('../../state/modelRegistry');
  const providers=registryDraft({...config.llm,profiles:[{...config.llm.profiles[0]!,api_key:'***'}]},new Set(['p']));
  providers[0]!.models.push({...newModel(),apiName:'second',displayName:'Second',tools:false,images:true,contextLimit:128000});
  const value=buildModelSettings(config.llm,providers,{...defaultAssistantPreferences,defaultMode:'discuss',defaultModel:providers[0]!.models[1]!.id});
  expect(value.inheritance).toEqual({p:'p',[providers[0]!.models[1]!.id]:'p'});expect(value.llm.profiles[1]).toMatchObject({model:'second',api_key:'***',supports_tools:false});
  expect(JSON.stringify(value.registry)).not.toContain('***');expect(JSON.stringify(value.registry)).not.toContain('synthetic-key');expect(value.registry.providers[0]!.models[1]).toMatchObject({images:true,contextLimit:128000});expect(modelSettingsError(value)).toBeNull();
  value.llm.profiles[1]!.extra_body={messages:[]};expect(modelSettingsError(value)).toContain('cannot replace');
});

test('metadata and assistant preferences survive general settings saves; auth failures are not mislabeled CORS',async()=>{
  const {publicSettings}=await import('../../state/browserSettings');const {probeFailure}=await import('./ProfileProbe');const {messages}=await import('../../i18n');
  const storage=new Memory(),assistant={sendShortcut:'command' as const,defaultMode:'discuss' as const,defaultModel:'p'},registry={version:1 as const,providers:[]};
  rememberConfig(config,new Set(),storage,{registry,assistant});rememberConfig({...config,general:{...config.general,language:'zh-CN'}},new Set(),storage);expect(publicSettings(storage)).toEqual({registry,assistant});
  expect(probeFailure('HTTP status 401',messages('en'),true)).toContain('Authentication rejected');expect(probeFailure('HTTP status 401',messages('en'),true)).not.toContain('CORS');expect(probeFailure('HTTP request or UTF-8 response failed',messages('en'),true)).toContain('CORS or network error');
});
