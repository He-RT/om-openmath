import { useState } from "react";
import type { Messages } from "../../i18n";
export function CopyButton({
  label,
  source,
  t,
}: {
  label: string;
  source: string;
  t: Messages;
}) {
  const [status, setStatus] = useState<"idle" | "copied" | "error">("idle");
  return (
    <span className="copy-action">
      <button
        onClick={() => {
          setStatus("idle");
          void Promise.resolve()
            .then(() => navigator.clipboard.writeText(source))
            .then(
              () => setStatus("copied"),
              () => setStatus("error"),
            );
        }}
      >
        {label}
      </button>
      {status !== "idle" && (
        <span role={status === "error" ? "alert" : "status"}>
          {status === "error" ? t.copyError : t.copied}
        </span>
      )}
    </span>
  );
}
export function SourceActions({
  value,
  t,
  onInsert,
}: {
  value: { input_form: string; modern_form: string; latex: string };
  t: Messages;
  onInsert: (source: string) => void;
}) {
  return (
    <>
      <CopyButton label={t.copyLatex} source={value.latex} t={t} />
      <CopyButton label={t.copyWolfram} source={value.input_form} t={t} />
      <CopyButton label={t.copyModern} source={value.modern_form} t={t} />
      <button onClick={() => onInsert(value.input_form)}>
        {t.insertResult}
      </button>
    </>
  );
}
