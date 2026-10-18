import { useEffect, useRef, useState, type ReactNode } from 'react';
import type { OutputItem } from '../../kernel/generated/OutputItem';
import type { KernelClient } from '../../kernel/client';
import type { Messages, Locale } from '../../i18n';
import { ArtifactButton } from '../output/ArtifactButton';
import { GraphFooter } from '../output/GraphFooter';
import { ToolbarMenu } from '../ToolbarMenu';
import { WorkbenchIcon } from '../WorkbenchIcon';
import { canvasPngArtifact } from '../plot/artifacts';
import { createRenderer } from './render';
import { defaultCamera, fittedCamera, normalizePoint, project, zoom, type Camera } from './camera';
export interface CameraMemory { current: Camera; fitted?: boolean }
export function SceneView({ item, kernel, t, language, active = true, cameraMemory, renderStatus }: {
  item: Extract<OutputItem, { type: 'scene3_d' }>; kernel: KernelClient; t: Messages; language: Locale;
  active?: boolean; cameraMemory?: CameraMemory | undefined;
  renderStatus?: ((actions: ReactNode) => ReactNode) | undefined;
}) {
  const zh = language === 'zh-CN', text = (a: string, b: string) => zh ? a : b;
  const canvas = useRef<HTMLCanvasElement>(null), stage = useRef<HTMLDivElement>(null);
  const renderer = useRef<ReturnType<typeof createRenderer> | null>(null);
  const background = useRef<[number, number, number]>([.98, .99, .98]);
  const [camera, setCamera] = useState<Camera>(() => {
    const initial = cameraMemory?.fitted ? cameraMemory.current : item.data ? fittedCamera(item.data) : defaultCamera;
    if (cameraMemory) { cameraMemory.current = initial; cameraMemory.fitted = true; }
    return initial;
  });
  const current = useRef(camera); current.current = camera;
  const [error, setError] = useState<string | null>(null), [showData, setShowData] = useState(false);
  const [size, setSize] = useState({ width: 640, height: 400 });
  const [exportStatus,setExportStatus] = useState<string|null>(null);
  const [restore, setRestore] = useState(0), [interacted, setInteracted] = useState(false);
  const [fullscreen, setFullscreen] = useState(false), [controlError, setControlError] = useState('');
  const source = item.request.expressions.join(', '), data = item.data;
  const resetCamera = data ? fittedCamera(data) : defaultCamera;
  const axisNames = item.request.kind === 'parametric' || item.request.kind === 'scene' ? ['x', 'y', 'z'] :
    [item.request.axes[0]?.name ?? 'x', item.request.axes[1]?.name ?? 'y', item.request.axes[2]?.name ?? 'z'];
  const update = (next: Camera) => {
    if (!active) return;
    setInteracted(true); setCamera(next); if (cameraMemory) cameraMemory.current = next;
  };
  useEffect(() => {
    const element = canvas.current; if (!element || !data) return;
    const draw = () => renderer.current?.draw(current.current, background.current);
    const theme = () => {
      const rgb = getComputedStyle(element).backgroundColor.match(/[\d.]+/g)?.slice(0, 3).map(Number);
      if (rgb?.length === 3) background.current = rgb.map(v => v / 255) as [number, number, number];
      draw();
    };
    try { renderer.current = createRenderer(element, data); theme(); setError(null); }
    catch (failure) { setError(failure instanceof Error ? failure.message : 'WebGL2 unavailable'); }
    const resize = () => { const box = element.getBoundingClientRect(); setSize({ width: box.width, height: box.height }); draw(); };
    const observer = typeof ResizeObserver !== 'undefined' ? new ResizeObserver(resize) : null;
    observer?.observe(element);
    const appearance = new MutationObserver(theme);
    appearance.observe(document.documentElement, { attributes: true, attributeFilter: ['data-theme', 'class', 'style'] });
    const systemTheme = window.matchMedia('(prefers-color-scheme: dark)'); systemTheme.addEventListener('change', theme);
    const lost = (event: Event) => {
      event.preventDefault(); renderer.current?.dispose(); renderer.current = null;
      setError(text('三维图形上下文已丢失，可复位或导出 OBJ', '3D graphics context lost; reset or export OBJ'));
    };
    const regained = () => setRestore(v => v + 1);
    element.addEventListener('webglcontextlost', lost); element.addEventListener('webglcontextrestored', regained); resize();
    return () => {
      observer?.disconnect(); appearance.disconnect(); systemTheme.removeEventListener('change', theme);
      element.removeEventListener('webglcontextlost', lost); element.removeEventListener('webglcontextrestored', regained);
      renderer.current?.dispose(); renderer.current = null;
    };
  }, [data, restore]);
  useEffect(() => { renderer.current?.draw(camera, background.current); }, [camera]);
  useEffect(() => {
    const changed = () => setFullscreen(document.fullscreenElement === stage.current);
    document.addEventListener('fullscreenchange', changed);
    return () => document.removeEventListener('fullscreenchange', changed);
  }, []);
  const wheel = useRef<(event: WheelEvent) => void>(() => {});
  wheel.current = event => {
    if (!active) return; event.preventDefault();
    update(zoom(current.current, Math.exp(Math.max(-1, Math.min(1, event.deltaY * .002)))));
  };
  useEffect(() => {
    const c = canvas.current, handler = (e: WheelEvent) => wheel.current(e);
    c?.addEventListener('wheel', handler, { passive: false });
    return () => c?.removeEventListener('wheel', handler);
  }, []);
  const drag = useRef<{ id: number; point: [number, number]; camera: Camera; pan: boolean } | null>(null);
  const rotate = (yaw: number, pitch: number) => update({ ...camera, yaw: camera.yaw + yaw, pitch: Math.max(-1.5, Math.min(1.5, camera.pitch + pitch)) });
  const count = data?.meshes.reduce((sum, mesh) => sum + mesh.triangles.length, 0) ?? 0;
  const points = data ? data.meshes.reduce((n, mesh) => n + mesh.positions.length, 0) + data.lines.reduce((n, line) => n + line.positions.length, 0) + data.points.length : 0;
  const aspect = Math.max(1, size.width) / Math.max(1, size.height);
  const labels = data ? [
    [data.bounds[1][0], data.bounds[0][1], data.bounds[0][2]],
    [data.bounds[0][0], data.bounds[1][1], data.bounds[0][2]],
    [data.bounds[0][0], data.bounds[0][1], data.bounds[1][2]],
  ].map((p, i) => ({ name: axisNames[i]!, p: project(normalizePoint(p as [number, number, number], data.bounds), camera, aspect) })) : [];
  const reset = () => { update({ ...resetCamera }); if (error) setRestore(v => v + 1); };
  const fullScreen = () => {
    setControlError('');
    const result = document.fullscreenElement === stage.current ? document.exitFullscreen() : stage.current?.requestFullscreen();
    void result?.catch(() => setControlError(text('此浏览器无法进入全屏', 'This browser could not enter full screen')));
  };
  return <figure className="scene-view" aria-label={text('真实三维场景', 'Actual 3D scene')}>
    {data && <div ref={stage} className={`scene-canvas-wrap ${error ? 'graph-error' : ''}`}>
      <canvas ref={canvas} height="400" style={error ? { visibility: 'hidden' } : undefined}
        aria-label={text('三维网格：方向键旋转，Shift加方向键平移，正负键缩放，Home复位', '3D mesh: arrows rotate, Shift+arrows pan, plus/minus zoom, Home resets')}
        role="img" tabIndex={active ? 0 : -1}
        onPointerDown={e => { if (!active || e.button !== 0) return; e.currentTarget.setPointerCapture?.(e.pointerId); drag.current = { id: e.pointerId, point: [e.clientX, e.clientY], camera, pan: e.shiftKey }; }}
        onPointerMove={e => {
          const start = drag.current; if (!start || start.id !== e.pointerId || !active) return;
          const dx = e.clientX - start.point[0], dy = e.clientY - start.point[1];
          update(start.pan ? { ...start.camera, panX: start.camera.panX + dx / 120, panY: start.camera.panY - dy / 120 } :
            { ...start.camera, yaw: start.camera.yaw + dx * .006, pitch: Math.max(-1.5, Math.min(1.5, start.camera.pitch + dy * .006)) });
        }}
        onPointerUp={e => { if (drag.current?.id !== e.pointerId) return; drag.current = null; if (e.currentTarget.hasPointerCapture?.(e.pointerId)) e.currentTarget.releasePointerCapture?.(e.pointerId); }}
        onPointerCancel={() => { drag.current = null; }} onDoubleClick={reset}
        onKeyDown={e => {
          if (!active || !['ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown', '+', '=', '-', 'Home'].includes(e.key)) return;
          e.preventDefault();
          if (e.key === 'Home') reset();
          else if (e.key === '+' || e.key === '=') update(zoom(camera, .8));
          else if (e.key === '-') update(zoom(camera, 1.25));
          else if (e.shiftKey) update({ ...camera, panX: camera.panX + (e.key === 'ArrowLeft' ? -.1 : e.key === 'ArrowRight' ? .1 : 0), panY: camera.panY + (e.key === 'ArrowUp' ? .1 : e.key === 'ArrowDown' ? -.1 : 0) });
          else rotate(e.key === 'ArrowLeft' ? -.1 : e.key === 'ArrowRight' ? .1 : 0, e.key === 'ArrowUp' ? -.1 : e.key === 'ArrowDown' ? .1 : 0);
        }} />
      {!error && (data.labels ?? []).map((label, i) => {
        const p = project(normalizePoint(label.position, data.bounds), camera, aspect);
        return p && <span className="scene-text-label" key={i} style={{ left: `calc(${p[0] * 100}% + ${label.offset[0]}px)`, top: `calc(${p[1] * 100}% + ${label.offset[1]}px)`, color: `rgba(${label.color[0] * 255},${label.color[1] * 255},${label.color[2] * 255},${label.color[3]})` }}>{label.text}</span>;
      })}
      {!error && labels.map(label => label.p && <span className="scene-axis-label" key={label.name} style={{ left: `${label.p[0] * 100}%`, top: `${label.p[1] * 100}%` }}>{label.name}</span>)}
      {!interacted && !error && <span className="scene-hint">{text('拖动旋转，滚动缩放', 'Drag to rotate, scroll to zoom')}</span>}
      <div className="graph-controls" role="group" aria-label={t.graphControls}>
        <button disabled={!active} onClick={reset} aria-label={error ? t.retry : t.resetView} title={error ? t.retry : t.resetView}><WorkbenchIcon name="reset" /></button>
        <button disabled={!active || !!error || !document.fullscreenEnabled} onClick={fullScreen} aria-label={fullscreen ? t.exitFullScreen : t.fullScreen} title={fullscreen ? t.exitFullScreen : t.fullScreen}><WorkbenchIcon name="expand" /></button>
        <ToolbarMenu label={t.graphControls} icon={<WorkbenchIcon name="more" />}>
          <button disabled={!active || !!error} onClick={() => rotate(-.15, 0)}>{text('左转', 'Rotate left')}</button>
          <button disabled={!active || !!error} onClick={() => rotate(.15, 0)}>{text('右转', 'Rotate right')}</button>
          <button disabled={!active || !!error} onClick={() => rotate(0, .15)}>{text('向上查看', 'Tilt up')}</button>
          <button disabled={!active || !!error} onClick={() => rotate(0, -.15)}>{text('向下查看', 'Tilt down')}</button>
          <button disabled={!active || !!error} onClick={() => update(zoom(camera, .8))}>{t.zoomIn}</button>
          <button disabled={!active || !!error} onClick={() => update(zoom(camera, 1.25))}>{t.zoomOut}</button>
        </ToolbarMenu>
      </div>
    </div>}
    {(error || item.unavailable) && <p role="alert">{item.unavailable ?? text('此环境暂不能显示 WebGL2 三维场景；原式与 OBJ 仍可用。', 'This environment cannot display the WebGL2 scene; source and OBJ remain available.')} {error}</p>}
    {controlError && <p role="status">{controlError}</p>}
    <GraphFooter t={t} renderStatus={renderStatus} status={exportStatus} information={<>
        {data && <>
          <p>{count} {text('个实际三角形', 'actual triangles')} · {t.samplePoints}: {points}</p>
          <p>{t.graphRange}: {data.bounds[0].map((lo, i) => <span key={i}>{axisNames[i]} [{lo.toPrecision(6)} … {data.bounds[1][i]!.toPrecision(6)}] </span>)}</p>
          <p>{data.sampled ? text('有限机器采样，曲面边界未经认证。', 'Finite machine samples; no certified surface boundary.') : text('内核生成的有限图元几何。', 'Finite primitive geometry generated by the kernel.')} {t.skippedSamples}: {data.skipped}</p>
          <details onToggle={e => setShowData(e.currentTarget.open)}><summary>{text('内核网格数据', 'Kernel mesh data')}</summary>{showData && <pre>{JSON.stringify(data, null, 2)}</pre>}</details>
        </>}
        <details><summary>{text('原始三维表达式', 'Original 3D expression')}</summary><pre>{source}</pre></details>
      </>} exports={data && <ToolbarMenu label={t.graphExport} icon={<>{t.graphExport}<WorkbenchIcon name="chevron" /></>} className="graph-export">
        <ArtifactButton onStatus={setExportStatus} kernel={kernel} t={t} title="OpenMath-scene" label={text('导出 OBJ', 'Export OBJ')} version={data} enabled={active} request={() => ({ type: 'export_scene3_d', data, title: 'OpenMath' })} />
        <ArtifactButton onStatus={setExportStatus} kernel={kernel} t={t} title="OpenMath-scene" label={t.exportPng} version={data} enabled={active && !error} createArtifact={() => canvasPngArtifact(canvas.current!)} />
      </ToolbarMenu>} />
  </figure>;
}
