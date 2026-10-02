import { StateEffect, StateField } from "@codemirror/state";
import { Decoration, type DecorationSet } from "@codemirror/view";
import type { PreviewResult } from "../../kernel/generated/PreviewResult";
import { fromByte } from "./positions";
export const previewTokens = StateEffect.define<DecorationSet>();
export const highlighting = StateField.define<DecorationSet>({
  create: () => Decoration.none,
  update: (value, transaction) => {
    value = value.map(transaction.changes);
    for (const effect of transaction.effects)
      if (effect.is(previewTokens)) value = effect.value;
    return value;
  },
});
export function tokenMarks(source: string, tokens: PreviewResult["tokens"]) {
  return Decoration.set(
    tokens.flatMap(([span, kind]) => {
      const from = fromByte(source, span.start);
      const to = fromByte(source, span.end);
      return to > from
        ? [
            Decoration.mark({ class: `syntax-${kind.toLowerCase()}` }).range(
              from,
              to,
            ),
          ]
        : [];
    }),
    true,
  );
}
