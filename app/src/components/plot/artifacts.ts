import type { Artifact } from '../../kernel/generated/Artifact';
import type { PlotData } from '../../kernel/generated/PlotData';

function bytesArtifact(bytes: Uint8Array, mime: string, extension: string): Artifact {
  let binary = '';
  for (let i = 0; i < bytes.length; i += 8192)
    binary += String.fromCharCode(...bytes.subarray(i, i + 8192));
  return { mime, extension, base64: btoa(binary), byte_len: bytes.length };
}

/** Serializes retained coordinates only; never evaluates or interpolates an expression. */
export function plotCsvArtifact(data: PlotData): Artifact {
  const rows: unknown[][] = [['kind', 'series', 'label', 'segment', 'index', 'x', 'y', 'details']];
  data.curves.forEach((curve, series) => curve.segments.forEach((segment, s) =>
    segment.forEach(([x, y], i) => rows.push(['curve', series, curve.label, s, i, x, y, '']))));
  const g = data.geometry;
  g?.points.forEach(([x, y], i) => rows.push(['point', '', '', '', i, x, y, '']));
  g?.tiles.forEach((tile, i) => rows.push(['tile', '', '', '', i, ...tile.bounds[0], JSON.stringify(tile)]));
  g?.arrows.forEach((arrow, i) => rows.push(['vector', '', '', '', i, ...arrow.start, JSON.stringify(arrow)]));
  for (const [kind, paths] of [['path', g?.paths], ['polygon', g?.polygons]] as const)
    paths?.forEach((path, series) => path.points.forEach(([x, y], i) =>
      rows.push([kind, series, '', '', i, x, y, JSON.stringify({ ...path, points: undefined })])));
  g?.markers?.forEach((marker, i) => rows.push(['marker', '', '', '', i, ...marker.position, JSON.stringify(marker)]));
  g?.labels?.forEach((label, i) => rows.push(['label', '', label.text, '', i, ...label.position, JSON.stringify(label)]));
  data.highlights?.points.forEach(([x, y], i) => rows.push(['highlight', '', '', '', i, x, y, '']));
  data.highlights?.shade.forEach((range, i) => rows.push(['shade', '', '', '', i, '', '', JSON.stringify(range)]));
  const csv = rows.map(row => row.map(value => {
    const text = String(value);
    return /[",\r\n]/.test(text) ? `"${text.replaceAll('"', '""')}"` : text;
  }).join(',')).join('\r\n') + '\r\n';
  return bytesArtifact(new TextEncoder().encode(csv), 'text/csv;charset=utf-8', 'csv');
}

export async function canvasPngArtifact(canvas: HTMLCanvasElement): Promise<Artifact> {
  const blob = await new Promise<Blob>((resolve, reject) => canvas.toBlob(
    value => value ? resolve(value) : reject(new Error('PNG encoding failed')), 'image/png'));
  return bytesArtifact(new Uint8Array(await blob.arrayBuffer()), 'image/png', 'png');
}
