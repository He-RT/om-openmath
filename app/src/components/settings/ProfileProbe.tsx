import { useEffect, useRef, useState } from "react";
import type { KernelClient } from "../../kernel/client";
import type { ProfileConfig } from "../../kernel/generated/ProfileConfig";
import type { Messages } from "../../i18n";
export function ProfileProbe({
  profile,
  kernel,
  t,
  invalid = false,
}: {
  profile: ProfileConfig;
  kernel: KernelClient;
  t: Messages;
  invalid?: boolean;
}) {
  const [value, setValue] = useState<{
    busy: boolean;
    error?: string;
    response?: string;
    latency?: number | null;
    first?: number | null;
  }>({ busy: false });
  const owner = useRef({
    alive: false,
    id: null as string | null,
    cancelled: new Set<string>(),
    pending: new Set<string>(),
  });
  const cancel = () => {
    const s = owner.current;
    if (s.id) {
      const request_id = s.id;
      s.id = null;
      if (s.pending.has(request_id)) s.cancelled.add(request_id);
      void kernel.request({ type: "llm_cancel", request_id }).catch(() => {});
    }
    if (s.alive) setValue({ busy: false });
  };
  useEffect(() => {
    const s = owner.current;
    s.alive = true;
    setValue({ busy: false });
    const off = kernel.onEvent((e) => {
      if (!s.alive || !("request_id" in e) || e.request_id !== s.id) return;
      if (e.type === "llm_profile_test")
        setValue({
          busy: false,
          response: e.response,
          latency: e.latency_ms,
          first: e.first_byte_ms,
        });
      if (e.type === "llm_error") {
        s.id = null;
        setValue({ busy: false, error: e.message });
      }
      if (e.type === "llm_done") s.id = null;
    });
    return () => {
      cancel();
      s.alive = false;
      off();
    };
  }, [kernel, profile, invalid]);
  const start = () => {
    cancel();
    const s = owner.current,
      id = crypto.randomUUID();
    s.id = id;
    s.pending.add(id);
    setValue({ busy: true });
    void kernel
      .request({
        type: "llm_test_profile",
        request_id: id,
        profile: profile.name,
        config: profile,
      })
      .then(
        (r) => {
          s.pending.delete(id);
          if (s.cancelled.delete(id)) {
            if (r.type === "llm_started")
              void kernel
                .request({ type: "llm_cancel", request_id: id })
                .catch(() => {});
            return;
          }
          if (s.alive && s.id === id && r.type !== "llm_started") {
            s.id = null;
            setValue({
              busy: false,
              error: r.type === "error" ? r.message : t.aiRequestError,
            });
          }
        },
        () => {
          s.pending.delete(id);
          s.cancelled.delete(id);
          if (s.alive && s.id === id) {
            s.id = null;
            setValue({ busy: false, error: t.aiRequestError });
          }
        },
      );
  };
  return (
    <div className="profile-probe">
      <div>
        <button
          type="button"
          disabled={
            invalid ||
            value.busy ||
            !profile.name ||
            !profile.base_url ||
            !profile.model
          }
          onClick={start}
        >
          {t.testConnection}
        </button>
        {value.busy && (
          <button type="button" onClick={cancel}>
            {t.cancel}
          </button>
        )}
      </div>
      <p className="settings-hint">{t.testPrivacy}</p>
      {value.error && (
        <p role="alert" className="output-error">
          {value.error}
          {kernel.kind === "wasm" && <span> {t.corsHint}</span>}
        </p>
      )}
      {value.response !== undefined && (
        <p role="status">
          {t.connectionReply}: {value.response} · {t.timing}:{" "}
          {value.latency == null ? "—" : Math.round(value.latency)} ms ·{" "}
          {t.firstByte}: {value.first == null ? "—" : Math.round(value.first)}{" "}
          ms
        </p>
      )}
    </div>
  );
}
