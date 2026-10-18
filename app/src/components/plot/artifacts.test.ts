import { expect, test } from 'vitest';
import { plotCsvArtifact } from './artifacts';
import type { PlotData } from '../../kernel/generated/PlotData';

test('CSV retains original numbers, discontinuity boundaries, Unicode labels and structured geometry', () => {
  const data: PlotData = {
    x_range: [-1, 1], y_range: [-3, 3],
    curves: [{ label: '中文,"line"\nnext', segments: [[[-1, -3]], [[.125, 2.718281828459045]]] }],
    geometry: { points: [[7, 9]], tiles: [{ bounds: [[0, 1], [2, 3]], value: 4, color: '#abcdef' }],
      arrows: [{ start: [1, 2], end: [3, 4], value: [5, 6] }], skipped: 2, color_range: [0, 4] },
  };
  const artifact = plotCsvArtifact(data);
  const bytes = Uint8Array.from(atob(artifact.base64), c => c.charCodeAt(0));
  expect(bytes.length).toBe(artifact.byte_len);
  const csv = new TextDecoder().decode(bytes);
  expect(csv).toContain('"中文,""line""\nnext",1,0,0.125,2.718281828459045');
  expect(csv).toContain('point,,,,0,7,9,\r\n');
  expect(csv).toContain('""bounds"":[[0,1],[2,3]]');
  expect(csv).toContain('""value"":[5,6]');
  expect(csv).not.toContain('NaN');
});
