import { render, screen } from "@testing-library/react";
import { test, expect } from "vitest";
import { Markdown } from "./TextCell";
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
