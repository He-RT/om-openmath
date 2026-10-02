import { useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import type { KernelClient } from "../../kernel/client";
import type { ProfileConfig } from "../../kernel/generated/ProfileConfig";
import type { Messages } from "../../i18n";
const consent = new WeakMap<KernelClient, Set<string>>();
export function destination(profile: ProfileConfig) {
  return `${profile.name}:${profile.base_url}:${profile.model}`;
}
export function hasConsent(kernel: KernelClient, profile: ProfileConfig) {
  return consent.get(kernel)?.has(destination(profile)) ?? false;
}
/** Ephemeral consent is scoped to this client and the disclosed destination. */
export function PrivacyPrompt({
  kernel,
  profile,
  t,
  onSend,
  onClose,
}: {
  kernel: KernelClient;
  profile: ProfileConfig;
  t: Messages;
  onSend: () => void;
  onClose: () => void;
}) {
  const [remember, setRemember] = useState(false);
  const root = useRef<HTMLElement>(null);
  useEffect(() => {
    const previous = document.activeElement;
    const element = root.current;
    element?.querySelector<HTMLElement>("button")?.focus();
    const key = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        onClose();
      }
      if (e.key === "Tab" && element) {
        const controls = [
          ...element.querySelectorAll<HTMLElement>("button,input"),
        ];
        const first = controls[0],
          last = controls.at(-1);
        if (e.shiftKey && document.activeElement === first) {
          e.preventDefault();
          last?.focus();
        } else if (!e.shiftKey && document.activeElement === last) {
          e.preventDefault();
          first?.focus();
        }
      }
    };
    element?.addEventListener("keydown", key);
    return () => {
      element?.removeEventListener("keydown", key);
      if (previous instanceof HTMLElement) previous.focus();
    };
  }, [onClose]);
  return createPortal(
    <div
      className="modal-backdrop"
      onMouseDown={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <section
        className="privacy-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="privacy-title"
        ref={root}
      >
        <h2 id="privacy-title">{t.aiPrivacy}</h2>
        <p>{t.explainPrivacy}</p>
        <code className="privacy-destination">{profile.base_url}</code>
        <p>{profile.model}</p>
        <label>
          <input
            type="checkbox"
            checked={remember}
            onChange={(e) => setRemember(e.target.checked)}
          />
          {t.rememberConsent}
        </label>
        <div className="privacy-actions">
          <button onClick={onClose}>{t.cancel}</button>
          <button
            onClick={() => {
              if (remember) {
                let values = consent.get(kernel);
                if (!values) {
                  values = new Set();
                  consent.set(kernel, values);
                }
                values.add(destination(profile));
              }
              onSend();
            }}
          >
            {t.sendAI}
          </button>
        </div>
      </section>
    </div>,
    document.body,
  );
}
