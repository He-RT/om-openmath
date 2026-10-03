import { useEffect, useRef, useState } from "react";
import type { ProfileConfig } from "../../kernel/generated/ProfileConfig";
import type { KernelClient } from "../../kernel/client";
import type { Messages } from "../../i18n";
import { presets, applyPreset } from "./presets";
import { ProfileProbe } from "./ProfileProbe";
const kinds = [
  "openai_chat",
  "anthropic",
  "openai_fim",
  "ollama_fim",
  "mistral_fim",
] as const;
const emptyParameters: Record<string, unknown> = {};
function JsonFields({
  value,
  onChange,
  onValid,
  t,
  headers = false,
}: {
  value: Record<string, unknown>;
  onChange: (value: Record<string, unknown>) => void;
  onValid: (valid: boolean) => void;
  t: Messages;
  headers?: boolean;
}) {
  const [text, setText] = useState(JSON.stringify(value, null, 2)),
    [error, setError] = useState(false);
  const edited = useRef<Record<string, unknown> | null>(null),
    report = useRef(onValid);
  report.current = onValid;
  useEffect(() => {
    if (edited.current === value) {
      edited.current = null;
      return;
    }
    setText(JSON.stringify(value, null, 2));
    setError(false);
    report.current(true);
  }, [value]);
  return (
    <label>
      {headers ? t.extraHeaders : t.extraParameters}
      <textarea
        aria-label={headers ? t.extraHeaders : t.extraParameters}
        spellCheck={false}
        value={text}
        onChange={(e) => {
          const source = e.target.value;
          setText(source);
          try {
            const value = JSON.parse(source || "{}") as Record<string, unknown>;
            if (
              !value ||
              typeof value !== "object" ||
              Array.isArray(value) ||
              (headers &&
                Object.values(value).some((v) => typeof v !== "string"))
            )
              throw new Error();
            edited.current = value;
            onChange(value);
            setError(false);
            onValid(true);
          } catch {
            setError(true);
            onValid(false);
          }
        }}
      />
      {error && (
        <span role="alert" className="output-error">
          {t.invalidJson}
        </span>
      )}
    </label>
  );
}
export function ProfileForm({
  profile,
  onChange,
  onRename,
  kernel,
  t,
  remember,
  onRemember,
  onValidity,
  invalid = false,
}: {
  profile: ProfileConfig;
  onChange: (patch: Partial<ProfileConfig>) => void;
  onRename: (name: string) => void;
  kernel: KernelClient;
  t: Messages;
  remember: boolean;
  onRemember: (remember: boolean) => void;
  onValidity: (key: string, valid: boolean) => void;
  invalid?: boolean;
}) {
  const chat = profile.kind === "openai_chat" || profile.kind === "anthropic";
  return (
    <div className="profile-form">
      <label>
        {t.preset}
        <select
          aria-label={t.preset}
          value=""
          onChange={(e) => {
            const preset = presets.find((p) => p.id === e.target.value);
            if (preset) onChange(applyPreset(profile, preset));
          }}
        >
          <option value="">{t.choosePreset}</option>
          {presets.map((p) => (
            <option key={p.id} value={p.id}>
              {p.label}
            </option>
          ))}
        </select>
      </label>
      <div className="profile-basic">
        <label>
          {t.profileName}
          <input
            aria-label={t.profileName}
            value={profile.name}
            onChange={(e) => onRename(e.target.value)}
          />
        </label>
        <label>
          {t.providerType}
          <select
            aria-label={t.providerType}
            value={profile.kind}
            onChange={(e) =>
              onChange({
                kind: e.target.value as ProfileConfig["kind"],
                api_key: null,
                api_key_env: null,
                extra_headers: {},
                extra_body: {},
              })
            }
          >
            {kinds.map((k) => (
              <option key={k} value={k}>
                {t[k]}
              </option>
            ))}
          </select>
        </label>
      </div>
      <label>
        Base URL
        <input
          aria-label="Base URL"
          spellCheck={false}
          value={profile.base_url}
          onChange={(e) =>
            onChange({
              base_url: e.target.value,
              api_key: null,
              api_key_env: null,
              extra_headers: {},
            })
          }
        />
      </label>
      <label>
        {t.model}
        <input
          aria-label={t.model}
          spellCheck={false}
          value={profile.model}
          onChange={(e) => onChange({ model: e.target.value })}
        />
      </label>
      <label className="settings-toggle">
        <input
          type="checkbox"
          checked={profile.requires_api_key !== false}
          onChange={(e) =>
            onChange({
              requires_api_key: e.target.checked,
              ...(!e.target.checked
                ? { api_key: null, api_key_env: null }
                : {}),
            })
          }
        />
        {t.requireKey}
      </label>
      <label>
        API Key
        <input
          aria-label="API Key"
          type="password"
          autoComplete="new-password"
          placeholder={
            profile.api_key === "***" ? t.keyStored : t.keyPlaceholder
          }
          value={profile.api_key === "***" ? "" : (profile.api_key ?? "")}
          onChange={(e) => onChange({ api_key: e.target.value || null })}
        />
        <button type="button" onClick={() => onChange({ api_key: null })}>
          {t.clearKey}
        </button>
      </label>
      {kernel.kind === "tauri" ? (
        <>
          <label>
            {t.keyEnvironment}
            <input
              aria-label={t.keyEnvironment}
              value={profile.api_key_env ?? ""}
              onChange={(e) =>
                onChange({ api_key_env: e.target.value || null })
              }
            />
          </label>
          <p className="settings-hint">{t.nativeKeyStorage}</p>
        </>
      ) : (
        <>
          <label className="settings-toggle">
            <input
              type="checkbox"
              checked={remember}
              onChange={(e) => onRemember(e.target.checked)}
            />
            {t.rememberProvider}
          </label>
          {remember && <p className="settings-risk">{t.storageRisk}</p>}
        </>
      )}
      <div className="profile-numbers">
        <label>
          {t.temperature}
          <input
            type="number"
            aria-label={t.temperature}
            min="0"
            max="2"
            step="0.1"
            value={profile.temperature}
            onChange={(e) => onChange({ temperature: Number(e.target.value) })}
          />
        </label>
        <label>
          {t.maxTokens}
          <input
            type="number"
            aria-label={t.maxTokens}
            min="1"
            value={profile.max_tokens}
            onChange={(e) => onChange({ max_tokens: Number(e.target.value) })}
          />
        </label>
        <label>
          {t.httpTimeout}
          <input
            type="number"
            aria-label={t.httpTimeout}
            min="1"
            value={profile.timeout_ms}
            onChange={(e) => onChange({ timeout_ms: Number(e.target.value) })}
          />
        </label>
      </div>
      <label className="settings-toggle">
        <input
          type="checkbox"
          checked={profile.supports_tools}
          disabled={!chat}
          onChange={(e) => onChange({ supports_tools: e.target.checked })}
        />
        {t.supportsTools}
      </label>
      <label className="settings-toggle">
        <input
          type="checkbox"
          checked={profile.supports_json_mode}
          disabled={!chat}
          onChange={(e) => onChange({ supports_json_mode: e.target.checked })}
        />
        {t.supportsJson}
      </label>
      <JsonFields
        value={profile.extra_headers}
        onValid={(valid) => onValidity("headers", valid)}
        headers
        t={t}
        onChange={(value) =>
          onChange({ extra_headers: value as Record<string, string> })
        }
      />
      <JsonFields
        value={profile.extra_body ?? emptyParameters}
        onValid={(valid) => onValidity("parameters", valid)}
        t={t}
        onChange={(extra_body) => onChange({ extra_body })}
      />
      {kernel.kind === "wasm" && (
        <p className="settings-hint">
          {profile.kind === "ollama_fim" || profile.base_url.includes(":11434")
            ? t.ollamaCors
            : profile.kind === "anthropic"
              ? t.anthropicCors
              : t.corsHint}
        </p>
      )}
      <ProfileProbe invalid={invalid} profile={profile} kernel={kernel} t={t} />
    </div>
  );
}
