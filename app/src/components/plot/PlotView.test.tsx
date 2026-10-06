import {
  act,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { expect, test, vi } from "vitest";
import { PlotView } from "./PlotView";
import { MockKernel } from "../../test/mockKernel";
import { messages } from "../../i18n";
import type { PlotData } from "../../kernel/generated/PlotData";
import type { PlotRequest } from "../../kernel/generated/PlotRequest";
const request: PlotRequest = {
  kind: "Function",
  exprs: ["1/x", "a"],
  var_x: "x",
  var_y: null,
  x_range: [-5, 5],
  y_range: [-5, 5],
  params: { a: 1 },
  param_ranges: { a: [-5, 5] },
  points: [[1, 1]],
  shade: [],
  solve: { source: "1/x==a", domain: "Reals" },
};
const data: PlotData = {
  x_range: [-5, 5],
  y_range: [-5, 5],
  curves: [
    {
      label: "1/x",
      segments: [
        [
          [-5, -0.2],
          [-0.2, -5],
        ],
        [
          [0.2, 5],
          [5, 0.2],
        ],
      ],
    },
    {
      label: "a",
      segments: [
        [
          [-5, 1],
          [5, 1],
        ],
      ],
    },
  ],
  highlights: { points: [[1, 1]], shade: [[-1, 2]] },
};
const t = messages("en");
test("actual segments, solution labels, shade, legend and nice grids render without joining discontinuities", () => {
  render(
    <PlotView request={request} data={data} kernel={new MockKernel()} t={t} />,
  );
  expect(document.querySelectorAll(".plot-curve path")).toHaveLength(3);
  expect(document.querySelectorAll(".solution-point")).toHaveLength(1);
  expect(document.querySelector(".plot-point-label")?.textContent).toBe(
    "(1, 1)",
  );
  expect(document.querySelectorAll(".plot-shade")).toHaveLength(1);
  expect(screen.getByLabelText("a")).toBeTruthy();
  expect(screen.getByText("1/x", { selector: "code" })).toBeTruthy();
  expect(document.querySelectorAll(".plot-grid line").length).toBeGreaterThan(
    1,
  );
});
test("parameter drags coalesce behind real sampling; only newest results update actual highlights", async () => {
  const completions: ((
    value: import("../../kernel/client").Response,
  ) => void)[] = [];
  const kernel = new MockKernel(
    () => new Promise((resolve) => completions.push(resolve)),
  );
  const view = render(
    <PlotView request={request} data={data} kernel={kernel} t={t} />,
  );
  fireEvent.change(screen.getByLabelText("a"), { target: { value: "2" } });
  await waitFor(() => expect(kernel.requests).toHaveLength(1));
  fireEvent.change(screen.getByLabelText("a"), { target: { value: "3" } });
  const latest = {
    ...data,
    highlights: { points: [[1 / 3, 3]] as [number, number][], shade: [] },
  };
  await act(async () =>
    completions[0]!({
      type: "plot",
      data: { ...data, highlights: { points: [[0.5, 2]], shade: [] } },
    }),
  );
  await waitFor(() => expect(kernel.requests).toHaveLength(2));
  expect(kernel.requests[1]).toMatchObject({
    type: "sample_plot",
    request: { params: { a: 3 }, solve: request.solve },
  });
  await act(async () => completions[1]!({ type: "plot", data: latest }));
  expect(document.querySelector(".plot-point-label")?.textContent).toContain(
    "3)",
  );
  expect(document.querySelectorAll(".plot-shade")).toHaveLength(0);
  view.unmount();
});
test("real wheel, keyboard pan and reset send viewport requests after debounce, with unchanged solver source", async () => {
  const kernel = new MockKernel((r) =>
    r.type === "sample_plot"
      ? {
          type: "plot",
          data: {
            ...data,
            x_range: r.request.x_range,
            y_range: r.request.y_range ?? data.y_range,
          },
        }
      : { type: "ok" },
  );
  render(<PlotView request={request} data={data} kernel={kernel} t={t} />);
  const svg = screen.getByRole("img", { name: "Plot" });
  vi.spyOn(svg, "getBoundingClientRect").mockReturnValue({
    x: 0,
    y: 0,
    left: 0,
    top: 0,
    right: 640,
    bottom: 320,
    width: 640,
    height: 320,
    toJSON: () => ({}),
  });
  fireEvent.wheel(svg, { clientX: 320, clientY: 160, deltaY: -100 });
  expect(kernel.requests).toHaveLength(0);
  await waitFor(() => expect(kernel.requests).toHaveLength(1));
  const first = kernel.requests[0];
  if (first?.type !== "sample_plot") throw new Error("missing sample");
  expect(first.request.x_range[1] - first.request.x_range[0]).toBeLessThan(10);
  expect(first.request.solve).toEqual(request.solve);
  fireEvent.keyDown(svg, { key: "ArrowRight" });
  await waitFor(() => expect(kernel.requests).toHaveLength(2));
  fireEvent.doubleClick(svg);
  await waitFor(() => expect(kernel.requests).toHaveLength(3));
  expect(kernel.requests[2]).toMatchObject({
    request: { x_range: [-5, 5], y_range: [-5, 5] },
  });
});
test("failures are shown and retry works; stale output and disposed plots do not apply old replies", async () => {
  let done!: (value: import("../../kernel/client").Response) => void;
  const kernel = new MockKernel(
    () => new Promise((resolve) => (done = resolve)),
  );
  const view = render(<PlotView request={request} kernel={kernel} t={t} />);
  await waitFor(() => expect(kernel.requests).toHaveLength(1));
  await act(async () =>
    done({ type: "error", message: "Cannot sample this viewport" }),
  );
  expect((await screen.findByRole("alert")).textContent).toContain(
    "Cannot sample",
  );
  fireEvent.click(screen.getByRole("button", { name: "Retry" }));
  await waitFor(() => expect(kernel.requests).toHaveLength(2));
  view.rerender(
    <PlotView
      request={request}
      data={data}
      kernel={kernel}
      t={t}
      active={false}
    />,
  );
  await act(async () =>
    done({
      type: "plot",
      data: { ...data, highlights: { points: [[4, 4]], shade: [] } },
    }),
  );
  expect(document.querySelector(".plot-point-label")?.textContent).toBe(
    "(1, 1)",
  );
  expect((screen.getByLabelText("a") as HTMLInputElement).disabled).toBe(true);
});

test("extended tiles, vectors and data points render supplied geometry and expose actual values", () => {
  const extended:PlotData={...data,curves:[],geometry:{tiles:[{bounds:[[1,1],[2,3]],value:7,color:'#abcdef'}],arrows:[{start:[1,2],end:[2,3],value:[4,5]}],points:[[2,4]],skipped:3,color_range:[0,7]}};
  render(<PlotView request={{...request,kind:'Data'}} data={extended} kernel={new MockKernel()} t={t}/>);
  expect(document.querySelector('rect[fill="#abcdef"] title')?.textContent).toBe('7');
  expect(document.querySelector('circle[r="3"] title')?.textContent).toBe('(2, 4)');
  expect(screen.getByText('Skipped nonfinite or out-of-domain samples: 3')).toBeTruthy();
  fireEvent(document.querySelector('details')!,new Event('toggle'));
  const details=document.querySelector('details')!;details.open=true;fireEvent(details,new Event('toggle'));
  expect(document.querySelector('.plot-sample-data')?.textContent).toContain('"value": 7');
  expect(document.querySelector('.plot-sample-data')?.textContent).toContain('"arrows"');
});
