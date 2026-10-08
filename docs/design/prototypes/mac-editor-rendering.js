/* Review prototype only. No kernel, model, native text engine or math renderer. */
const $ = id => document.getElementById(id);
const esc = value => String(value ?? '').replace(/[&<>"']/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]));
const cells = {
  a: { source: 'let a = 2', sequence: 0, selection: [9, 9], history: [], value: '2', resultSource: 'let a = 2', status: 'current', composition: false },
  b: { source: 'a+1', sequence: 0, selection: [3, 3], history: [], value: '3', resultSource: 'a+1', status: 'current', composition: false },
};
let active = 'a', panel = 'results', editorGeneration = 1, requestGeneration = 0, candidate = null, diagnostic = null;
let run = null, runGeneration = 0, pageIndex = 0, camera = { zoom: 1, offset: 0 }, mediaVisible = true;
let userReduce = false, pendingCopy = '', opener = null;
const reduceSystem = matchMedia('(prefers-reduced-motion: reduce)');
const status = text => { $('status').textContent = text; };
const activeState = () => cells[active];

// Exact scalar-boundary mapping for reviewing range guards, not an AppKit port.
function indexPairs(text) {
  const pairs = [[0, 0]];
  let utf16 = 0, utf8 = 0;
  for (const scalar of text) {
    utf16 += scalar.length;
    utf8 += new TextEncoder().encode(scalar).length;
    pairs.push([utf16, utf8]);
  }
  return pairs;
}
function utf16ToByte(text, offset) {
  if (!Number.isSafeInteger(offset) || offset < 0) return null;
  return indexPairs(text).find(pair => pair[0] === offset)?.[1] ?? null;
}
function byteToUtf16(text, offset) {
  if (!Number.isSafeInteger(offset) || offset < 0) return null;
  return indexPairs(text).find(pair => pair[1] === offset)?.[0] ?? null;
}
function graphemeBoundary(text, position) {
  if (position === 0 || position === text.length) return true;
  return [...new Intl.Segmenter('zh', { granularity: 'grapheme' }).segment(text)].some(segment => segment.index === position);
}
function keySnapshot() {
  const state = activeState();
  return { cell: active, editor: editorGeneration, sequence: state.sequence, source: state.source, selection: [...state.selection], dialect: $('dialect').value };
}
function matches(key, includeCursor = true) {
  const now = keySnapshot();
  return key.cell === now.cell && key.editor === now.editor && key.sequence === now.sequence && key.source === now.source && key.dialect === now.dialect
    && (!includeCursor || key.selection.every((n, i) => n === now.selection[i])) && !activeState().composition;
}
function invalidateSuggestions() {
  requestGeneration++;
  candidate = null;
  diagnostic = null;
  $('suggestion-slot').replaceChildren();
  $('diagnostic-slot').replaceChildren();
}
function rememberSelection() {
  const state = activeState();
  state.selection = [$('source').selectionStart, $('source').selectionEnd];
}
function invalidateResult() {
  activeState().status = 'stale';
  if (active === 'a') cells.b.status = 'stale';
  $('doc-status').textContent = '示例源码已修改';
}
function edited(text, selection, remember = true) {
  const state = activeState();
  if (remember && text !== state.source) state.history.push({ source: state.source, selection: [...state.selection] });
  state.source = text;
  state.selection = selection;
  state.sequence++;
  invalidateSuggestions();
  invalidateResult();
  renderTextState();
}
function replaceBytes(span, replacement, key) {
  const state = activeState();
  if (!matches(key) || state.composition) { status('建议已过期或输入正在合成；原源码保留。'); return false; }
  const start = byteToUtf16(state.source, span.start), end = byteToUtf16(state.source, span.end);
  if (start === null || end === null || start > end || !graphemeBoundary(state.source, start) || !graphemeBoundary(state.source, end)) {
    status('无效源码范围；没有裁剪范围或修改文字。'); return false;
  }
  const next = state.source.slice(0, start) + replacement + state.source.slice(end), caret = start + replacement.length;
  edited(next, [caret, caret]);
  $('source').value = next;
  $('source').setSelectionRange(caret, caret);
  $('source').focus({ preventScroll: true });
  status('示例：一次替换和一次可撤销编辑；正式版需原生事务及持久化回执。');
  return true;
}
function renderTextState() {
  const state = activeState();
  $('gutter').textContent = state.source.split(/\r\n|\r|\n/).map((_, i) => i + 1).join('\n');
  $('editor-state').textContent = state.composition ? '输入正在合成 · 不接受补全、修复或运行' : '源码草稿已保留 · 此页不进行语法分析';
  $('complete').disabled = state.composition;
  $('ghost').disabled = state.composition;
  $('hover').disabled = state.composition;
  $('demo-run').disabled = !!run || Object.values(cells).some(c => c.composition);
  $('compose-demo').checked = state.composition;
  $('undo').disabled = state.history.length === 0 || state.composition;
  const trimmed = state.source.trim();
  $('preview-body').textContent = trimmed === 'let a = 2' ? 'a = 2' : trimmed === 'let a = 5' ? 'a = 5' : trimmed === 'a+1' ? 'a + 1' : '当前源码的排版预览未接通；源码保持。';
  renderResults();
}
function renderResults() {
  const state = activeState(), companion = cells[active === 'a' ? 'b' : 'a'];
  const stateNames = { current: '已接纳样例', stale: '上次结果 · 已过期', refreshing: '上次结果 · 演示重新计算中' };
  $('result-status').textContent = `${stateNames[state.status]} · 对应 ${state.resultSource}`;
  $('result-status').className = 'result-status' + (state.status === 'stale' ? ' warning' : '');
  $('result-value').textContent = state.value;
  $('companion-status').textContent = stateNames[companion.status] + ' · 保留原生产来源';
  $('companion-status').className = 'result-status' + (companion.status === 'stale' ? ' warning' : '');
  $('companion-value').textContent = companion.value;
}
function selectCell(id) {
  rememberSelection();
  // A native composing editor must be pinned; this mock also refuses a switch.
  if (activeState().composition) { status('当前输入正在合成，编辑 owner 保留；请完成输入后再切换。'); return; }
  active = id;
  editorGeneration++;
  invalidateSuggestions();
  const state = activeState(), companionId = active === 'a' ? 'b' : 'a';
  $('source').value = state.source;
  $('source').setSelectionRange(...state.selection);
  $('cell-title').textContent = active === 'a' ? '1 · 参数定义' : '2 · 依赖结果';
  $('companion-title').textContent = active === 'a' ? '2 · 依赖结果' : '1 · 参数定义';
  $('companion-source').textContent = cells[companionId].source;
  document.querySelectorAll('[data-cell]').forEach(n => n.setAttribute('aria-selected', String(n.dataset.cell === active)));
  renderTextState();
}
function requestCandidate(type) {
  rememberSelection();
  const state = activeState();
  if (state.composition || state.selection[0] !== state.selection[1]) return;
  invalidateSuggestions();
  const key = keySnapshot(), token = requestGeneration;
  $('editor-state').textContent = type === 'ghost' ? '正在演示获取建议，正文保持…' : '正在演示本地候选…';
  setTimeout(() => {
    if (token !== requestGeneration || !matches(key)) return;
    const cursor = key.selection[0], end = utf16ToByte(key.source, cursor);
    if (end === null) { status('光标不在完整 Unicode 边界，未创建候选。'); return; }
    const word = /[A-Za-z_]+$/.exec(key.source.slice(0, cursor));
    const start = type === 'ghost' ? end : utf16ToByte(key.source, cursor - (word?.[0].length ?? 0));
    if (start === null) return;
    candidate = { type, key, span: { start, end }, text: type === 'ghost' ? ' + 1' : 'sin(x)' };
    $('suggestion-slot').innerHTML = `<div class="suggestion"><span><span class="small muted">${type === 'ghost' ? 'AI 建议样例 · 未写入源码' : '本地补全样例'}</span><br><span class="ghost-text">${esc(candidate.text)}</span></span><button id="accept-candidate">接受</button>${type === 'ghost' ? '<button id="accept-part">接受一段</button>' : ''}<button id="dismiss-candidate">关闭</button></div>`;
    $('accept-candidate').onclick = acceptCandidate;
    if ($('accept-part')) $('accept-part').onclick = acceptPart;
    $('dismiss-candidate').onclick = () => { invalidateSuggestions(); $('source').focus(); };
    $('editor-state').textContent = '演示候选可接受；真实实现还需 Complete/FIM 回执。';
  }, 150);
}
function acceptCandidate() {
  if (!candidate) return;
  const value = candidate;
  replaceBytes(value.span, value.text, value.key);
}
function acceptPart() {
  if (!candidate || candidate.type !== 'ghost') return;
  const value = candidate;
  const prefix = value.text.startsWith(' + ') ? ' + ' : [...new Intl.Segmenter('zh', { granularity: 'grapheme' }).segment(value.text)][0]?.segment;
  if (!prefix || !replaceBytes(value.span, prefix, value.key)) return;
  const remaining = value.text.slice(prefix.length);
  if (!remaining) return;
  const key = keySnapshot(), anchor = utf16ToByte(key.source, key.selection[0]);
  candidate = { type: 'ghost', key, span: { start: anchor, end: anchor }, text: remaining };
  $('suggestion-slot').innerHTML = `<div class="suggestion"><span class="ghost-text">${esc(remaining)}</span><button id="accept-candidate">接受剩余</button><button id="dismiss-candidate">关闭</button></div>`;
  $('accept-candidate').onclick = acceptCandidate;
  $('dismiss-candidate').onclick = () => { invalidateSuggestions(); $('source').focus(); };
}
function loadError() {
  if (activeState().composition) return;
  edited('solve(x^2-1=0,x', [15, 15]);
  $('source').value = activeState().source;
  $('source').setSelectionRange(15, 15);
  const key = keySnapshot(), end = new TextEncoder().encode(key.source).length;
  diagnostic = { key, span: { start: end, end }, replacement: ')' };
  $('diagnostic-slot').innerHTML = '<div class="diag"><span><strong>缺少右括号 · 诊断样例</strong><br>定位到完整源码末尾。修复前再次核对源身份。</span><button id="apply-fix">补上右括号</button></div>';
  $('apply-fix').onclick = () => { if (diagnostic) replaceBytes(diagnostic.span, diagnostic.replacement, diagnostic.key); };
}
function beginComposition() {
  activeState().composition = true;
  invalidateSuggestions();
  renderTextState();
}
function endComposition() {
  activeState().composition = false;
  rememberSelection();
  renderTextState();
}
function undoEdit() {
  const state = activeState();
  if (state.composition || !state.history.length) return;
  const old = state.history.pop();
  edited(old.source, old.selection, false);
  $('source').value = old.source;
  $('source').setSelectionRange(...old.selection);
  status('示例编辑已撤销，历史结果仍按来源标记，未自动重算。');
}
function startRun() {
  if (run || Object.values(cells).some(c => c.composition)) return;
  const snapshot = { a: cells.a.source, b: cells.b.source, aSequence: cells.a.sequence, bSequence: cells.b.sequence, generation: ++runGeneration };
  run = snapshot;
  cells.a.status = cells.b.status = 'refreshing';
  $('demo-stop').disabled = false;
  renderTextState();
  status('演示开始；这不是内核计算。可以继续编辑以检查旧回复隔离。');
  setTimeout(() => {
    if (!run || snapshot.generation !== runGeneration) return;
    const stale = snapshot.aSequence !== cells.a.sequence || snapshot.bSequence !== cells.b.sequence;
    run = null;
    $('demo-stop').disabled = true;
    const fixture = snapshot.b.trim() === 'a+1' && ({ 'let a = 2': ['2', '3'], 'let a = 5': ['5', '6'] })[snapshot.a.trim()];
    if (stale || !fixture) {
      cells.a.status = cells.b.status = 'stale';
      status(stale ? '旧样例回复已拒绝，新草稿保留；没有接纳旧状态。' : '没有对应的计算样例；原结果保留为过期，不在浏览器求值。');
    } else {
      [cells.a.value, cells.b.value] = fixture;
      cells.a.resultSource = snapshot.a;
      cells.b.resultSource = snapshot.b;
      cells.a.status = cells.b.status = 'current';
      status('预设 3→6/原始样例已接纳（仅演示）；不是 CAS/原生验收。');
    }
    renderTextState();
  }, 600);
}
function stopRun() {
  if (!run) return;
  runGeneration++;
  run = null;
  cells.a.status = cells.b.status = 'stale';
  $('demo-stop').disabled = true;
  renderTextState();
  status('演示已停止，源码与已显示历史结果保留。');
}
const rows = [
  ['1', '1/3', '精确有理数'], ['2', '2/3', '精确有理数'], ['3', '1', '精确整数'],
  ['4', '4/3', '精确有理数'], ['5', '5/3', '精确有理数'], ['6', '2', '精确整数'],
];
function resultsPanel() {
  const page = rows.slice(pageIndex * 3, pageIndex * 3 + 3);
  return `<h2>数学结果</h2><p class="intro">下面为独立内容样例，排版与数值不是本页新计算。</p><section class="result-block"><h3>精确数与矩阵</h3><div class="formula-scroll"><span class="math"><span class="fraction"><span>1</span><span>3</span></span>　<span class="matrix"><span>1</span><span>2</span><span>3</span><span>4</span></span></span></div><p class="condition">精确值与机器近似分别保留；只显示实际验证。</p></section><section class="result-block"><h3>长式与精确 Root</h3><div class="formula-scroll"><div class="long-math">Root(x⁵ − x − 1, k)　+　(a² + b² + c² + d² + e²)</div></div><p class="caption">横向滚动，不缩小数学字号，不删条件。</p></section><section class="result-block"><div class="fallback-title">未知排版命令 · 保留原式</div><pre class="fallback">\\unsupported{x} + \\frac{1}{3}</pre><button id="open-fallback">查看 / 复制完整原式</button></section><section class="result-block"><h3>结构化值 · 只读分页</h3><div class="table-wrap"><table><thead><tr><th>行</th><th>值</th><th>性质</th></tr></thead><tbody>${page.map(r => '<tr>' + r.map(c => '<td>' + esc(c) + '</td>').join('') + '</tr>').join('')}</tbody></table></div><div class="pager"><span>${pageIndex * 3 + 1}–${pageIndex * 3 + 3} / 6 行样例</span><button id="previous-page" ${pageIndex === 0 ? 'disabled' : ''}>上一页</button><button id="next-page" ${pageIndex === 1 ? 'disabled' : ''}>下一页</button></div></section>`;
}
function stepsPanel() {
  return '<h2>步骤与证明</h2><p class="intro">结构示意；正式版只展示内核记录的 rule_id 和来源。</p><details open><summary>S1 · 规范化方程</summary><p>每个节点对应真实记录，保留 before/after/条件。</p></details><details><summary>S1.1 · 变换与适用条件</summary><p>展开状态绑定 result ID。源码改变后保留历史步骤并标过期。</p></details><details><summary>S2 · 验证候选</summary><p>精确恒等验证、数值残差与未验证分别显示。</p></details><p class="caption">没有记录时显示“本次未记录”，AI 讲解不能补造证明树。</p>';
}
function markdownPanel() {
  return '<h2>Markdown 与公式</h2><p class="intro">阅读流与源码分开；以下是原生布局的 HTML 示意。</p><div class="markdown"><h3>推导记录</h3><p>保留 <strong>粗体与 <em>嵌套强调</em></strong>，并让行内 <span style="font:16px Georgia">α² + β²</span> 与正文基线一致。</p><p>代码 <code>$不是公式$</code> 保持原样，链接只在用户点击后打开。</p><blockquote>条件与误差估计不能因排版被省略。</blockquote><pre>let f(x) = sin(x)\n# 代码块仅展示，不自动执行</pre><div class="image-placeholder">[远程图片未加载] · 图像说明保留</div><table><thead><tr><th>记录</th><th>状态</th></tr></thead><tbody><tr><td>当前源码</td><td>单独编辑</td></tr><tr><td>原始公式</td><td>完整复制</td></tr></tbody></table><p class="caption">首版表格/图形跨块拖选不冒充完整支持，提供全文 Markdown 复制。</p></div>';
}
const points = [[-2, 4], [-1.5, 2.25], [-1, 1], [-.5, .25], [0, 0], [.5, .25], [1, 1], [1.5, 2.25], [2, 4]];
function plotPanel() {
  return `<h2>图形与相机</h2><p class="intro">静态坐标样例；视窗变化不修改原数据或求数学函数。</p><svg class="graph" viewBox="0 0 300 205" role="img" aria-label="预设采样点示意"><defs><clipPath id="plot-clip"><rect x="31" y="14" width="251" height="159"/></clipPath></defs><path d="M31 14V173H282" stroke="var(--line)" fill="none"/><text id="tick-left" class="graph-text" x="31" y="192">−2</text><text id="tick-mid" class="graph-text" x="155" y="192">0</text><text id="tick-right" class="graph-text" x="269" y="192">2</text><g clip-path="url(#plot-clip)"><polyline id="sample-line" fill="none" stroke="var(--accent)" stroke-width="2" points=""/></g></svg><div class="graph-controls"><button id="zoom-in" aria-label="放大视窗">＋</button><button id="zoom-out" aria-label="缩小视窗">−</button><button id="pan-left">向左</button><button id="reset-camera">复位</button></div><p id="camera-status" class="caption"></p><details><summary>查看实际样例坐标</summary><pre class="fallback">${esc(JSON.stringify(points))}</pre></details><div class="divider"></div><h3 style="margin-top:19px">三维 · 原生 Metal</h3><img class="image-preview" src="assets/editor-watermelon.png" alt="既有 .3 西瓜场景的验收截图，静态参考"><p class="caption">这里是既有 .3 的静态验收截图，不是新 Metal 画面或当前计算结果。</p><button id="gpu-unavailable" style="margin-top:9px">演示 renderer 不可用</button><p id="gpu-status" class="caption">正式版核对真实 GPU/frame/result 身份。</p>`;
}
function renderPanel() {
  const functions = { results: resultsPanel, steps: stepsPanel, markdown: markdownPanel, plot: plotPanel };
  $('inspector-content').innerHTML = functions[panel]();
  $('inspector-content').classList.add('enter');
  document.querySelectorAll('[data-panel]').forEach(n => n.setAttribute('aria-selected', String(n.dataset.panel === panel)));
  if (panel === 'results') {
    $('next-page').onclick = () => { pageIndex = Math.min(1, pageIndex + 1); renderPanel(); };
    $('previous-page').onclick = () => { pageIndex = Math.max(0, pageIndex - 1); renderPanel(); };
    $('open-fallback').onclick = () => showSource('\\unsupported{x} + \\frac{1}{3}', '排版不支持 · 原 LaTeX 完整保留');
  } else if (panel === 'plot') {
    $('zoom-in').onclick = () => { camera.zoom = Math.min(4, camera.zoom * 1.25); drawSample(); };
    $('zoom-out').onclick = () => { camera.zoom = Math.max(.5, camera.zoom / 1.25); drawSample(); };
    $('pan-left').onclick = () => { camera.offset -= .25; drawSample(); };
    $('reset-camera').onclick = () => { camera = { zoom: 1, offset: 0 }; drawSample(); };
    $('gpu-unavailable').onclick = () => { mediaVisible = false; $('gpu-status').textContent = '三维绘制不可用（示例）· 原式/数据/OBJ仍可查，不冒称新图已绘制。'; document.querySelector('.image-preview').hidden = true; };
    if (!mediaVisible) { document.querySelector('.image-preview').hidden = true; $('gpu-status').textContent = '三维绘制不可用（示例）· 仍保留数据与原式。'; }
    drawSample();
  }
}
function drawSample() {
  const position = points.map(([x, y]) => [31 + 251 * ((x + camera.offset) * camera.zoom + 2) / 4, 173 - 159 * y / 4]);
  $('sample-line').setAttribute('points', position.map(p => p.join(',')).join(' '));
  $('tick-left').textContent = (-2/camera.zoom-camera.offset).toFixed(2);
  $('tick-mid').textContent = (-camera.offset).toFixed(2);
  $('tick-right').textContent = (2/camera.zoom-camera.offset).toFixed(2);
  $('camera-status').textContent = `相机缩放 ${camera.zoom.toFixed(2)} · 平移 ${camera.offset.toFixed(2)}；9 个原坐标保持。`;
}
function showSource(text, origin) {
  pendingCopy = text;
  opener = document.activeElement;
  $('source-origin').textContent = origin;
  $('source-full').textContent = text;
  $('source-dialog').showModal();
}
async function copySource(text, origin) {
  try { await navigator.clipboard.writeText(text); status(origin + ' · 剪贴板写入完成。'); }
  catch { showSource(text, origin + ' · 剪贴板未获确认，请选取原文复制。'); }
}
$('source').addEventListener('input', event => {
  const selection = [event.target.selectionStart, event.target.selectionEnd];
  edited(event.target.value, selection);
});
for (const name of ['keyup', 'click', 'select']) $('source').addEventListener(name, () => {
  const prior = [...activeState().selection];
  rememberSelection();
  if (prior.some((n, i) => n !== activeState().selection[i])) invalidateSuggestions();
});
$('source').addEventListener('compositionstart', beginComposition);
$('source').addEventListener('compositionend', endComposition);
$('source').addEventListener('scroll', () => { $('gutter').scrollTop = $('source').scrollTop; });
$('source').addEventListener('keydown', event => {
  if (activeState().composition || event.isComposing) return;
  if (event.key === 'Tab' && candidate) { event.preventDefault(); acceptCandidate(); }
  if (event.key === 'Escape') { invalidateSuggestions(); }
});
window.addEventListener('keydown', event => {
  if (event.key !== 'Escape' || event.isComposing || activeState().composition || $('source-dialog').open || !candidate) return;
  invalidateSuggestions();
  $('source').focus({ preventScroll: true });
});
$('complete').onclick = () => requestCandidate('local');
$('ghost').onclick = () => requestCandidate('ghost');
$('load-error').onclick = loadError;
$('hover').onclick = () => showSource('sin(x)\n返回三角函数表达式；实际 Hover 来自当前元数据/定义快照。', '函数帮助样例 · 不读取当前 CAS 定义');
$('undo').onclick = undoEdit;
$('demo-run').onclick = startRun;
$('demo-stop').onclick = stopRun;
$('copy-result').onclick = () => copySource(activeState().value, activeState().status === 'current' ? '样例完整结果' : '历史结果（已过期）');
$('show-source').onclick = () => showSource(activeState().value, '结果对应源码：' + activeState().resultSource + (activeState().status !== 'current' ? ' · 历史结果' : ''));
$('close-dialog').onclick = () => $('source-dialog').close();
$('copy-dialog-source').onclick = () => copySource(pendingCopy, '完整原式');
$('source-dialog').addEventListener('close', () => { if (opener?.isConnected) opener.focus(); else $('source').focus(); });
$('switch-cell').onclick = () => selectCell(active === 'a' ? 'b' : 'a');
document.querySelectorAll('[data-cell]').forEach(n => n.onclick = () => selectCell(n.dataset.cell));
document.querySelectorAll('[data-panel]').forEach(n => n.onclick = () => { panel = n.dataset.panel; renderPanel(); });
$('dialect').onchange = () => { editorGeneration++; invalidateSuggestions(); status('方言已改变，旧分析/候选已失效；此草案不运行解析器。'); };
$('wrap').onchange = event => { $('source').classList.toggle('no-wrap', !event.target.checked); status('只改变视觉换行，原源码没有增加或删除换行。'); };
$('theme').onchange = event => { document.documentElement.dataset.theme = event.target.value; };
$('compose-demo').onchange = event => event.target.checked ? beginComposition() : endComposition();
$('toggle-inspector').onclick = () => { const on = $('workspace').classList.toggle('focus-inspector'); $('toggle-inspector').textContent = on ? '返回笔记本' : '检查面板'; };
function updateMotion() {
  const reduced = userReduce || reduceSystem.matches;
  document.body.classList.toggle('reduce', reduced);
  $('reduce').checked = reduced;
  $('reduce').disabled = reduceSystem.matches;
}
$('reduce').onchange = event => { userReduce = event.target.checked; updateMotion(); };
reduceSystem.addEventListener('change', updateMotion);
renderTextState();
renderPanel();
updateMotion();
