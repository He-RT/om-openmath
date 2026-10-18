import { useEffect, useRef, useState, type ReactNode } from "react";
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
  renderStatus,
  active=true,
}: {
  value: ExpressionView;
  kernel: KernelClient;
  t: Messages;
  onInsert: (source: string) => void;
  renderStatus?: (actions: ReactNode) => ReactNode;
  active?: boolean;
}) {
  const [numeric, setNumeric] = useState<ExpressionView | null>(null);
  const [showNumeric, setShowNumeric] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState(false);
  const alive = useRef(true);
  const sequence=useRef(0);
  useEffect(() => {
    alive.current = true;
    return () => {
      alive.current = false;
    };
  }, []);
  useEffect(()=>{sequence.current++;if(!active){setBusy(false);setNumeric(null);setShowNumeric(false);}},[active]);
  const displayed = showNumeric && numeric ? numeric : value;
  const actions=(<>
        {showNumeric && <span className="numeric-projection">{t.numericProjection}</span>}
        <SourceActions value={displayed} t={t} onInsert={onInsert} />
        <button
          disabled={busy || !active}
          aria-pressed={showNumeric}
          onClick={() => {
            setError(false);
            if (showNumeric || numeric) {
              setShowNumeric(!showNumeric);
              return;
            }
            setBusy(true);
            const request=++sequence.current;
            void kernel
              .request({
                type: "inspect_expression",
                source: value.input_form,
                numeric: true,
              })
              .then(
                (response) => {
                  if (!alive.current || sequence.current!==request) return;
                  if (response.type !== "expression") {
                    setError(true);
                    return;
                  }
                  setNumeric(response.value);
                  setShowNumeric(true);
                },
                () => {
                  if (alive.current && sequence.current===request) setError(true);
                },
              )
              .finally(() => {
                if (alive.current && sequence.current===request) setBusy(false);
              });
          }}
        >
          {showNumeric ? t.exact : t.numericAction}
        </button>
  </>);
  return (
    <div className="expr-view">
      <Katex display latex={displayed.latex} label={displayed.input_form} />
      {renderStatus ? renderStatus(actions) : <div className="output-toolbar" aria-label={t.resultActions}>{actions}</div>}
      {error && (
        <span role="alert" className="output-error">
          {t.inspectError}
        </span>
      )}
    </div>
  );
}
