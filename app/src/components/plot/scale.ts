/** A finite ordered numeric domain or pixel range. */
export type Range = [number, number];
/** Match the kernel's finite positive-width viewport contract. */
export function validRange([lo, hi]: Range) {
  return (
    Number.isFinite(lo) &&
    Number.isFinite(hi) &&
    lo < hi &&
    Number.isFinite(hi - lo)
  );
}
/** Generate deterministic 1/2/5×10^k ticks with a bounded output. */
export function niceTicks(domain: Range, count = 6): number[] {
  if (!validRange(domain) || !Number.isFinite(count) || count < 1) return [];
  const [lo, hi] = domain;
  const raw = (hi - lo) / Math.min(count, 32) || hi - lo;
  const base = Math.max(Number.MIN_VALUE, 10 ** Math.floor(Math.log10(raw)));
  const fraction = raw / base;
  const step =
    (fraction <= 1 ? 1 : fraction <= 2 ? 2 : fraction <= 5 ? 5 : 10) * base;
  if (!Number.isFinite(step) || step <= 0) return [lo, hi];
  const start = Math.ceil(lo / step) * step;
  if (!Number.isFinite(start)) return [lo, hi];
  const result: number[] = [];
  for (let i = 0; i < 64; i++) {
    let value = start + i * step;
    if ((hi - lo) / Math.max(Math.abs(lo), Math.abs(hi)) > 1e-12)
      value = Number(value.toPrecision(14));
    if (value > hi) break;
    if (
      value >= lo &&
      Number.isFinite(value) &&
      (result.length === 0 || value > result[result.length - 1]!)
    )
      result.push(value === 0 ? 0 : value);
  }
  return result.length ? result : [lo, hi];
}
/** Stable affine coordinate conversion, with the same inverse mapping. */
export function linearScale(domain: Range, pixels: Range) {
  const magnitude = Math.max(Math.abs(domain[0]), Math.abs(domain[1])) || 1;
  const lo = domain[0] / magnitude,
    hi = domain[1] / magnitude;
  return {
    map: (value: number) =>
      pixels[0] +
      ((value / magnitude - lo) / (hi - lo)) * (pixels[1] - pixels[0]),
    invert: (pixel: number) =>
      (lo + ((pixel - pixels[0]) / (pixels[1] - pixels[0])) * (hi - lo)) *
      magnitude,
  };
}
/** Display transform only; mathematical samples remain original kernel coordinates. */
export function axisScale(domain: Range, pixels: Range, logarithmic = false) {
  if (!logarithmic) return linearScale(domain,pixels);
  const scale=linearScale([Math.log(domain[0]),Math.log(domain[1])],pixels);
  return {map:(value:number)=>scale.map(Math.log(value)),invert:(pixel:number)=>Math.exp(scale.invert(pixel))};
}
export function axisTicks(domain:Range,count:number,logarithmic=false){
  return logarithmic ? niceTicks([Math.log10(domain[0]),Math.log10(domain[1])],count).map(v=>10**v) : niceTicks(domain,count);
}
/** Zoom around the given fraction, retaining the cursor's world coordinate. */
export function zoomRange(
  domain: Range,
  fraction: number,
  factor: number,
  logarithmic = false,
): Range {
  if (logarithmic) {
    if(domain[0]<=0)return domain;
    const r=zoomRange([Math.log(domain[0]),Math.log(domain[1])],fraction,factor);
    const result:Range=[Math.exp(r[0]),Math.exp(r[1])];return validRange(result)&&result[0]>0?result:domain;
  }
  if (
    !validRange(domain) ||
    !Number.isFinite(fraction) ||
    !Number.isFinite(factor) ||
    factor <= 0
  )
    return domain;
  const anchor = domain[0] + (domain[1] - domain[0]) * fraction;
  const result: Range = [
    anchor + (domain[0] - anchor) * factor,
    anchor + (domain[1] - anchor) * factor,
  ];
  return validRange(result) ? result : domain;
}
/** Translate by a fraction of the domain width, never creating invalid ranges. */
export function panRange(domain: Range, fraction: number, logarithmic = false): Range {
  if(logarithmic){if(domain[0]<=0)return domain;const r=panRange([Math.log(domain[0]),Math.log(domain[1])],fraction);const result:Range=[Math.exp(r[0]),Math.exp(r[1])];return validRange(result)&&result[0]>0?result:domain;}
  const offset = (domain[1] - domain[0]) * fraction;
  const result: Range = [domain[0] + offset, domain[1] + offset];
  return Number.isFinite(fraction) && validRange(result) ? result : domain;
}
/** Short decimal tick/coordinate labels; never used as a CAS approximation. */
export function formatCoordinate(value: number) {
  return Number(value.toPrecision(6)).toString();
}
