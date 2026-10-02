import { expect, test } from "vitest";
import { decodeNotebook } from "./files";
test("opening files admits only source cells and rejects duplicate/version/invalid syntax DTOs", () => {
  const incoming = {
    version: 1,
    title: "source",
    config: { api_key: "private-key" },
    cells: [
      {
        id: "a",
        kind: "Math",
        source: "solve(",
        dialect: "Modern",
        output: { private: "result" },
        api_key: "private-key",
      },
    ],
  };
  const file = decodeNotebook(JSON.stringify(incoming));
  expect(file.cells[0]?.source).toBe("solve(");
  expect(JSON.stringify(file)).not.toContain("private-key");
  expect(JSON.stringify(file)).not.toContain("output");
  expect(() =>
    decodeNotebook(JSON.stringify({ ...incoming, version: 2 })),
  ).toThrow();
  expect(() =>
    decodeNotebook(
      JSON.stringify({
        ...incoming,
        cells: [incoming.cells[0], incoming.cells[0]],
      }),
    ),
  ).toThrow();
});
