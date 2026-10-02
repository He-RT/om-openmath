import { expect, test } from "vitest";
import { toByte, fromByte, replaceBytes } from "./positions";
test("editor UTF16 indices map to exact kernel UTF8 boundaries for Greek and surrogate pairs", () => {
  const source = "α🙂x";
  expect(toByte(source, 1)).toBe(2);
  expect(toByte(source, 3)).toBe(6);
  expect(toByte(source, 4)).toBe(7);
  expect(fromByte(source, 6)).toBe(3);
  expect(fromByte(source, 3)).toBe(1);
  expect(toByte(source, 2)).toBe(2);
  expect(replaceBytes(source, { start: 2, end: 6 }, "β")).toBe("αβx");
});
