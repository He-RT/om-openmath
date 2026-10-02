import { useId } from "react";
import type { PlotData } from "../../kernel/generated/PlotData";
import type { PlotRequest } from "../../kernel/generated/PlotRequest";
import { formatCoordinate, linearScale, niceTicks, type Range } from "./scale";
export interface Viewport {
  x: Range;
  y: Range;
}
export const margins = { left: 50, right: 20, top: 18, bottom: 36 };
export function PlotGeometry({
  data,
  request,
  view,
  width,
  cursor,
}: {
  data: PlotData | null;
  request: PlotRequest;
  view: Viewport;
  width: number;
  cursor: [number, number] | null;
}) {
  const id = `plot-${useId().replace(/:/g, "")}`;
  const { left, right, top, bottom } = margins,
    edge = width - right,
    floor = 320 - bottom;
  const x = linearScale(view.x, [left, edge]),
    y = linearScale(view.y, [floor, top]);
  const xs = niceTicks(view.x, Math.max(2, Math.floor((edge - left) / 85))),
    ys = niceTicks(view.y, 5);
  const highlights = data?.highlights ?? {
    points: request.points,
    shade: request.shade,
  };
  const coordinate = (value: number) =>
    Number.isFinite(value)
      ? Math.max(-1e6, Math.min(1e6, value))
      : value > 0
        ? 1e6
        : -1e6;
  return (
    <>
      <defs>
        <clipPath id={id}>
          <rect x={left} y={top} width={edge - left} height={floor - top} />
        </clipPath>
      </defs>
      <g className="plot-grid">
        {xs.map((v, i) => (
          <line key={`x${i}`} x1={x.map(v)} x2={x.map(v)} y1={top} y2={floor} />
        ))}
        {ys.map((v, i) => (
          <line key={`y${i}`} x1={left} x2={edge} y1={y.map(v)} y2={y.map(v)} />
        ))}
      </g>
      <g className="plot-ticks">
        {xs.map((v, i) => (
          <text key={`x${i}`} x={x.map(v)} y={floor + 20} textAnchor="middle">
            {formatCoordinate(v)}
          </text>
        ))}
        {ys.map((v, i) => (
          <text key={`y${i}`} x={left - 9} y={y.map(v) + 3} textAnchor="end">
            {formatCoordinate(v)}
          </text>
        ))}
      </g>
      <g clipPath={`url(#${id})`}>
        {highlights.shade.map(([lo, hi], i) => {
          const a = Math.max(lo, view.x[0]),
            b = Math.min(hi, view.x[1]);
          return a <= b ? (
            <rect
              key={i}
              className="plot-shade"
              x={x.map(a)}
              y={top}
              width={x.map(b) - x.map(a)}
              height={floor - top}
            />
          ) : null;
        })}
        {view.x[0] <= 0 && view.x[1] >= 0 && (
          <line
            className="plot-axis"
            x1={x.map(0)}
            x2={x.map(0)}
            y1={top}
            y2={floor}
          />
        )}
        {view.y[0] <= 0 && view.y[1] >= 0 && (
          <line
            className="plot-axis"
            x1={left}
            x2={edge}
            y1={y.map(0)}
            y2={y.map(0)}
          />
        )}
        {data?.curves.map((curve, index) => (
          <g className="plot-curve sampled-curve" key={index}>
            <title>{curve.label}</title>
            {curve.segments.map((segment, i) => (
              <path
                key={i}
                style={{ stroke: `var(--plot-${(index % 6) + 1})` }}
                d={segment
                  .map(
                    ([a, b], j) =>
                      `${j ? "L" : "M"}${coordinate(x.map(a))} ${coordinate(y.map(b))}`,
                  )
                  .join(" ")}
              />
            ))}
          </g>
        ))}
        {highlights.points
          .filter(
            ([a, b]) =>
              a >= view.x[0] &&
              a <= view.x[1] &&
              b >= view.y[0] &&
              b <= view.y[1],
          )
          .map(([a, b], i) => (
            <g key={i} className="plot-solution">
              <circle
                className="solution-point"
                cx={x.map(a)}
                cy={y.map(b)}
                r="4"
              >
                <title>
                  ({a}, {b})
                </title>
              </circle>
              <text
                className="plot-point-label"
                x={x.map(a) + 8}
                y={y.map(b) - 10}
              >
                ({formatCoordinate(a)}, {formatCoordinate(b)})
              </text>
            </g>
          ))}
        {cursor && (
          <g className="plot-crosshair">
            <line x1={cursor[0]} x2={cursor[0]} y1={top} y2={floor} />
            <line x1={left} x2={edge} y1={cursor[1]} y2={cursor[1]} />
          </g>
        )}
      </g>
      <text className="plot-axis-name" x={edge} y={313} textAnchor="end">
        {request.var_x}
      </text>
      <text className="plot-axis-name" x={left} y={12}>
        {request.var_y ?? "y"}
      </text>
      <rect
        className="plot-frame"
        x={left}
        y={top}
        width={edge - left}
        height={floor - top}
      />
    </>
  );
}
