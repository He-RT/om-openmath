import type { PlotRequest } from "../../kernel/generated/PlotRequest";
import { formatCoordinate, validRange } from "./scale";
export function Sliders({
  request,
  values,
  onChange,
  disabled,
}: {
  request: PlotRequest;
  values: Record<string, number>;
  onChange: (name: string, value: number) => void;
  disabled: boolean;
}) {
  return (
    <div className="plot-sliders">
      {Object.entries(values).map(([name, value]) => {
        const range = request.param_ranges[name];
        if (!range || !validRange(range))
          return (
            <div className="plot-parameter" key={name}>
              <code>{name}</code>
              <span>{formatCoordinate(value)}</span>
            </div>
          );
        return (
          <label className="plot-slider" key={name}>
            <code>{name}</code>
            <input
              type="range"
              aria-label={name}
              min={range[0]}
              max={range[1]}
              step={(range[1] - range[0]) / 200}
              value={value}
              disabled={disabled}
              onChange={(e) => onChange(name, Number(e.target.value))}
            />
            <output>{formatCoordinate(value)}</output>
          </label>
        );
      })}
    </div>
  );
}
