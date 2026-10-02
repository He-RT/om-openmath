import { useEffect, useState } from "react";
import type { KernelClient } from "../../kernel/client";
import type { PlotRequest } from "../../kernel/generated/PlotRequest";
import type { PlotData } from "../../kernel/generated/PlotData";
import type { Messages } from "../../i18n";
/** Basic actual sampled preview; full viewport/slider interaction is M13.5. */
export function PlotPreview({
  request,
  kernel,
  t,
}: {
  request: PlotRequest;
  kernel: KernelClient;
  t: Messages;
}) {
  const [data, setData] = useState<PlotData | null>(null);
  const [failed, setFailed] = useState(false);
  useEffect(() => {
    let current = true;
    void kernel.request({ type: "sample_plot", request }).then(
      (response) => {
        if (!current) return;
        if (response.type === "plot") setData(response.data);
        else setFailed(true);
      },
      () => {
        if (current) setFailed(true);
      },
    );
    return () => {
      current = false;
    };
  }, [request, kernel]);
  if (failed)
    return (
      <p role="alert" className="output-error">
        {t.plotError}
      </p>
    );
  if (!data)
    return (
      <p role="status" className="muted">
        {t.sampling}
      </p>
    );
  return <SampledPreview data={data} request={request} t={t} />;
}
export function SampledPreview({
  data,
  request,
  t,
}: {
  data: PlotData;
  request: PlotRequest;
  t: Messages;
}) {
  const [xmin, xmax] = data.x_range,
    [ymin, ymax] = data.y_range;
  // Normalizing before subtraction keeps extreme finite ranges representable.
  const scale = (value: number, lo: number, hi: number, size: number) => {
    const magnitude = Math.max(Math.abs(lo), Math.abs(hi), 1);
    return (
      ((value / magnitude - lo / magnitude) /
        (hi / magnitude - lo / magnitude)) *
      size
    );
  };
  const x = (v: number) => scale(v, xmin, xmax, 480),
    y = (v: number) => 240 - scale(v, ymin, ymax, 240);
  if (
    ![xmin, xmax, ymin, ymax].every(Number.isFinite) ||
    xmin >= xmax ||
    ymin >= ymax
  )
    return <p role="alert">{t.plotError}</p>;
  const highlights = data.highlights ?? {
    points: request.points,
    shade: request.shade,
  };
  return (
    <figure className="sampled-preview">
      <svg viewBox="0 0 480 240" role="img" aria-label={t.plot}>
        <svg width="480" height="240" overflow="hidden">
          {highlights.shade.map(([lo, hi], i) => (
            <rect
              key={i}
              className="plot-shade"
              x={x(Math.max(lo, xmin))}
              y="0"
              width={Math.max(0, x(Math.min(hi, xmax)) - x(Math.max(lo, xmin)))}
              height="240"
            />
          ))}
          {xmin <= 0 && xmax >= 0 && (
            <line className="number-axis" x1={x(0)} x2={x(0)} y1="0" y2="240" />
          )}
          {ymin <= 0 && ymax >= 0 && (
            <line className="number-axis" x1="0" x2="480" y1={y(0)} y2={y(0)} />
          )}
          {data.curves.map((c, index) => (
            <g className="sampled-curve" key={index}>
              <title>{c.label}</title>
              {c.segments.map((segment, i) => (
                <path
                  key={i}
                  style={{
                    stroke: `var(--plot-${(index % 6) + 1}, var(--accent))`,
                  }}
                  d={segment
                    .map(([a, b], j) => `${j ? "L" : "M"}${x(a)} ${y(b)}`)
                    .join(" ")}
                />
              ))}
            </g>
          ))}
          {highlights.points
            .filter(
              ([a, b]) => a >= xmin && a <= xmax && b >= ymin && b <= ymax,
            )
            .map(([a, b], i) => (
              <circle
                className="solution-point"
                key={i}
                cx={x(a)}
                cy={y(b)}
                r="4"
              >
                <title>
                  ({a}, {b})
                </title>
              </circle>
            ))}
        </svg>
      </svg>
      <figcaption>
        {data.curves.map((c) => (
          <code key={c.label}>{c.label}</code>
        ))}
        <span>
          {request.var_x}: [{xmin}, {xmax}]
        </span>
      </figcaption>
    </figure>
  );
}
