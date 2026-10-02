import { useMemo } from "react";
import katex from "katex";
import "katex/dist/katex.min.css";
export function Katex({
  latex,
  label = latex,
  display = false,
}: {
  latex: string;
  label?: string;
  display?: boolean;
}) {
  const markup = useMemo(() => {
    try {
      return katex.renderToString(latex, {
        displayMode: display,
        throwOnError: true,
        trust: false,
        strict: "ignore",
        output: "htmlAndMathml",
        maxExpand: 1000,
      });
    } catch {
      return null;
    }
  }, [latex, display]);
  return markup ? (
    <span
      className={`formula ${display ? "formula-display" : ""}`}
      aria-label={label}
      dangerouslySetInnerHTML={{ __html: markup }}
    />
  ) : (
    <code className="formula-fallback" aria-label={label}>{latex}</code>
  );
}
