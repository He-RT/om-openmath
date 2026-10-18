import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { useState } from "react";
import { test, expect, vi } from "vitest";
import { Markdown, TextCell } from "./TextCell";
import { CodeBlock,codeDialect } from "./CodeBlock";
import { messages } from "../i18n";
import { NotebookController } from "../state/notebookStore";
import { MockKernel } from "../test/mockKernel";
function TextHarness({initial="# Notes\n\nRead this paragraph."}:{initial?:string}) {
  const [source,setSource]=useState(initial),[editing,setEditing]=useState(false);
  return <TextCell source={source} index={0} editing={editing} onModeChange={setEditing} onChange={setSource} onFocus={()=>{}} t={messages("en")} onInsertCode={()=>{}}/>;
}
test("actual Markdown grammar formats lists/emphasis/math while code and HTML remain safe literal data", () => {
  const { container } = render(
    <Markdown
      source={
        "# Notes\n\n*careful* **reasoning**\n\n1. First\n2. Second\n\n`$x^2$` and $x^2$\n\n<script>window.evil=true</script>\n\n[bad](javascript:alert(1))\n\n![remote](https://tracker.invalid/pixel)"
      }
    />,
  );
  expect(screen.getByRole("heading", { name: "Notes" })).toBeTruthy();
  expect(container.querySelector("em")?.textContent).toBe("careful");
  expect(container.querySelector("ol")?.children.length).toBe(2);
  expect(container.querySelector("code")?.textContent).toBe("$x^2$");
  expect(container.querySelector(".katex")).toBeTruthy();
  expect(container.querySelector("script")).toBeNull();
  expect(container.querySelector("img")).toBeNull();
  expect(screen.getByText("bad").getAttribute("href")).toBeNull();
});
test("loaded text defaults to reading, then explicit edit and read preserve source and selection",()=>{
  render(<TextHarness/>);
  expect(screen.getByRole("heading",{name:"Notes"})).toBeTruthy();expect(screen.queryByRole("textbox")).toBeNull();
  fireEvent.click(screen.getByRole("button",{name:"Edit Text 1"}));
  const editor=screen.getByRole("textbox",{name:"Text input 1"}) as HTMLTextAreaElement;
  fireEvent.change(editor,{target:{value:"# Changed\n\nSource remains."}});editor.setSelectionRange(3,7);
  fireEvent.keyDown(editor,{key:"Escape"});
  expect(screen.getByRole("heading",{name:"Changed"})).toBeTruthy();expect(screen.queryByRole("textbox")).toBeNull();
  fireEvent.click(screen.getByRole("button",{name:"Edit Text 1"}));
  const reopened=screen.getByRole("textbox",{name:"Text input 1"}) as HTMLTextAreaElement;
  expect(reopened.value).toBe("# Changed\n\nSource remains.");expect(reopened.selectionStart).toBe(3);expect(reopened.selectionEnd).toBe(7);
  fireEvent.keyDown(reopened,{key:"Enter",ctrlKey:true});expect(screen.queryByRole("textbox")).toBeNull();
});
test("double click enters text editing while IME composition blocks the read shortcut",()=>{
  render(<TextHarness/>);fireEvent.doubleClick(screen.getByText("Read this paragraph."));
  const editor=screen.getByRole("textbox",{name:"Text input 1"});
  fireEvent.compositionStart(editor);fireEvent.keyDown(editor,{key:"Escape"});expect(screen.getByRole("textbox")).toBe(editor);
  fireEvent.compositionEnd(editor);fireEvent.keyDown(editor,{key:"Enter",metaKey:true});expect(screen.queryByRole("textbox")).toBeNull();
});
test("a shared fenced code block preserves literal bytes, never wraps and only inserts on explicit action",async()=>{
  const code="let α = 2\n\tα + 1\n<script>literal()</script>",copy=vi.fn().mockResolvedValue(undefined),insert=vi.fn();
  Object.defineProperty(navigator,"clipboard",{configurable:true,value:{writeText:copy}});
  const {container}=render(<CodeBlock code={code} language="OpenMath" t={messages("en")} onInsert={insert}/>);
  expect(container.querySelectorAll("pre")).toHaveLength(1);expect(container.querySelectorAll("script")).toHaveLength(0);
  const pre=screen.getByLabelText("Code · OpenMath");expect(pre.textContent).toBe(code);expect(getComputedStyle(pre).whiteSpace).toBe("pre");
  expect(insert).not.toHaveBeenCalled();fireEvent.click(screen.getByRole("button",{name:"Copy"}));await waitFor(()=>expect(copy).toHaveBeenCalledWith(code));
  fireEvent.click(screen.getByRole("button",{name:"Insert as cell"}));expect(insert).toHaveBeenCalledExactlyOnceWith(code,"Modern");
});
test("unknown and unlabelled code has no insert action; block contents do not become math or step links",()=>{
  const insert=vi.fn();
  const {container}=render(<Markdown source={'```javascript\n$x^2$ [S1] <script>literal()</script>\n```\n\n```\nraw text\n```'} t={messages("en")} onInsertCode={insert} stepReferences={{knownIds:new Set(["S1"]),onReference:vi.fn()}}/>);
  expect(container.querySelectorAll(".code-block")).toHaveLength(2);expect(container.querySelector(".katex")).toBeNull();expect(container.querySelector(".step-reference")).toBeNull();expect(container.querySelector("script")).toBeNull();
  expect(screen.queryByRole("button",{name:"Insert as cell"})).toBeNull();
  expect(codeDialect("wl")).toBe("Wolfram");expect(codeDialect("OpenMath title=example")).toBe("Modern");expect(codeDialect("python")).toBeNull();
});
test("new text enters editing but reading-mode changes are UI-only and never evaluate",async()=>{
  const kernel=new MockKernel(()=>({type:"ok"})),controller=new NotebookController(kernel);
  const id=controller.add("Text","# Source");await controller.flush();
  expect(controller.store.getState().cells[0]?.textEditing).toBe(true);
  const file=controller.file(),revision=controller.store.getState().cells[0]?.revision;
  controller.textMode(id,false);expect(controller.file()).toEqual(file);expect(controller.store.getState().cells[0]?.revision).toBe(revision);
  expect(kernel.requests.map(request=>request.type)).toEqual(["upsert_cell"]);expect(JSON.stringify(file)).not.toContain("textEditing");controller.dispose();
});
