import type { Span } from "../../kernel/generated/Span";
let cached: string | undefined;
let utf16: number[] = [];
let utf8: number[] = [];
function offsets(source: string) {
  if (source === cached) return;
  cached = source;
  utf16 = [0];
  utf8 = [0];
  let position = 0;
  let bytes = 0;
  for (const char of source) {
    const cp = char.codePointAt(0) ?? 0;
    const length = cp < 128 ? 1 : cp < 2048 ? 2 : cp < 65536 ? 3 : 4;
    for (let i = 0; i < char.length; i++) utf16[position + i] = bytes;
    for (let i = 0; i < length; i++) utf8[bytes + i] = position;
    position += char.length;
    bytes += length;
    utf16[position] = bytes;
    utf8[bytes] = position;
  }
}
export function toByte(source: string, index: number) {
  offsets(source);
  return utf16[Math.max(0, Math.min(index, source.length))] ?? 0;
}
export function fromByte(source: string, index: number) {
  offsets(source);
  return utf8[Math.max(0, Math.min(index, utf8.length - 1))] ?? 0;
}
export function replaceBytes(source: string, span: Span, text: string): string {
  return (
    source.slice(0, fromByte(source, span.start)) +
    text +
    source.slice(fromByte(source, span.end))
  );
}
