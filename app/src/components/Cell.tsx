import { useState } from "react";
import type { NotebookController, UiCell } from "../state/notebookStore";
import type { Messages, Locale } from "../i18n";
import type { Dialect } from "../kernel/generated/Dialect";
import { MathEditor } from "./editor/MathEditor";
import { TextCell } from "./TextCell";
import { OutputView } from "./output/OutputView";
import { AskCell } from "./AskCell";
import { cellPresentation, presentationLabel } from "./cellPresentation";
import { CellStatusLine } from "./CellStatusLine";
import { ToolbarMenu } from "./ToolbarMenu";
import { WorkbenchIcon } from "./WorkbenchIcon";
import { RepairControl } from "./assistant/RepairControl";
interface Props {
  cell: UiCell;
  index: number;
  controller: NotebookController;
  t: Messages;
  language: Locale;
  settingsKey: string;
  active: boolean;
  focusKey: number;
  busy: boolean;
}
export function Cell({
  cell,
  index,
  controller,
  t,
  language,
  settingsKey,
  active,
  focusKey,
  busy,
}: Props) {
  const [selection, setSelection] = useState<{
    action: string;
    source: string;
    dialect: Dialect;
  } | null>(null);
  const [variable, setVariable] = useState("x");
  const [preview, setPreview] = useState(false);
  const [reveal,setReveal]=useState<{start:number;end:number;serial:number}|null>(null);
  const presentation = cellPresentation(cell, t);
  const selectedAction = (
    action: string,
    source: string,
    dialect: Dialect,
    axis = "x",
  ) => {
    const wl = dialect === "Wolfram";
    const name = action[0]?.toUpperCase() + action.slice(1);
    const code =
      action === "plot"
        ? wl
          ? `Plot[${source}, {${axis}, -5, 5}]`
          : `plot(${source}, [${axis}, -5, 5])`
        : action === "solve"
          ? wl
            ? `Solve[${source}, ${axis}]`
            : `solve(${source}, ${axis})`
          : wl
            ? `${name}[${source}]`
            : `${action}(${source})`;
    const id = controller.add("Math", code, cell.id);
    controller.edit(id, { dialect });
    void controller.run(id);
    setSelection(null);
  };
  return (
    <section
      className={`notebook-cell ${active ? "cell-active" : ""}`}
      data-cell-id={cell.id}
      data-status={cell.status}
      data-kind={cell.kind}
      data-presentation={presentation.status}
      aria-label={cell.kind==="Text" ? `${t.text} ${index+1}`:`${index + 1} · ${presentationLabel(presentation,t)}`}
      onMouseDown={() => controller.select(cell.id)}
      onFocus={() => controller.select(cell.id)}
    >
      <div className={`cell-rail rail-${presentation.status} ${presentation.byAgent?'rail-by-agent':''}`} aria-hidden="true" />
      <div className="cell-body">
        <div className={cell.kind === "Math" ? `source-well ${preview ? "show-source-preview" : ""}` : "cell-source"}>
          {cell.kind==="Math" && <span className="execution-count sr-only">[{cell.exec_count ?? " "}]</span>}
          <div className="cell-toolbar">
            {cell.kind === "Math" && <button className="run-cell" aria-label={`${presentation.status === "running" ? t.stop : t.run} ${index+1}`} disabled={busy && presentation.status !== "running"} onClick={() => void (presentation.status === "running" ? controller.interrupt() : controller.run(cell.id))}><WorkbenchIcon name={presentation.status === "running" ? "stop" : "play"}/></button>}
            <ToolbarMenu className="cell-more" label={`${t.cellMore} ${index+1}`} icon={<WorkbenchIcon name="more"/>}>
              <label>{t.math} / {t.text}<select aria-label={`${t.math} / ${t.text} ${index+1}`} value={cell.kind} onChange={event=>controller.edit(cell.id,{kind:event.target.value as UiCell["kind"]})}><option value="Math">{t.math}</option><option value="Text">{t.text}</option>{cell.kind==='Ask'&&<option value="Ask" disabled>{t.ask}</option>}</select></label>
              {cell.kind === "Math" && <><label>{t.dialect}<select aria-label={`${t.dialect} ${index+1}`} value={cell.dialect} onChange={event=>controller.edit(cell.id,{dialect:event.target.value as Dialect})}><option value="Auto">{t.auto}</option><option value="Modern">{t.modern}</option><option value="Wolfram">Wolfram</option></select></label><button aria-pressed={preview} onClick={()=>setPreview(value=>!value)}>{t.preview}</button></>}
              <button aria-label={`${t.moveUp} ${index+1}`} disabled={index===0} onClick={()=>controller.move(cell.id,-1)}>{t.moveUp}</button>
              <button aria-label={`${t.moveDown} ${index+1}`} disabled={index===controller.store.getState().cells.length-1} onClick={()=>controller.move(cell.id,1)}>{t.moveDown}</button>
              <button onClick={()=>controller.store.setState({panel:'assistant',assistantReferences:[cell.id]})}>{t.assistant==='助手'?'在助手中引用':'Reference in assistant'}</button>
              <button aria-label={`${t.delete} ${index+1}`} onClick={()=>controller.remove(cell.id)}>{t.delete}</button>
            </ToolbarMenu>
          </div>
        {cell.kind === "Math" ? (
          <MathEditor
            id={cell.id}
            index={index}
            source={cell.source}
            dialect={cell.dialect}
            settingsKey={settingsKey}
            client={controller.kernel}
            t={t}
            language={language}
            llm={controller.store.getState().config?.llm ?? null}
            busy={busy}
            focus={active}
            focusKey={focusKey}
            reveal={reveal}
            acceptedErrors={cell.resultSource?.source===cell.source ? cell.output?.items.filter(item=>item.type==="error") ?? []:[]}
            onChange={(source) => controller.edit(cell.id, { source })}
            onFocus={() => controller.select(cell.id)}
            onRun={(mode) => void controller.run(cell.id, mode)}
            onAction={(source) => {
              controller.edit(cell.id, { source });
              void controller.run(cell.id);
            }}
            onSelect={(action, source, dialect) => {
              if (action === "solve" || action === "plot") {
                setSelection({ action, source, dialect });
                setVariable(cell.uses[0] ?? "x");
              } else selectedAction(action, source, dialect);
            }}
          />
        ) : cell.kind === "Ask" ? (
          <AskCell
            key={cell.id}
            cell={cell}
            index={index}
            controller={controller}
            t={t}
            busy={busy}
          />
        ) : (
          <TextCell source={cell.source} index={index} t={t} editing={cell.textEditing===true} onModeChange={editing=>controller.textMode(cell.id,editing)} onFocus={()=>controller.select(cell.id)} onChange={source=>controller.edit(cell.id,{source})} onInsertCode={(source,dialect)=>{const id=controller.add("Math",source,cell.id);controller.edit(id,{dialect});}}/>
        )}
        </div>
        {cell.kind === "Math" && !cell.output && <CellStatusLine presentation={presentation} t={t} />}
        {cell.kind === "Math" && cell.output && (
          <OutputView
            cellId={cell.id}
            output={cell.output}
            presentation={presentation}
            onRun={!busy ? ()=>void controller.update(cell.id):undefined}
            source={cell.resultSource?.source ?? cell.source}
            onLocate={span=>setReveal(previous=>({...span,serial:(previous?.serial ?? 0)+1}))}
            stale={
              cell.status === "Stale" ||
              cell.status === "Running" ||
              cell.status === "Queued"
            }
            t={t}
            language={language}
            kernel={controller.kernel}
            onInsert={(source) => {
              const id = controller.add("Math", source, cell.id);
              controller.edit(id, { dialect: "Wolfram" });
            }}
            onSteps={(outIndex) => {
              controller.select(cell.id);
              controller.store.setState({
                panel: "steps",
                stepSelection: { cellId: cell.id, outIndex },
              });
            }}
          />
        )}
        {cell.kind === "Math" && cell.previousOutput && <div className="previous-result"><p>{t.previousResult}</p><OutputView output={cell.previousOutput} stale t={t} language={language} kernel={controller.kernel} onInsert={source=>{const id=controller.add("Math",source,cell.id);controller.edit(id,{dialect:"Wolfram"});}} onSteps={()=>{}} presentation={{status:"stale",byAgent:false}} /></div>}
        {cell.kind === "Math" && cell.status === "Error" && (
          <RepairControl
            key={`${cell.id}:${cell.revision}`}
            cell={cell}
            controller={controller}
            t={t}
            busy={busy}
          />
        )}
        {selection && (
          <form
            className="variable-prompt"
            onSubmit={(e) => {
              e.preventDefault();
              selectedAction(
                selection.action,
                selection.source,
                selection.dialect,
                variable,
              );
            }}
          >
            <label>
              {t.variable}
              <input
                autoFocus
                aria-label={t.variable}
                value={variable}
                onChange={(e) => setVariable(e.target.value)}
              />
            </label>
            <button type="submit">{t.apply}</button>
            <button type="button" onClick={() => setSelection(null)}>
              {t.close}
            </button>
          </form>
        )}
      </div>
    </section>
  );
}
