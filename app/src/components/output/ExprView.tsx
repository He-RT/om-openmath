import { useEffect, useRef, useState } from "react";
import type { KernelClient } from "../../kernel/client";
import type { ExpressionView } from "../../kernel/generated/ExpressionView";
import type { Messages } from "../../i18n";
import { Katex } from "./Katex";
import { SourceActions } from "./Actions";
export function ExprView({
  value,
  kernel,
  t,
  onInsert,
}: {
  value: ExpressionView;
  kernel: KernelClient;
  t: Messages;
  onInsert: (source: string) => void;
}) {
  const [numeric, setNumeric] = useState<ExpressionView | null>(null);
  const [showNumeric, setShowNumeric] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState(false);
  const alive = useRef(true);
  useEffect(() => {
    alive.current = true;
    return () => {
      alive.current = false;
    };
  }, []);
  const displayed = showNumeric && numeric ? numeric : value;
  return (
    <div className="expr-view">
      <Katex display latex={displayed.latex} label={displayed.input_form} />
      <div className="output-toolbar" aria-label={t.resultActions}>
        <SourceActions value={displayed} t={t} onInsert={onInsert} />
        <button
          disabled={busy}
          aria-pressed={showNumeric}
          onClick={() => {
            setError(false);
            if (showNumeric || numeric) {
              setShowNumeric(!showNumeric);
              return;
            }
            setBusy(true);
            void kernel
              .request({
                type: "inspect_expression",
                source: value.input_form,
                numeric: true,
              })
              .then(
                (response) => {
                  if (!alive.current) return;
                  if (response.type !== "expression") {
                    setError(true);
                    return;
                  }
                  setNumeric(response.value);
                  setShowNumeric(true);
                },
                () => {
                  if (alive.current) setError(true);
                },
              )
              .finally(() => {
                if (alive.current) setBusy(false);
              });
          }}
        >
          {showNumeric ? t.exact : t.numericAction}
        </button>
      </div>
      {error && (
        <span role="alert" className="output-error">
          {t.inspectError}
        </span>
      )}
    </div>
  );
}
