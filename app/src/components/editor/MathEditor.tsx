import { useEffect, useRef, useState } from "react";
import { EditorState, Compartment } from "@codemirror/state";
import {
  EditorView,
  keymap,
  drawSelection,
  hoverTooltip,
  type KeyBinding,
} from "@codemirror/view";
import {
  history,
  historyKeymap,
  defaultKeymap,
  indentWithTab,
} from "@codemirror/commands";
import {
  autocompletion,
  closeBrackets,
  closeBracketsKeymap,
  completionKeymap,
  snippetCompletion,
  type CompletionSource,
} from "@codemirror/autocomplete";
import {
  lintGutter,
  setDiagnostics,
  type Diagnostic as CmDiagnostic,
} from "@codemirror/lint";
import type { KernelClient } from "../../kernel/client";
import type { Dialect } from "../../kernel/generated/Dialect";
import type { PreviewResult } from "../../kernel/generated/PreviewResult";
import type { Diagnostic } from "../../kernel/generated/Diagnostic";
import type { Messages, Locale } from "../../i18n";
import { toByte, fromByte, replaceBytes } from "./positions";
import { highlighting, previewTokens, tokenMarks } from "./highlight";
import { Katex } from "../output/Katex";
interface Props {
  id: string;
  index: number;
  source: string;
  dialect: Dialect;
  settingsKey: string;
  client: KernelClient;
  t: Messages;
  language: Locale;
  focus: boolean;
  focusKey: number;
  onChange: (source: string) => void;
  onFocus: () => void;
  onRun: (mode: "stay" | "next" | "insert") => void;
  onAction: (source: string) => void;
  onSelect: (action: string, source: string, dialect: Dialect) => void;
}
const names: Record<string, string> = {
  alpha: "α",
  beta: "β",
  gamma: "γ",
  delta: "δ",
  theta: "θ",
  lambda: "λ",
  pi: "π",
  sigma: "σ",
  omega: "ω",
};
function diagnosticText(d: Diagnostic, language: Locale) {
  if (language === "zh-CN") return d.message;
  const table: Record<string, string> = {
    E001: "Invalid character",
    E002: "Unclosed string",
    E003: "Unclosed comment",
    E005: "Unknown named character",
    E006: "Invalid string escape",
    E023: "Missing closing bracket",
    E024: "Unexpected closing bracket",
    E010: "Ambiguous function call",
    W001: "Implicit multiplication",
    W002: "Ambiguous adjacent calls",
  };
  return table[d.code] ?? d.message;
}
export function MathEditor(props: Props) {
  const root = useRef<HTMLDivElement>(null);
  const editor = useRef<EditorView | null>(null);
  const latest = useRef(props);
  latest.current = props;
  const generation = useRef(0);
  const [preview, setPreview] = useState<PreviewResult | null>(null);
  const previewSource = useRef("");
  const [selection, setSelection] = useState("");
  const attributes = useRef(new Compartment());
  useEffect(() => {
    if (!root.current) return;
    const completion: CompletionSource = async (context) => {
      const source = context.state.doc.toString();
      const dialect = latest.current.dialect;
      const settings = latest.current.settingsKey;
      const response = await props.client
        .request({
          type: "complete",
          source,
          dialect,
          cursor: toByte(source, context.pos),
        })
        .catch(() => null);
      if (
        !response ||
        context.aborted ||
        editor.current?.state.doc.toString() !== source ||
        latest.current.dialect !== dialect ||
        latest.current.settingsKey !== settings ||
        response.type !== "completions"
      )
        return null;
      return {
        from: fromByte(source, response.from),
        to: fromByte(source, response.to),
        options: response.items.map((item) =>
          snippetCompletion(item.insert_text, {
            label: item.label,
            ...(item.detail ? { detail: item.detail } : {}),
            type:
              item.kind === "Symbol"
                ? "variable"
                : item.kind === "Keyword"
                  ? "keyword"
                  : "function",
          }),
        ),
      };
    };
    const bindings: KeyBinding[] = [
      {
        key: "Mod-Enter",
        run: () => {
          latest.current.onRun("stay");
          return true;
        },
      },
      {
        key: "Shift-Enter",
        run: () => {
          latest.current.onRun("next");
          return true;
        },
      },
      {
        key: "Alt-Enter",
        run: () => {
          latest.current.onRun("insert");
          return true;
        },
      },
      {
        key: "Tab",
        run: (view) => {
          const cursor = view.state.selection.main.head;
          const source = view.state.doc.toString();
          const before = source.slice(0, cursor);
          const name = /\\([a-z]+)$/.exec(before)?.[1];
          const char = name ? names[name] : undefined;
          if (!char) return false;
          void props.client
            .request({
              type: "preview",
              source,
              dialect: latest.current.dialect,
              cursor: toByte(source, cursor),
            })
            .then((response) => {
              if (
                editor.current !== view ||
                view.state.doc.toString() !== source ||
                view.state.selection.main.head !== cursor ||
                response.type !== "preview"
              )
                return;
              const byte = toByte(source, cursor);
              if (
                response.tokens.some(
                  ([span, kind]) =>
                    (kind === "String" || kind === "Comment") &&
                    span.start <= byte &&
                    byte <= span.end,
                )
              ) {
                indentWithTab.run?.(view);
                return;
              }
              view.dispatch({
                changes: {
                  from: cursor - (name?.length ?? 0) - 1,
                  to: cursor,
                  insert: char,
                },
              });
            })
            .catch(() => {});
          return true;
        },
      },
    ];
    const view = new EditorView({
      parent: root.current,
      state: EditorState.create({
        doc: props.source,
        extensions: [
          history(),
          drawSelection(),
          EditorView.lineWrapping,
          closeBrackets(),
          keymap.of([
            ...bindings,
            ...closeBracketsKeymap,
            ...completionKeymap,
            ...historyKeymap,
            ...defaultKeymap,
            indentWithTab,
          ]),
          autocompletion({ override: [completion], maxRenderedOptions: 20 }),
          lintGutter(),
          highlighting,
          EditorView.decorations.from(highlighting),
          attributes.current.of(
            EditorView.contentAttributes.of({
              "aria-label": `${props.t.editor} ${props.index + 1}`,
              role: "textbox",
              "aria-multiline": "true",
              spellcheck: "false",
            }),
          ),
          hoverTooltip(async (view, pos) => {
            const source = view.state.doc.toString();
            const dialect = latest.current.dialect;
            const settings = latest.current.settingsKey;
            const response = await props.client
              .request({
                type: "hover",
                source,
                dialect,
                cursor: toByte(source, pos),
              })
              .catch(() => null);
            if (
              !response ||
              view.state.doc.toString() !== source ||
              latest.current.dialect !== dialect ||
              latest.current.settingsKey !== settings ||
              response.type !== "hover" ||
              !response.info
            )
              return null;
            const info = response.info;
            return {
              pos,
              create: () => {
                const dom = document.createElement("div");
                dom.className = "math-help";
                const title = document.createElement("strong");
                title.textContent = info.signature ?? info.value ?? "";
                const text = document.createElement("p");
                text.textContent = info.summary;
                dom.append(title, text);
                if (info.examples.length) {
                  const sample = document.createElement("code");
                  sample.textContent = info.examples[0] ?? "";
                  dom.append(sample);
                }
                return { dom };
              },
            };
          }),
          EditorView.updateListener.of((update) => {
            if (update.docChanged)
              latest.current.onChange(update.state.doc.toString());
            if (update.focusChanged && view.hasFocus) latest.current.onFocus();
            if (update.selectionSet || update.docChanged) {
              const range = update.state.selection.main;
              setSelection(update.state.doc.sliceString(range.from, range.to));
            }
          }),
        ],
      }),
    });
    editor.current = view;
    return () => {
      generation.current++;
      editor.current = null;
      view.destroy();
    };
  }, [props.client, props.id]);
  useEffect(() => {
    const view = editor.current;
    if (view && view.state.doc.toString() !== props.source)
      view.dispatch({
        changes: { from: 0, to: view.state.doc.length, insert: props.source },
      });
  }, [props.source]);
  useEffect(() => {
    editor.current?.dispatch({
      effects: attributes.current.reconfigure(
        EditorView.contentAttributes.of({
          "aria-label": `${props.t.editor} ${props.index + 1}`,
          role: "textbox",
          "aria-multiline": "true",
          spellcheck: "false",
        }),
      ),
    });
  }, [props.t.editor, props.index]);
  useEffect(() => {
    if (props.focus) editor.current?.focus();
  }, [props.focusKey]);
  useEffect(() => {
    const request = ++generation.current;
    setPreview(null);
    const timer = setTimeout(() => {
      const view = editor.current;
      if (!view) return;
      const source = view.state.doc.toString();
      void props.client
        .request({
          type: "preview",
          source,
          dialect: props.dialect,
          cursor: toByte(source, view.state.selection.main.head),
        })
        .then((response) => {
          if (
            request !== generation.current ||
            editor.current !== view ||
            view.state.doc.toString() !== source ||
            response.type !== "preview"
          )
            return;
          previewSource.current = source;
          setPreview(response);
          const diagnostics: CmDiagnostic[] = response.diagnostics.map((d) => ({
            from: fromByte(source, d.span.start),
            to: fromByte(source, d.span.end),
            severity:
              d.severity === "Error"
                ? "error"
                : d.severity === "Warning"
                  ? "warning"
                  : "info",
            message: `${d.code}: ${diagnosticText(d, props.language)}`,
            actions: d.fix
              ? [
                  {
                    name: props.t.fix,
                    apply: (target) =>
                      target.dispatch({
                        changes: {
                          from: fromByte(source, d.fix?.span.start ?? 0),
                          to: fromByte(source, d.fix?.span.end ?? 0),
                          insert: d.fix?.replacement ?? "",
                        },
                      }),
                  },
                ]
              : [],
          }));
          view.dispatch(setDiagnostics(view.state, diagnostics), {
            effects: previewTokens.of(tokenMarks(source, response.tokens)),
          });
        })
        .catch(() => {});
    }, 80);
    return () => {
      clearTimeout(timer);
      generation.current++;
    };
  }, [
    props.client,
    props.source,
    props.dialect,
    props.language,
    props.t.fix,
    props.settingsKey,
  ]);
  const currentPreview =
    previewSource.current === props.source ? preview : null;
  return (
    <div className="math-editor-area">
      {selection && (
        <div className="selection-tools" aria-label={props.t.selection}>
          {(["factor", "expand", "simplify", "solve", "plot"] as const).map(
            (action) => (
              <button
                key={action}
                onClick={() =>
                  props.onSelect(
                    action,
                    selection,
                    currentPreview?.dialect ?? props.dialect,
                  )
                }
              >
                {props.t[action]}
              </button>
            ),
          )}
        </div>
      )}
      <div ref={root} className="math-editor" />
      <div className="input-preview" aria-label={props.t.preview}>
        {currentPreview?.latex ? (
          <Katex latex={currentPreview.latex} />
        ) : (
          <span className="preview-empty">
            {props.source.trim() ? props.t.previewInvalid : "…"}
          </span>
        )}
      </div>
      {currentPreview && currentPreview.diagnostics.length > 0 && (
        <div className="inline-diagnostics">
          {currentPreview.diagnostics.map((d, i) => (
            <div key={i} className={`diagnostic ${d.severity.toLowerCase()}`}>
              <span>
                {d.code} · {diagnosticText(d, props.language)}
              </span>
              {d.fix && (
                <button
                  onClick={() =>
                    props.onChange(
                      replaceBytes(
                        props.source,
                        d.fix!.span,
                        d.fix!.replacement,
                      ),
                    )
                  }
                >
                  {props.t.fix}
                </button>
              )}
            </div>
          ))}
        </div>
      )}
      {currentPreview && currentPreview.actions.length > 0 && (
        <div className="input-actions">
          {currentPreview.actions.map((action, i) => (
            <button key={i} onClick={() => props.onAction(action.source)}>
              {props.t.solve} ↗
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
