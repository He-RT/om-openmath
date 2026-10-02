import type { IntervalView } from "../../kernel/generated/IntervalView";
import type { Messages } from "../../i18n";
import { Katex } from "./Katex";
export function NumberLine({
  intervals,
  variable,
  t,
}: {
  intervals: IntervalView[];
  variable: string;
  t: Messages;
}) {
  // Missing symbolic coordinates are kept as conditions, never plotted as infinity.
  const drawable = intervals.filter(
    (i) =>
      (i.lo === null || (i.lo_value !== null && Number.isFinite(i.lo_value))) &&
      (i.hi === null || (i.hi_value !== null && Number.isFinite(i.hi_value))),
  );
  const endpoints = drawable
    .flatMap((i) => [i.lo_value, i.hi_value])
    .filter((x): x is number => x !== null && Number.isFinite(x));
  const magnitude = endpoints.length
    ? Math.max(...endpoints.map(Math.abs)) || 1
    : 1;
  const normalized = endpoints.map((v) => v / magnitude);
  let lo = Math.min(...normalized, 0),
    hi = Math.max(...normalized, 0);
  const pad = Math.max((hi - lo) * 0.18, 0.2);
  lo -= pad;
  hi += pad;
  const x = (v: number) => 32 + ((v / magnitude - lo) / (hi - lo)) * 416;
  const raw = (hi - lo) / 6,
    base = 10 ** Math.floor(Math.log10(raw)),
    step =
      (raw / base <= 1 ? 1 : raw / base <= 2 ? 2 : raw / base <= 5 ? 5 : 10) *
      base;
  const ticks: number[] = [];
  for (
    let v = Math.ceil(lo / step) * step;
    v <= hi && ticks.length < 12;
    v += step
  )
    ticks.push(v * magnitude);
  const label = (v: number) => Number(v.toPrecision(5)).toString();
  return (
    <div className="number-line">
      {drawable.length > 0 && (
        <svg
          viewBox="0 0 480 94"
          role="img"
          aria-label={`${t.numberLine} ${variable}`}
        >
          <line className="number-axis" x1="20" x2="460" y1="42" y2="42" />
          {ticks.filter(Number.isFinite).map((v, i) => (
            <g className="number-tick" key={i}>
              <line x1={x(v)} x2={x(v)} y1="37" y2="47" />
              <text x={x(v)} y="72" textAnchor="middle">
                {label(v)}
              </text>
            </g>
          ))}
          {drawable.map((i, index) => {
            const left = i.lo === null ? 22 : x(i.lo_value!),
              right = i.hi === null ? 458 : x(i.hi_value!);
            return (
              <g key={index}>
                <line
                  className="interval-segment"
                  x1={left}
                  x2={right}
                  y1="42"
                  y2="42"
                />
                {i.lo === null ? (
                  <path className="infinite-arrow" d="M28 35 L20 42 L28 49" />
                ) : (
                  <circle
                    className={
                      i.lo_closed ? "endpoint-closed" : "endpoint-open"
                    }
                    cx={left}
                    cy="42"
                    r="5"
                  >
                    <title>{i.lo}</title>
                  </circle>
                )}
                {i.hi === null ? (
                  <path
                    className="infinite-arrow"
                    d="M452 35 L460 42 L452 49"
                  />
                ) : (
                  <circle
                    className={
                      i.hi_closed ? "endpoint-closed" : "endpoint-open"
                    }
                    cx={right}
                    cy="42"
                    r="5"
                  >
                    <title>{i.hi}</title>
                  </circle>
                )}
              </g>
            );
          })}
          <text x="472" y="46" textAnchor="end" className="axis-variable">
            {variable}
          </text>
        </svg>
      )}
      <div className="interval-labels">
        {intervals.map((i, index) => (
          <Katex
            key={index}
            latex={`${i.lo_closed ? "[" : "("}${i.lo ?? "-\\infty"},\\,${i.hi ?? "+\\infty"}${i.hi_closed ? "]" : ")"}`}
          />
        ))}
      </div>
      {drawable.length < intervals.length && (
        <span className="muted">{t.symbolicLine}</span>
      )}
    </div>
  );
}
