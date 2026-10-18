import type { CellOutput } from "../../kernel/generated/CellOutput";
import type { Messages, Locale } from "../../i18n";
import type { KernelClient } from "../../kernel/client";
import { SceneView, type CameraMemory } from "../scene/SceneView";
import { ExprView } from "./ExprView";
import { SolutionLine } from "./SolutionLine";
import { deriveCellPresentation, outputFacts, type CellPresentation } from "../cellPresentation";
import { CellStatusLine } from "../CellStatusLine";
import { Messages as OutputMessages } from "./Messages";
import { PlotView } from "../plot/PlotView";
import { ExploreView } from "../explore/ExploreView";
import { ValueView } from './ValueView';
import type { Span } from "../../kernel/generated/Span";
import { fromByte } from "../editor/positions";
import type { ReactNode } from "react";
export function OutputView({
  output,
  stale,
  t,
  language,
  kernel,
  onInsert,
  onSteps,
  cellId = '',
  exportParameters = {},
  sceneCameraMemory,
  presentation,
  onRun,
  source,
  onLocate,
}: {
  output: CellOutput;
  stale: boolean;
  t: Messages;
  language: Locale;
  kernel: KernelClient;
  onInsert: (source: string) => void;
  onSteps: (outIndex: number) => void;
  cellId?: string;
  exportParameters?: Record<string,number>;
  sceneCameraMemory?: CameraMemory;
  presentation?: CellPresentation;
  onRun?: (() => void) | undefined;
  source?: string;
  onLocate?: (span: Span) => void;
}) {
  return (
    <div
      className={`cell-output ${stale ? "output-stale" : ""}`}
      aria-label={t.allOutputs}
    >
      {output.items.map((item, i) => {
        const state = stale ? presentation ?? {status:"stale" as const,byAgent:false} : presentation?.status === "stopped" ? presentation : deriveCellPresentation(outputFacts({items:[item],messages:[],timing_ms:0}));
        const renderStatus=(actions:ReactNode)=><CellStatusLine presentation={state} t={t} timing={i===output.items.length-1 ? output.timing_ms:undefined} onRun={onRun} actions={actions}/>;
        return (
        <div className="output-item" key={i}>
          {item.type === "scene3_d" ? <SceneView item={item} kernel={kernel} t={t} language={language} active={!stale} cameraMemory={sceneCameraMemory}/> : item.type === "explore" ? <ExploreView key={`${cellId}:${item.out_index}:${item.view_id}`} item={item} cellId={cellId} fresh={!stale} kernel={kernel} t={t} language={language} onInsert={onInsert}/> : item.type === "expr" ? (
            item.presentation ? <ValueView key={`${item.out_index}:${item.presentation.view_id}`}
              initial={item.presentation} cellId={cellId} outIndex={item.out_index} value={item}
              fresh={!stale} kernel={kernel} language={language} t={t} onInsert={onInsert} renderStatus={renderStatus} /> : <ExprView
              key={`${item.out_index}:${item.input_form}`}
              value={item}
              active={!stale}
              kernel={kernel}
              t={t}
              onInsert={onInsert}
              renderStatus={renderStatus}
            />
          ) : item.type === "solutions" ? (
            <SolutionLine
              key={`${item.out_index}:${item.input_form}`}
              item={item}
              kernel={kernel}
              t={t}
              onInsert={onInsert}
              onSteps={() => onSteps(item.out_index)}
              active={!stale}
              presentation={state}
              timing={i === output.items.length - 1 ? output.timing_ms : undefined}
              onRun={onRun}
            />
          ) : item.type === "error" ? (
            <div role="alert" className="output-error output-diagnostic">
              {source !== undefined && item.span && <code>{source.slice(0,fromByte(source,item.span.start)).split("\n").length}:{Array.from(source.slice(0,fromByte(source,item.span.start)).split("\n").at(-1) ?? "").length+1} </code>}
              <span>{item.message}</span>
              {item.span && onLocate && <button disabled={stale} onClick={()=>onLocate(item.span!)}>{t.locate}</button>}
            </div>
          ) : (
            <PlotView
              request={item.request}
              data={item.data}
              kernel={kernel}
              t={t}
              active={!stale}
              exportParameters={exportParameters}
            />
          )}
          {item.type !== "solutions" && item.type !== "expr" && renderStatus(null)}
        </div>
      );})}
      <OutputMessages items={output.messages} t={t} language={language} collapseErrors={output.messages.filter(message=>message.level==="Error").every(message=>output.items.some(item=>item.type==="error" && (item.message===message.text || item.message===`${message.tag}: ${message.text}`)))} />
      {output.items.length === 0 && output.messages.length === 0 && (
        <div className="suppressed-output">{t.suppress}</div>
      )}
      {output.items.length===0 && <CellStatusLine presentation={presentation ?? deriveCellPresentation(outputFacts(output))} t={t} timing={output.timing_ms} onRun={onRun} />}
    </div>
  );
}
