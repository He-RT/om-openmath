import { memo } from "react";
import type { Dialect } from "../kernel/generated/Dialect";
import type { Messages } from "../i18n";
import { CopyButton } from "./output/Actions";
export function codeDialect(language: string): Dialect | null {
  const name=language.trim().split(/\s+/)[0]?.toLowerCase();
  if (["openmath","om","modern"].includes(name ?? "")) return "Modern";
  if (["wolfram","wl","mathematica"].includes(name ?? "")) return "Wolfram";
  return null;
}
export const CodeBlock=memo(function CodeBlock({ code, language="", t, onInsert }: {code:string;language?:string;t:Messages;onInsert?: ((code:string,dialect:Dialect)=>void)|undefined}) {
  const dialect=codeDialect(language),name=dialect==="Modern" ? "OpenMath":dialect==="Wolfram" ? "Wolfram":language.trim().split(/\s+/)[0] || t.code;
  return <div className="code-block" data-openmath={dialect!==null}>
    <header><span className="code-language" title={language}>{name}</span><span className="code-block-actions"><CopyButton label={t.copy} source={code} t={t}/>{dialect && onInsert && <button onClick={()=>onInsert(code,dialect)}>{t.insertCodeCell}</button>}</span></header>
    <pre tabIndex={0} aria-label={`${t.code} · ${name}`} style={{whiteSpace:"pre",overflowX:"auto"}}><code>{code}</code></pre>
  </div>;
});
