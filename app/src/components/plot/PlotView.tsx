import { useEffect, useRef, useState, type ReactNode } from "react";
import type { KernelClient } from "../../kernel/client";
import type { PlotRequest } from "../../kernel/generated/PlotRequest";
import type { PlotData } from "../../kernel/generated/PlotData";
import type { Messages } from "../../i18n";
import {
  axisScale,
  zoomRange,
  panRange,
  validRange,
  formatCoordinate,
} from "./scale";
import { ArtifactButton } from "../output/ArtifactButton";
import { PlotGeometry, margins, plotHeight, type Viewport } from "./geometry";
import { usePlotSampler } from "./usePlotSampler";
import { GraphFooter } from "../output/GraphFooter";
import { Sliders } from "./sliders";
import { ToolbarMenu } from "../ToolbarMenu";
import { WorkbenchIcon } from "../WorkbenchIcon";
import { plotCsvArtifact } from "./artifacts";
export interface PlotProps {
  request: PlotRequest;
  data?: PlotData;
  kernel: KernelClient;
  t: Messages;
  active?: boolean;
  exportParameters?: Record<string,number>;
  renderStatus?: ((actions: ReactNode) => ReactNode) | undefined;
}
const identities = new WeakMap<object, number>();
let nextIdentity = 0;
function identity(value: object | undefined) {
  if (!value) return 0;
  let id = identities.get(value);
  if (id === undefined) {
    id = ++nextIdentity;
    identities.set(value, id);
  }
  return id;
}
/** Source-keyed actual interactive plot; geometry never evaluates mathematics. */
export function PlotView(props: PlotProps) {
  return (
    <InteractivePlot
      key={`${identity(props.request)}:${identity(props.data)}:${props.active ?? true}`}
      {...props}
    />
  );
}
function InteractivePlot({
  request,
  data,
  kernel,
  t,
  active = true,
  exportParameters = {},
  renderStatus,
}: PlotProps) {
  const initial: Viewport = {
    x: data?.x_range ?? request.x_range,
    y: data?.y_range ?? request.y_range ?? [-5, 5],
  };
  const axisMode=data?.scale ?? request.options?.scale ?? 'linear';
  const logX=axisMode==='log_x'||axisMode==='log_log',logY=axisMode==='log_y'||axisMode==='log_log';
  const [view, setView] = useState(initial),
    viewRef = useRef(initial);
  const [params, setParams] = useState({ ...request.params }),
    paramsRef = useRef({ ...request.params });
  const [showData, setShowData] = useState(false);
  const [exportStatus,setExportStatus] = useState<string|null>(null);
  const [cursor, setCursor] = useState<[number, number] | null>(null);
  const [width, setWidth] = useState(640);
  const root = useRef<HTMLElement>(null),
    svg = useRef<SVGSVGElement>(null);
  const manual = useRef(false);
  const drag = useRef<{
    id: number;
    point: [number, number];
    view: Viewport;
  } | null>(null);
  const sampler = usePlotSampler(
    kernel,
    request,
    data,
    active,
    t.plotError,
    (result) => {
      if (validRange(result.x_range) && validRange(result.y_range)) {
        const next = { x: result.x_range, y: result.y_range };
        viewRef.current = next;
        setView(next);
      }
    },
  );
  const sample = (
    next: Viewport,
    values: Record<string, number>,
    mode: "view" | "parameter" | "now",
  ) =>
    sampler.sample(
      {
        ...request,
        x_range: next.x,
        y_range: manual.current ? next.y : request.y_range,
        params: values,
      },
      mode,
    );
  const changeView = (next: Viewport, mode: "view" | "now" = "view") => {
    if (!active || !validRange(next.x) || !validRange(next.y)) return;
    manual.current = true;
    viewRef.current = next;
    setView(next);
    sample(next, paramsRef.current, mode);
  };
  const reset = () => {
    if (!active) return;
    manual.current = false;
    viewRef.current = initial;
    setView(initial);
    sample(initial, paramsRef.current, "now");
  };
  const point = (clientX: number, clientY: number): [number, number] => {
    const box = svg.current?.getBoundingClientRect();
    return box && box.width > 0 && box.height > 0
      ? [
          ((clientX - box.left) / box.width) * width,
          ((clientY - box.top) / box.height) * plotHeight,
        ]
      : [width / 2, plotHeight / 2];
  };
  const zoom = (factor: number, p: [number, number] = [width / 2, plotHeight / 2]) => {
    const v = viewRef.current;
    const fx = Math.max(
        0,
        Math.min(
          1,
          (p[0] - margins.left) / (width - margins.left - margins.right),
        ),
      ),
      fy = Math.max(
        0,
        Math.min(
          1,
          (plotHeight - margins.bottom - p[1]) / (plotHeight - margins.top - margins.bottom),
        ),
      );
    changeView({
      x: zoomRange(v.x, fx, factor,logX),
      y: zoomRange(v.y, fy, factor,logY),
    });
  };
  const wheel = useRef<(e: WheelEvent) => void>(() => {});
  wheel.current = (e) => {
    if (!active) return;
    e.preventDefault();
    const pixels =
      e.deltaY * (e.deltaMode === 1 ? 16 : e.deltaMode === 2 ? plotHeight : 1);
    zoom(
      Math.exp(Math.max(-1.4, Math.min(1.4, pixels * 0.002))),
      point(e.clientX, e.clientY),
    );
  };
  useEffect(() => {
    const element = svg.current;
    const fn = (e: WheelEvent) => wheel.current(e);
    element?.addEventListener("wheel", fn, { passive: false });
    return () => element?.removeEventListener("wheel", fn);
  }, []);
  useEffect(() => {
    const element = root.current;
    if (!element) return;
    const measure = () => {
      const w = element.getBoundingClientRect().width;
      if (w > 0) setWidth(Math.max(240, w));
    };
    measure();
    const observer =
      typeof ResizeObserver !== "undefined"
        ? new ResizeObserver(measure)
        : null;
    observer?.observe(element);
    window.addEventListener("resize", measure);
    return () => {
      observer?.disconnect();
      window.removeEventListener("resize", measure);
    };
  }, []);
  const realCursor = cursor
    ? [
        axisScale(view.x, [margins.left, width - margins.right],logX).invert(
          cursor[0],
        ),
        axisScale(view.y, [plotHeight - margins.bottom, margins.top],logY).invert(
          cursor[1],
        ),
      ]
    : null;
  const geometry = sampler.data?.geometry;
  const pointCount = (sampler.data?.curves.reduce((n,c)=>n+c.segments.reduce((m,s)=>m+s.length,0),0) ?? 0)
    + (geometry?.points.length ?? 0) + (geometry?.markers?.length ?? 0)
    + (geometry?.paths?.reduce((n,p)=>n+p.points.length,0) ?? 0)
    + (geometry?.polygons?.reduce((n,p)=>n+p.points.length,0) ?? 0);
  return (
    <figure
      className={`plot-view sampled-preview ${sampler.pending || sampler.error ? "plot-pending" : ""}`}
      ref={root}
      aria-busy={sampler.pending}
    >
      <div className="plot-stage">
        <div className="graph-controls" role="group" aria-label={t.graphControls}>
          <button
            disabled={!active}
            aria-label={t.zoomIn}
            onClick={() => zoom(0.8)}
          >
            ＋
          </button>
          <button
            disabled={!active}
            aria-label={t.zoomOut}
            onClick={() => zoom(1.25)}
          >
            −
          </button>
          <button disabled={!active} onClick={reset} aria-label={t.resetView} title={t.resetView}>
            <WorkbenchIcon name="reset" />
          </button>
        </div>
      <svg
        ref={svg}
        width="100%"
        height={plotHeight}
        viewBox={`0 0 ${width} ${plotHeight}`}
        role="img"
        aria-label={t.plot}
        tabIndex={active ? 0 : -1}
        aria-description={`${sampler.data?.curves.length ?? request.exprs.length} ${t.graphCurves}; x ${view.x.map(formatCoordinate).join(' … ')}. ${t.plotControls}`}
        onDoubleClick={reset}
        onPointerDown={(e) => {
          if (!active || e.button !== 0) return;
          e.currentTarget.setPointerCapture?.(e.pointerId);
          drag.current = {
            id: e.pointerId,
            point: point(e.clientX, e.clientY),
            view: viewRef.current,
          };
        }}
        onPointerMove={(e) => {
          const p = point(e.clientX, e.clientY);
          setCursor(
            p[0] >= margins.left &&
              p[0] <= width - margins.right &&
              p[1] >= margins.top &&
              p[1] <= plotHeight - margins.bottom
              ? p
              : null,
          );
          const start = drag.current;
          if (!start || start.id !== e.pointerId) return;
          changeView({
            x: panRange(
              start.view.x,
              -(p[0] - start.point[0]) / (width - margins.left - margins.right),
              logX,
            ),
            y: panRange(
              start.view.y,
              (p[1] - start.point[1]) / (plotHeight - margins.top - margins.bottom),
              logY,
            ),
          });
        }}
        onPointerUp={(e) => {
          if (drag.current?.id === e.pointerId) {
            drag.current = null;
            if (e.currentTarget.hasPointerCapture?.(e.pointerId))
              e.currentTarget.releasePointerCapture?.(e.pointerId);
          }
        }}
        onPointerCancel={() => {
          drag.current = null;
        }}
        onPointerLeave={() => {
          if (!drag.current) setCursor(null);
        }}
        onKeyDown={(e) => {
          const v = viewRef.current;
          if (
            [
              "ArrowLeft",
              "ArrowRight",
              "ArrowUp",
              "ArrowDown",
              "+",
              "=",
              "-",
              "Home",
            ].includes(e.key)
          ) {
            e.preventDefault();
            if (e.key === "Home") reset();
            else if (e.key === "+" || e.key === "=") zoom(0.8);
            else if (e.key === "-") zoom(1.25);
            else
              changeView({
                x: panRange(
                  v.x,
                  e.key === "ArrowRight"
                    ? 0.1
                    : e.key === "ArrowLeft"
                      ? -0.1
                      : 0,
                  logX,
                ),
                y: panRange(
                  v.y,
                  e.key === "ArrowUp" ? 0.1 : e.key === "ArrowDown" ? -0.1 : 0,
                  logY,
                ),
              });
          }
        }}
      >
        <PlotGeometry
          data={sampler.data}
          request={request}
          view={view}
          width={width}
          cursor={cursor}
        />
      </svg>
      <div className="plot-readout">
        <span>
          {t.coordinates}:{" "}
          {realCursor
            ? `(${formatCoordinate(realCursor[0]!)}, ${formatCoordinate(realCursor[1]!)})`
            : "—"}
        </span>
        <span role="status">
          {sampler.pending ? t.sampling : !active ? t.stale : ""}
        </span>
      </div>
      </div>
      <figcaption className="plot-legend">
        {(
          sampler.data?.curves ?? request.exprs.map((label) => ({ label }))
        ).map((c, i) => (
          <span key={i}>
            <i style={{ background: request.options?.color ?? `var(--plot-${(i % 6) + 1})` }} />
            <code>{c.label}</code>
          </span>
        ))}
      </figcaption>
      {sampler.error && (
        <div role="alert" className="output-error">
          {sampler.error}
          <button
            disabled={!active}
            onClick={() => sample(viewRef.current, paramsRef.current, "now")}
          >
            {t.retry}
          </button>
        </div>
      )}
      <GraphFooter t={t} renderStatus={renderStatus} status={exportStatus} information={<>
            <p>{sampler.data?.curves.length ?? 0} {t.graphCurves} · {t.samplePoints}: {pointCount}</p>
            {geometry && (geometry.tiles.length > 0 || geometry.arrows.length > 0) && <p>{t.graphTiles}: {geometry.tiles.length} · {t.graphVectors}: {geometry.arrows.length}</p>}
            {sampler.data && <p>{t.graphRange}: x [{sampler.data.x_range.map(formatCoordinate).join(' … ')}], y [{sampler.data.y_range.map(formatCoordinate).join(' … ')}]</p>}
            {request.kind !== "Data" && request.kind !== "Histogram" && request.kind !== "Scene" && <p>{t.plotApproximation}</p>}
            {sampler.data?.geometry && <p>{t.skippedSamples}: {sampler.data.geometry.skipped}</p>}
            <details onToggle={event=>setShowData(event.currentTarget.open)}>
              <summary>{t.plotData}</summary>
              {showData && <pre className="plot-sample-data">{JSON.stringify(sampler.data,null,2)}</pre>}
            </details>
        </>} exports={<ToolbarMenu label={t.graphExport} icon={<>{t.graphExport}<WorkbenchIcon name="chevron" /></>} className="graph-export">
        {(['svg','png'] as const).map(format=><ArtifactButton key={format} onStatus={setExportStatus} kernel={kernel} t={t} label={format==='svg'?t.exportSvg:t.exportPng}
          title="OpenMath-plot" version={sampler.data} enabled={active&&!sampler.pending&&!sampler.error&&sampler.data!==null}
          request={()=>sampler.data?{type:'export_plot',format,figure:{data:sampler.data,axis_x:request.kind==='Parametric'?'x':request.var_x,axis_y:request.var_y??'y',title:'OpenMath',color:request.options?.color??null,region:request.kind==='Region',parameters:{...paramsRef.current,...exportParameters},width:1000,height:600}}:undefined}/>)}
          <ArtifactButton onStatus={setExportStatus} kernel={kernel} t={t} title="OpenMath-plot" label={t.exportCsv}
            version={sampler.data} enabled={active&&!sampler.pending&&!sampler.error&&sampler.data!==null}
            createArtifact={()=>plotCsvArtifact(sampler.data!)} />
        </ToolbarMenu>} />
      <Sliders
        request={request}
        values={params}
        disabled={!active}
        onChange={(name, value) => {
          if (!active || !Number.isFinite(value)) return;
          const values = { ...paramsRef.current, [name]: value };
          paramsRef.current = values;
          setParams(values);
          sample(viewRef.current, values, "parameter");
        }}
      />
    </figure>
  );
}
