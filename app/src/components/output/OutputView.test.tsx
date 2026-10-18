import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { messages } from "../../i18n";
import { MockKernel } from "../../test/mockKernel";
import type { OutputItem } from "../../kernel/generated/OutputItem";
import { OutputView } from "./OutputView";
import { messageText } from "../../i18n/messages";
import { NumberLine } from "./NumberLine";
const t = messages("en");
async function copyAction(name:string) {
  fireEvent.click(screen.getByRole("button",{name:"Copy"}));
  fireEvent.click(await screen.findByRole("button",{name}));
}
const expr: Extract<OutputItem, { type: "expr" }> = {
  type: "expr",
  out_index: 1,
  input_form: "Sqrt[2]",
  modern_form: "sqrt(2)",
  latex: "\\sqrt{2}",
};
const solve: Extract<OutputItem, { type: "solutions" }> = {
  type: "solutions",
  out_index: 2,
  input_form: "{{x->1},{x->1}}",
  modern_form: "[[x->1],[x->1]]",
  steps: { root: [] },
  plot: null,
  view: {
    kind: "finite",
    vars: ["x"],
    region_latex: null,
    intervals: [],
    solutions: Array.from({ length: 2 }, () => ({
      bindings: [
        {
          var: "x",
          latex: "1",
          input_form: "1",
          modern_form: "1",
          numeric: "1.0000000000000000000",
          var_latex: "x",
        },
      ],
      condition_latex: null,
      verified: "Exact",
    })),
  },
};
function mount(
  item: OutputItem,
  options: {
    kernel?: MockKernel;
    insert?: (source: string) => void;
    steps?: () => void;
  } = {},
) {
  return render(
    <OutputView
      output={{ items: [item], messages: [], timing_ms: 12 }}
      stale={false}
      t={t}
      language="en"
      kernel={options.kernel ?? new MockKernel()}
      onInsert={options.insert ?? vi.fn()}
      onSteps={options.steps ?? vi.fn()}
    />,
  );
}
describe("actual output interactions", () => {
  it("shows actual numeric evidence, refuses false infinities and translates only known static message text", () => {
    const numeric = {
      ...solve,
      view: {
        ...solve.view,
        solutions: [
          {
            ...solve.view.solutions[0]!,
            verified: { Numeric: { digits: 20 } },
          },
        ],
      },
    };
    const view = mount(numeric);
    expect(screen.getByText("Numerically verified")).toBeTruthy();
    expect(screen.getByText("Numerical approximation")).toBeTruthy();
    expect(screen.getByText("20 verified digits")).toBeTruthy();
    view.unmount();
    render(
      <NumberLine
        intervals={[
          {
            lo: "a",
            hi: "b",
            lo_value: null,
            hi_value: null,
            lo_closed: true,
            hi_closed: false,
          },
        ]}
        variable="x"
        t={t}
      />,
    );
    expect(screen.queryByRole("img")).toBeNull();
    expect(
      screen.getByText("Symbolic endpoints are shown as conditions."),
    ).toBeTruthy();
    const m = {
      symbol: "Solve",
      tag: "svars",
      text: "Equations leave some requested variables free.",
      level: "Warning" as const,
    };
    expect(messageText(m, "zh-CN")).toBe("方程未确定部分待求变量的值。");
    expect(messageText({ ...m, text: "Original detail: x < 2" }, "zh-CN")).toBe(
      "Original detail: x < 2",
    );
    expect(messageText(m, "en")).toBe(m.text);
  });
  it("renders available radical projections without inventing them for unsupported roots", () => {
    const binding = {
      ...solve.view.solutions[0]!.bindings[0]!,
      input_form: "Root[#^2-2&,1]",
      root_index: 1,
      numeric: "-1.4142135623730950488",
      latex: "\\operatorname{Root}_1",
      radicals: {
        input_form: "-Sqrt[2]",
        modern_form: "-sqrt(2)",
        latex: "-\\sqrt{2}",
      },
    };
    mount({
      ...solve,
      view: {
        ...solve.view,
        solutions: [{ ...solve.view.solutions[0]!, bindings: [binding] }],
      },
    });
    expect(screen.getByText(/Root 1/)).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Radical form" }));
    expect(screen.getByRole("button", { name: "Root form" })).toBeTruthy();
    expect(
      document.querySelector(".solution-binding .formula")?.innerHTML,
    ).toContain("sqrt");
  });
  it("copies the exact three forms, inserts source and asks the kernel for N rather than computing in JS", async () => {
    const copy = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: { writeText: copy },
    });
    const insert = vi.fn();
    const kernel = new MockKernel(() => ({
      type: "expression",
      value: {
        input_form: "1.4142135623730950488",
        modern_form: "1.4142135623730950488",
        latex: "1.4142135623730950488",
      },
    }));
    mount(expr, { kernel, insert });
    for (const [name, value] of [
      ["Copy LaTeX", expr.latex],
      ["Copy Wolfram", expr.input_form],
      ["Copy modern syntax", expr.modern_form],
    ] as const) {
      await copyAction(name);
      await waitFor(() => expect(copy).toHaveBeenLastCalledWith(value));
    }
    await copyAction("Insert into new cell");
    expect(insert).toHaveBeenCalledWith("Sqrt[2]");
    fireEvent.click(screen.getByRole("button", { name: "Numeric ≈" }));
    await screen.findByLabelText("1.4142135623730950488");
    expect(kernel.requests).toEqual([
      { type: "inspect_expression", source: "Sqrt[2]", numeric: true },
    ]);
    fireEvent.click(screen.getByRole("button", { name: "Exact" }));
    expect(screen.getByLabelText("Sqrt[2]")).toBeTruthy();
  });
  it("reports clipboard denial and keeps source visible", async () => {
    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: { writeText: vi.fn().mockRejectedValue(new Error("denied")) },
    });
    mount(expr);
    await copyAction("Copy Wolfram");
    expect((await screen.findByRole("alert")).textContent).toContain(
      "Could not copy",
    );
    expect(screen.getByLabelText("Sqrt[2]")).toBeTruthy();
  });
  it("groups repeated roots but copies all rules and creates an editable substitution template", async () => {
    const copy = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: { writeText: copy },
    });
    const insert = vi.fn();
    mount(solve, { insert });
    expect(screen.getByText("2 solutions")).toBeTruthy();
    expect(document.querySelectorAll(".solution-entry")).toHaveLength(1);
    expect(screen.getByText("double root")).toBeTruthy();
    await copyAction("Copy all");
    await waitFor(() => expect(copy).toHaveBeenCalledWith(solve.input_form));
    await copyAction("Substitute…");
    expect(insert).toHaveBeenCalledWith("expr /. {x->1}");
  });
  it("never labels unverified evidence as verified and distinguishes All and None", () => {
    const item = {
      ...solve,
      view: {
        ...solve.view,
        solutions: [
          { ...solve.view.solutions[0]!, verified: "Unverified" as const },
        ],
      },
    };
    const { unmount } = mount(item);
    expect(screen.getByText("Unverified")).toBeTruthy();
    expect(screen.queryByText("Verified ✓")).toBeNull();
    unmount();
    const a = mount({
      ...solve,
      view: { ...solve.view, kind: "all", solutions: [] },
    });
    expect(screen.getByText("True for all values")).toBeTruthy();
    a.unmount();
    mount({ ...solve, view: { ...solve.view, kind: "none", solutions: [] } });
    expect(screen.getByText("No solutions")).toBeTruthy();
  });
  it("draws closed/open intervals and infinite arrows from the actual interval DTO", () => {
    mount({
      ...solve,
      view: {
        kind: "region",
        vars: ["x"],
        solutions: [],
        region_latex: "x\\le 1\\lor x>2",
        intervals: [
          {
            lo: null,
            hi: "1",
            lo_value: null,
            hi_value: 1,
            lo_closed: false,
            hi_closed: true,
          },
          {
            lo: "2",
            hi: null,
            lo_value: 2,
            hi_value: null,
            lo_closed: false,
            hi_closed: false,
          },
        ],
      },
    });
    const svg = screen.getByRole("img", { name: /Number line/ });
    expect(svg.querySelectorAll(".interval-segment")).toHaveLength(2);
    expect(svg.querySelectorAll(".endpoint-closed")).toHaveLength(1);
    expect(svg.querySelectorAll(".endpoint-open")).toHaveLength(1);
    expect(svg.querySelectorAll(".infinite-arrow")).toHaveLength(2);
  });
});
it("more than four solutions use a table with exact and actual numeric values",()=>{
  mount({...solve,view:{...solve.view,solutions:Array.from({length:5},(_,index)=>({...solve.view.solutions[0]!,bindings:[{...solve.view.solutions[0]!.bindings[0]!,input_form:String(index),latex:String(index),modern_form:String(index),numeric:`${index}.000`}]}))}});
  expect(screen.getByRole("table").querySelectorAll("tbody tr")).toHaveLength(5);
  expect(screen.getByRole("columnheader",{name:"Exact value"})).toBeTruthy();
  expect(screen.getByRole("columnheader",{name:"Approximation"})).toBeTruthy();
  expect(screen.getByText("x ≈ 4.000")).toBeTruthy();
});
it("a numerical-only solution table does not label its values as exact",()=>{
  mount({...solve,view:{...solve.view,solutions:Array.from({length:5},(_,index)=>({...solve.view.solutions[0]!,verified:{Numeric:{digits:20}},bindings:[{...solve.view.solutions[0]!.bindings[0]!,input_form:String(index),latex:String(index),modern_form:String(index)}]}))}});
  expect(screen.getByRole("columnheader",{name:"Value"})).toBeTruthy();
  expect(screen.queryByRole("columnheader",{name:"Exact value"})).toBeNull();
});
it("a numeric projection arriving after the source becomes stale cannot repaint its result",async()=>{
  let finish!:(value:import("../../kernel/client").Response)=>void;
  const kernel=new MockKernel(()=>new Promise(resolve=>{finish=resolve;}));
  const props={output:{items:[expr],messages:[],timing_ms:1},t,language:"en" as const,kernel,onInsert:vi.fn(),onSteps:vi.fn()};
  const view=render(<OutputView {...props} stale={false}/>);
  fireEvent.click(screen.getByRole("button",{name:"Numeric ≈"}));
  view.rerender(<OutputView {...props} stale/>);
  await act(async()=>{finish({type:"expression",value:{input_form:"1.414",modern_form:"1.414",latex:"1.414"}});});
  expect(screen.getByLabelText(expr.input_form)).toBeTruthy();expect(screen.queryByLabelText("1.414")).toBeNull();
  expect((screen.getByRole("button",{name:"Numeric ≈"}) as HTMLButtonElement).disabled).toBe(true);
});
it("error location uses the original UTF-8 span and is an explicit editor action",()=>{
  const locate=vi.fn(),span={start:3,end:4};
  render(<OutputView output={{items:[{type:"error",message:"incomplete",span}],messages:[],timing_ms:1}} stale={false} t={t} language="en" kernel={new MockKernel()} onInsert={vi.fn()} onSteps={vi.fn()} source="α +" onLocate={locate}/>);
  expect(screen.getByRole("alert").textContent).toContain("1:3");
  fireEvent.click(screen.getByRole("button",{name:"Locate"}));expect(locate).toHaveBeenCalledWith(span);
});
