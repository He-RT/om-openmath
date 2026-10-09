// Deterministic byte boundaries, never split by JavaScript UTF-16 character count.
export function* splitBytes(bytes, width) {
  if (!Number.isSafeInteger(width) || width <= 0) throw new RangeError('positive byte width required');
  for (let offset = 0; offset < bytes.byteLength; offset += width) yield bytes.subarray(offset, offset + width);
}
