import { useState, type ReactNode } from "react";
import type { Messages } from "../../i18n";
import { ToolbarMenu } from "../ToolbarMenu";
import { WorkbenchIcon } from "../WorkbenchIcon";
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
  return <CopyMenu value={value} t={t} onInsert={onInsert} />;
}
export function CopyMenu({ value, t, onInsert, all = false, children }: { value: { input_form: string; modern_form: string; latex?: string }; t: Messages; onInsert?: (source: string) => void; all?: boolean; children?: ReactNode }) {
  const [status,setStatus]=useState<"idle"|"copied"|"error">("idle");
  const copy=(source:string)=>{setStatus("idle");void Promise.resolve().then(()=>navigator.clipboard.writeText(source)).then(()=>setStatus("copied"),()=>setStatus("error"));};
  return <span className="copy-action">
    <ToolbarMenu className="copy-menu" label={t.copy} icon={<><span>{t.copy}</span><WorkbenchIcon name="chevron"/></>}>
      {value.latex !== undefined && <button onClick={()=>copy(value.latex!)}>{t.copyLatex}</button>}
      <button onClick={()=>copy(value.input_form)}>{all ? t.copyAll:t.copyWolfram}</button>
      <button onClick={()=>copy(value.modern_form)}>{t.copyModern}</button>
      {onInsert && <><hr/><button onClick={()=>onInsert(value.input_form)}>{t.insertResult}</button></>}
      {children}
    </ToolbarMenu>
    {status!=="idle" && <span role={status==="error" ? "alert":"status"}>{status==="error" ? t.copyError:t.copied}</span>}
  </span>;
}
