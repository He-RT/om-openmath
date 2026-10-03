import { useState } from "react";
import type { KernelConfig } from "../../kernel/generated/KernelConfig";
import type { NotebookController } from "../../state/notebookStore";
import type { Messages } from "../../i18n";
import {
  newProfile,
  uniqueName,
  cloneProfile,
  renameProfile,
  removeProfile,
  features,
  validationError,
} from "./presets";
import { rememberConfig, rememberedNames } from "../../state/browserSettings";
import { ProfileForm } from "./ProfileForm";
export function ModelSettings({
  config,
  controller,
  t,
  featuresOnly = false,
}: {
  config: KernelConfig;
  controller: NotebookController;
  t: Messages;
  featuresOnly?: boolean;
}) {
  const [llm, setLlm] = useState(() => structuredClone(config.llm)),
    [selected, setSelected] = useState(0),
    [remember, setRemember] = useState(() =>
      controller.kernel.kind === "wasm" ? rememberedNames() : new Set<string>(),
    ),
    [error, setError] = useState<string | null>(null),
    [saved, setSaved] = useState(false),
    [saving, setSaving] = useState(false);
  const [ids, setIds] = useState(() =>
      config.llm.profiles.map(() => crypto.randomUUID()),
    ),
    [invalidFields, setInvalidFields] = useState<Record<string, boolean>>({});
  const change = (next: typeof llm) => {
    setLlm(next);
    setSaved(false);
    setError(null);
  };
  const profile = llm.profiles[selected];
  const save = async () => {
    const invalid = Object.values(invalidFields).some(Boolean)
      ? "Invalid JSON fields"
      : validationError(llm);
    if (invalid) {
      setError(`${t.invalidSettings}: ${invalid}`);
      return;
    }
    setSaving(true);
    setError(null);
    try {
      // Validate opaque key remembrance before native/browser state changes.
      if (controller.kernel.kind === "wasm") {
        const memory = new Map<string, string>();
        const fake = {
          getItem: (k: string) => memory.get(k) ?? localStorage.getItem(k),
          setItem: (k: string, v: string) => {
            memory.set(k, v);
          },
          removeItem: (k: string) => {
            memory.delete(k);
          },
        };
        rememberConfig({ ...config, llm }, remember, fake);
      }
      const effective = await controller.configureLlm(llm);
      if (controller.kernel.kind === "wasm")
        rememberConfig(effective, remember);
      setLlm(structuredClone(effective.llm));
      setSaved(true);
    } catch (e) {
      setError(e instanceof Error ? e.message : t.aiRequestError);
    } finally {
      setSaving(false);
    }
  };
  return (
    <div className="model-settings">
      <label className="settings-toggle">
        <input
          type="checkbox"
          checked={llm.enabled}
          onChange={(e) => change({ ...llm, enabled: e.target.checked })}
        />
        {t.enableAI}
      </label>
      {featuresOnly ? (
        <>
          <div className="feature-mappings">
            {features.map((f) => (
              <label key={f}>
                {
                  t[
                    f === "fix"
                      ? "fixAI"
                      : f === "complete"
                        ? "completion"
                        : f === "translate"
                          ? "translation"
                          : f === "explain"
                            ? "explanation"
                            : "assistant"
                  ]
                }
                <select
                  aria-label={`${t.featureProfile} ${f}`}
                  value={llm[f]}
                  onChange={(e) => change({ ...llm, [f]: e.target.value })}
                >
                  <option value="">{t.disabled}</option>
                  {llm.profiles
                    .filter(
                      (p) =>
                        f === "complete" ||
                        (["openai_chat", "anthropic"].includes(p.kind) &&
                          (f !== "chat" || p.supports_tools)),
                    )
                    .map((p) => (
                      <option key={p.name} value={p.name}>
                        {p.name}
                      </option>
                    ))}
                </select>
              </label>
            ))}
          </div>
          <label className="settings-toggle">
            <input
              type="checkbox"
              checked={llm.send_context}
              onChange={(e) =>
                change({ ...llm, send_context: e.target.checked })
              }
            />
            {t.sendContext}
          </label>
        </>
      ) : (
        <div className="profile-layout">
          <div className="profile-list" role="listbox" aria-label={t.profiles}>
            {llm.profiles.map((p, i) => (
              <button
                type="button"
                role="option"
                aria-selected={i === selected}
                key={i}
                onClick={() => setSelected(i)}
              >
                {p.name || t.unnamedProfile}
              </button>
            ))}
            <div className="profile-list-actions">
              <button
                type="button"
                onClick={() => {
                  change({
                    ...llm,
                    profiles: [
                      ...llm.profiles,
                      newProfile(uniqueName("model", llm.profiles)),
                    ],
                  });
                  setIds((old) => [...old, crypto.randomUUID()]);
                  setSelected(llm.profiles.length);
                }}
              >
                {t.addProfile}
              </button>
              {profile && (
                <>
                  <button
                    type="button"
                    onClick={() => {
                      change({
                        ...llm,
                        profiles: [
                          ...llm.profiles,
                          cloneProfile(profile, llm.profiles),
                        ],
                      });
                      setIds((old) => [...old, crypto.randomUUID()]);
                      setSelected(llm.profiles.length);
                    }}
                  >
                    {t.duplicateProfile}
                  </button>
                  <button
                    type="button"
                    onClick={() => {
                      change(removeProfile(llm, profile.name));
                      setIds((old) => old.filter((_, i) => i !== selected));
                      setInvalidFields((old) =>
                        Object.fromEntries(
                          Object.entries(old).filter(
                            ([key]) => !key.startsWith(ids[selected] + ":"),
                          ),
                        ),
                      );
                      setSelected(Math.max(0, selected - 1));
                    }}
                  >
                    {t.deleteProfile}
                  </button>
                </>
              )}
            </div>
          </div>
          {profile && (
            <ProfileForm
              key={ids[selected]}
              invalid={Object.entries(invalidFields).some(
                ([key, value]) => value && key.startsWith(ids[selected] + ":"),
              )}
              onValidity={(field, valid) =>
                setInvalidFields((old) => {
                  const key = ids[selected] + ":" + field;
                  if (old[key] === !valid) return old;
                  return { ...old, [key]: !valid };
                })
              }
              profile={profile}
              kernel={controller.kernel}
              t={t}
              remember={remember.has(profile.name)}
              onRemember={(value) => {
                setSaved(false);
                setRemember((old) => {
                  const next = new Set(old);
                  if (value) next.add(profile.name);
                  else next.delete(profile.name);
                  return next;
                });
              }}
              onRename={(name) => {
                change(renameProfile(llm, profile.name, name));
                setRemember((old) => {
                  const next = new Set(old);
                  if (next.delete(profile.name)) next.add(name);
                  return next;
                });
              }}
              onChange={(patch) => {
                const next = {
                  ...llm,
                  profiles: llm.profiles.map((p, i) =>
                    i === selected ? { ...p, ...patch } : p,
                  ),
                };
                for (const feature of features) {
                  const mapped = next.profiles.find(
                    (p) => p.name === next[feature],
                  );
                  if (
                    mapped &&
                    feature !== "complete" &&
                    (!["openai_chat", "anthropic"].includes(mapped.kind) ||
                      (feature === "chat" && !mapped.supports_tools))
                  )
                    next[feature] = "";
                }
                change(next);
              }}
            />
          )}
        </div>
      )}
      {error && (
        <p role="alert" className="output-error">
          {error}
        </p>
      )}
      {saved && <p role="status">{t.settingsSaved}</p>}
      <div className="settings-save">
        <button type="button" disabled={saving} onClick={() => void save()}>
          {saving ? t.saving : t.saveSettings}
        </button>
      </div>
    </div>
  );
}
