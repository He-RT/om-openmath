import {
  EditorSelection,
  StateEffect,
  StateField,
  Prec,
  type EditorState,
  type Extension,
} from "@codemirror/state";
import {
  Decoration,
  EditorView,
  ViewPlugin,
  WidgetType,
  keymap,
  type ViewUpdate,
} from "@codemirror/view";
import { completionStatus } from "@codemirror/autocomplete";
import type { KernelClient, Event } from "../../kernel/client";
import type { Dialect } from "../../kernel/generated/Dialect";
import type { ProfileConfig } from "../../kernel/generated/ProfileConfig";
export interface Ghost {
  pos: number;
  text: string;
}
export const setGhost = StateEffect.define<Ghost | null>();
class GhostWidget extends WidgetType {
  constructor(readonly text: string) {
    super();
  }
  eq(other: GhostWidget) {
    return this.text === other.text;
  }
  toDOM() {
    const node = document.createElement("span");
    node.className = "cm-ghost";
    node.textContent = this.text;
    node.setAttribute("aria-hidden", "true");
    return node;
  }
  ignoreEvent() {
    return true;
  }
}
/** Suggestion positions and text use CodeMirror's UTF16 source coordinates. */
export const ghostText = StateField.define<Ghost | null>({
  create: () => null,
  update: (value, tr) => {
    if (value && tr.docChanged) {
      let insert = "",
        count = 0,
        compatible = true;
      tr.changes.iterChanges((from, to, _fromB, _toB, text) => {
        count++;
        if (from !== value?.pos || to !== from) compatible = false;
        insert = text.toString();
      });
      value =
        compatible &&
        count === 1 &&
        insert.length > 0 &&
        value.text.startsWith(insert)
          ? {
              pos: value.pos + insert.length,
              text: value.text.slice(insert.length),
            }
          : null;
    }
    for (const effect of tr.effects)
      if (effect.is(setGhost)) value = effect.value;
    const selection = tr.newSelection.main;
    return value &&
      value.text.length > 0 &&
      selection.empty &&
      selection.head === value.pos
      ? value
      : null;
  },
  provide: (field) =>
    EditorView.decorations.from(field, (value) =>
      value
        ? Decoration.set([
            Decoration.widget({
              widget: new GhostWidget(value.text),
              side: 1,
            }).range(value.pos),
          ])
        : Decoration.none,
    ),
});
/** Advance one actual word (or leading punctuation/space plus the next word). */
export function partialLength(text: string) {
  return (
    /^\s*[^\p{L}\p{N}_\s]+\s*/u.exec(text)?.[0].length ??
    /^\s*[\p{L}\p{N}_]+/u.exec(text)?.[0].length ??
    Array.from(text)[0]?.length ??
    0
  );
}
export function eligiblePosition(
  state: EditorState,
  focused: boolean,
  configured: boolean,
  busy: boolean,
  localOpen: boolean,
) {
  const range = state.selection.main;
  return (
    focused &&
    configured &&
    !busy &&
    !localOpen &&
    range.empty &&
    range.head === state.doc.lineAt(range.head).to &&
    state.doc.lineAt(range.head).text.trim().length >= 3
  );
}
export interface GhostCandidate {
  source: string;
  pos: number;
  dialect: Dialect;
  scope: object;
  profile: ProfileConfig;
}
interface Job {
  id: string;
  candidate: GhostCandidate;
  buffer: string;
  cancelled: boolean;
  ackPending: boolean;
}
/** One editor's cancelable provider lifecycle, independent of rendering. */
export class GhostSession {
  private alive = true;
  private timer: ReturnType<typeof setTimeout> | undefined;
  private revision = 0;
  private job: Job | null = null;
  private off: () => void;
  authorizing = false;
  constructor(
    private kernel: KernelClient,
    private read: () => GhostCandidate | null,
    private publish: (ghost: Ghost | null) => void,
    private authorize: (candidate: GhostCandidate) => Promise<boolean>,
    private failed: (message: string) => void,
  ) {
    this.off = kernel.onEvent((e) => this.event(e));
  }
  private matches(a: GhostCandidate, b: GhostCandidate | null) {
    return (
      !!b &&
      a.source === b.source &&
      a.pos === b.pos &&
      a.dialect === b.dialect &&
      a.scope === b.scope
    );
  }
  cancel(clear = true) {
    this.revision++;
    if (this.timer) clearTimeout(this.timer);
    this.timer = undefined;
    const job = this.job;
    this.job = null;
    if (job) {
      job.cancelled = true;
      void this.kernel
        .request({ type: "llm_cancel", request_id: job.id })
        .catch(() => {});
    }
    if (clear) this.publish(null);
  }
  changed(keepGhost: boolean) {
    this.cancel(!keepGhost);
    if (!keepGhost && this.alive)
      this.timer = setTimeout(() => {
        this.timer = undefined;
        void this.start();
      }, 350);
  }
  revalidate(clear = true) {
    if (this.authorizing) return;
    if (this.job && !this.matches(this.job.candidate, this.read()))
      this.cancel(clear);
    else if (!this.read()) this.cancel(clear);
  }
  private async start() {
    const candidate = this.read();
    if (!this.alive || !candidate) return;
    const revision = this.revision;
    this.authorizing = true;
    let allowed: boolean;
    try {
      allowed = await this.authorize(candidate);
    } catch {
      allowed = false;
    } finally {
      this.authorizing = false;
    }
    if (
      !allowed ||
      !this.alive ||
      revision !== this.revision ||
      !this.matches(candidate, this.read())
    )
      return;
    const job: Job = {
      id: crypto.randomUUID(),
      candidate,
      buffer: "",
      cancelled: false,
      ackPending: true,
    };
    this.job = job;
    try {
      const response = await this.kernel.request({
        type: "llm_complete",
        request_id: job.id,
        prefix: candidate.source.slice(0, candidate.pos),
        suffix: candidate.source.slice(candidate.pos),
        dialect: candidate.dialect,
      });
      job.ackPending = false;
      if (job.cancelled || !this.alive) {
        if (response.type === "llm_started")
          void this.kernel
            .request({ type: "llm_cancel", request_id: job.id })
            .catch(() => {});
        return;
      }
      if (this.job !== job) return;
      if (response.type !== "llm_started" || response.request_id !== job.id) {
        this.job = null;
        this.failed(
          response.type === "error"
            ? response.message
            : "Completion request failed",
        );
      }
    } catch {
      job.ackPending = false;
      if (this.alive && this.job === job) {
        this.job = null;
        this.failed("Completion request failed");
      }
    }
  }
  private event(event: Event) {
    if (!this.alive) return;
    if (event.type === "kernel_restarted") {
      this.cancel();
      return;
    }
    const job = this.job;
    if (!job || !("request_id" in event) || event.request_id !== job.id) return;
    if (!this.matches(job.candidate, this.read())) {
      this.cancel();
      return;
    }
    if (event.type === "llm_delta") {
      if (job.buffer.length + event.text.length > 1_048_576) {
        this.cancel();
        this.failed("Completion exceeds limit");
        return;
      }
      job.buffer += event.text;
    } else if (event.type === "llm_done") {
      this.job = null;
      if (job.buffer && !/[\r\n]/.test(job.buffer))
        this.publish({ pos: job.candidate.pos, text: job.buffer });
    } else if (event.type === "llm_error") {
      this.job = null;
      this.failed(event.message);
    }
  }
  destroy() {
    this.cancel();
    this.alive = false;
    this.off();
  }
}
export interface GhostOptions {
  kernel: KernelClient;
  read: (view: EditorView) => {
    profile: ProfileConfig;
    dialect: Dialect;
    scope: object;
    busy: boolean;
  } | null;
  authorize: (candidate: GhostCandidate) => Promise<boolean>;
  failed: (message: string) => void;
}
/** Explicit acceptance writes actual insertion text, never executes it. */
export function acceptGhost(view: EditorView, partial = false) {
  const value = view.state.field(ghostText, false);
  if (!value || completionStatus(view.state) !== null) return false;
  const count = partial ? partialLength(value.text) : value.text.length;
  const pos = value.pos + count,
    text = value.text.slice(count);
  view.dispatch({
    changes: { from: value.pos, insert: value.text.slice(0, count) },
    selection: EditorSelection.cursor(pos),
    effects: setGhost.of(text ? { pos, text } : null),
  });
  return true;
}
export function ghostExtension(options: GhostOptions): {
  extension: Extension;
  cancel: (view: EditorView) => void;
} {
  const plugin = ViewPlugin.define((view) => {
    let alive = true;
    const read = () => {
      const config = options.read(view);
      if (
        !config ||
        view.composing ||
        !eligiblePosition(
          view.state,
          view.hasFocus,
          true,
          config.busy,
          completionStatus(view.state) !== null,
        )
      )
        return null;
      return {
        source: view.state.doc.toString(),
        pos: view.state.selection.main.head,
        dialect: config.dialect,
        scope: config.scope,
        profile: config.profile,
      };
    };
    const session = new GhostSession(
      options.kernel,
      read,
      (value) => {
        const snapshot = value ? read() : null;
        queueMicrotask(() => {
          const current = value ? read() : null;
          if (
            alive &&
            (!value ||
              (snapshot &&
                current &&
                snapshot.source === current.source &&
                snapshot.pos === current.pos &&
                snapshot.scope === current.scope &&
                snapshot.dialect === current.dialect))
          )
            view.dispatch({ effects: setGhost.of(value) });
        });
      },
      options.authorize,
      options.failed,
    );
    return {
      session,
      update: (update: ViewUpdate) => {
        if (update.docChanged) {
          session.changed(update.state.field(ghostText) !== null);
        } else if (
          update.selectionSet ||
          update.focusChanged ||
          completionStatus(update.state) !== completionStatus(update.startState)
        )
          session.revalidate(completionStatus(update.state) !== "pending");
      },
      destroy: () => {
        alive = false;
        session.destroy();
      },
    };
  });
  const cancel = (view: EditorView) => view.plugin(plugin)?.session.cancel();
  return {
    cancel,
    extension: [
      ghostText,
      plugin,
      Prec.highest(
        keymap.of([
          { key: "Tab", run: (view) => acceptGhost(view) },
          { key: "Mod-ArrowRight", run: (view) => acceptGhost(view, true) },
          {
            key: "Escape",
            run: (view) => {
              const present = view.state.field(ghostText) !== null;
              cancel(view);
              return present && completionStatus(view.state) === null;
            },
          },
        ]),
      ),
    ],
  };
}
