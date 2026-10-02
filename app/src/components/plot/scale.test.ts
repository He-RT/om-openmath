import { expect, test } from "vitest";
import { niceTicks, linearScale, zoomRange, panRange } from "./scale";
test("nice ticks use 1/2/5 steps, ordered finite values and meaningful small/large ranges", () => {
  expect(niceTicks([-5, 5], 5)).toEqual([-4, -2, 0, 2, 4]);
  expect(niceTicks([0, 1], 5)).toEqual([0, 0.2, 0.4, 0.6, 0.8, 1]);
  expect(niceTicks([1, 1], 6)).toEqual([]);
  expect(niceTicks([NaN, 4], 6)).toEqual([]);
  for (const range of [
    [-1e-300, 1e-300],
    [1e308, 1.0000000000000002e308],
    [-2e100, 3e100],
  ] as [number, number][]) {
    const ticks = niceTicks(range, 6);
    expect(ticks.length).toBeGreaterThan(0);
    expect(ticks.length).toBeLessThan(20);
    expect(ticks.every(Number.isFinite)).toBe(true);
    expect(
      ticks.every(
        (v, i) =>
          v >= range[0] && v <= range[1] && (i === 0 || v > ticks[i - 1]!),
      ),
    ).toBe(true);
  }
});
test("scales preserve points and inverse transforms; cursor zoom preserves the anchor and invalid gestures stay finite", () => {
  const scale = linearScale([-5, 5], [40, 600]);
  for (const n of [-5, -2, 0, 1, 5])
    expect(scale.invert(scale.map(n))).toBeCloseTo(n, 14);
  expect(zoomRange([-5, 5], 0.25, 0.5)).toEqual([-3.75, 1.25]);
  expect(panRange([-5, 5], 0.2)).toEqual([-3, 7]);
  expect(zoomRange([1e308, 1.0000000000000002e308], 0.5, 1e200)).toEqual([
    1e308, 1.0000000000000002e308,
  ]);
});
