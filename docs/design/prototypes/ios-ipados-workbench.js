/* An offline design study. All results and conversations are preset examples. */
const $ = (selector, root = document) => root.querySelector(selector);
const $$ = (selector, root = document) => [...root.querySelectorAll(selector)];
const icon = (name, small = false) => `<svg class="ic${small ? ' sm' : ''}" aria-hidden="true"><use href="#i-${name}"></use></svg>`;
const escapeHTML = text => text.replace(/[&<>"']/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[c]);
const originalSource = 'solve(x^2 - 5*x + 6 = 0, x)';
const devices = ['phone', 'pad'].map(id => ({ id, el: $(`#${id}-app`), tab: 'steps', source: originalSource, editing: false, overlay: null, restoreFocus: null }));
let currentView = 'overview';
let currentOrientation = 'landscape';

function statusBar(phone) {
  return `<div class="status-bar"><span>${phone ? '9:41' : '9:41　10月22日 周四'}</span>${phone ? '<span class="dynamic-island"></span>' : ''}<div class="status-right">${phone ? '<span class="signal"><i></i><i></i><i></i><i></i></span>' : ''}${icon('wifi')}<svg class="ic battery" aria-hidden="true"><use href="#i-battery"></use></svg>${phone ? '' : '<span>100%</span>'}</div></div>`;
}
function tool(action, name, label, extra = '') {
  return `<button class="icon-button" data-action="${action}" aria-label="${label}" title="${label}" ${extra}>${icon(name)}</button>`;
}
function navigation(phone) {
  return phone
    ? `<header class="nav-bar"><button class="nav-back" data-action="library">${icon('back')}文稿</button><div class="nav-title">二次方程与图形${icon('down', true)}</div>${tool('run', 'play', '运行全部')}${tool('menu', 'more', '文稿菜单')}</header>`
    : `<header class="nav-bar"><div class="pad-brand"><span class="brand-mark">∑</span><strong>OpenMath</strong></div>${tool('outline', 'sidebar', '切换目录')}<div class="nav-title">二次方程与图形<small><span class="status-dot"></span> 已保存 · 内核就绪</small></div><span class="nav-space"></span>${tool('undo', 'undo', '撤销')}${tool('share', 'share', '分享和导出')}<span class="nav-divider"></span><button class="run-button" data-action="run">${icon('play')}运行</button>${tool('add', 'plus', '添加单元格')}${tool('inspector', 'inspector', '切换检查面板')}</header>`;
}
function outline() {
  return `<aside class="outline" aria-label="文稿目录"><div class="outline-heading"><span>目录</span><button data-action="library" aria-label="所有文稿">${icon('folder', true)}</button></div><p class="section-name">求解</p><button class="outline-row selected" data-action="locate" data-target="solve-cell"><span class="status-dot"></span><code>solve(x^2 - 5*x + 6…)</code></button><p class="section-name">图形</p><button class="outline-row" data-action="locate" data-target="plot-cell"><span class="status-dot"></span><code>plot(x^2 - 5*x + 6…)</code></button><button class="outline-row" data-action="locate" data-target="conclusion">${icon('book', true)}<span>观察与结论</span></button><div class="outline-spacer"></div><div class="outline-footer"><button data-action="library">${icon('folder', true)}所有文稿</button><button data-action="settings">${icon('gear', true)}设置</button><div class="outline-count">5 个单元格 · Modern</div></div></aside>`;
}
function plot() {
  // Fixed sample y = x² − 5x + 6. The visible plot range is x∈[0,5], y∈[−1,4].
  const left = 28, right = 322, top = 23, bottom = 188;
  const X = x => left + x / 5 * (right - left);
  const Y = y => bottom - (y + 1) / 5 * (bottom - top);
  let path = '';
  for (let n = 0; n <= 180; n++) {
    const x = n / 180 * 5, y = x * x - 5 * x + 6;
    path += `${n === 0 ? 'M' : 'L'}${X(x).toFixed(2)} ${Y(y).toFixed(2)} `;
  }
  let grid = '';
  for (let n = 0; n <= 5; n++) grid += `<path class="grid" d="M${X(n)} ${top}V${bottom}"/>`;
  for (let n = -1; n <= 4; n++) grid += `<path class="grid" d="M${left} ${Y(n)}H${right}"/>`;
  return `<svg class="plot" viewBox="0 0 350 225" role="img" aria-label="二次函数 y 等于 x 平方减五 x 加六，抛物线与 x 轴交于 2 和 3"><defs><clipPath id="plot-clip-${plot.counter}"><rect x="${left}" y="${top}" width="${right-left}" height="${bottom-top}"/></clipPath></defs>${grid}<path class="axis" d="M${left} ${Y(0)}H330M${left} ${bottom+5}V16"/><path class="curve" clip-path="url(#plot-clip-${plot.counter++})" d="${path}"/>${[1,2,3,4,5].map(x => `<text x="${X(x)}" y="${Y(0)+18}" text-anchor="middle">${x}</text>`).join('')}<text x="16" y="${Y(0)+4}">0</text><text x="16" y="${Y(2)+3}">2</text><text x="16" y="${Y(4)+3}">4</text><text class="axis-label" x="337" y="${Y(0)+5}">x</text><text class="axis-label" x="23" y="12">y</text><circle class="root" cx="${X(2)}" cy="${Y(0)}" r="4"/><circle class="root" cx="${X(3)}" cy="${Y(0)}" r="4"/><path d="M116 211h19" stroke="var(--blue)" stroke-width="2"/><text class="legend" x="142" y="215">y = x² − 5x + 6</text></svg>`;
}
plot.counter = 0;
function notebook() {
  return `<div class="notebook-scroll"><article class="notebook"><div class="document-kicker">数学笔记 / 代数</div><h1>二次方程与图形</h1><div class="document-meta"><span>今天 14:02</span><span>·</span><span class="status-dot"></span><span>本地已保存</span></div><h2>求解</h2><p class="prose">先求方程的根，再从图形中观察<br class="phone-break">它们与 <i>x</i> 轴交点的关系。</p><section class="cell solve-cell" data-cell="solve-cell" aria-label="求根单元格"><div class="source-well"><button class="source-line" data-action="edit" aria-label="编辑求根公式"><span class="fn">solve</span>(x^<span class="number">2</span> - <span class="number">5</span>*x + <span class="number">6</span> = <span class="number">0</span>, x)</button><div class="source-editor"><label>Modern<span>正在编辑</span></label><textarea aria-label="数学源码" spellcheck="false">${originalSource}</textarea></div></div><div class="result"><span><i>x</i> = 2,</span><span><i>x</i> = 3</span></div><div class="result-status">${icon('seal')}<span>精确解，已代入验证</span></div><div class="cell-actions"><button data-action="steps">步骤 ${icon('down', true)}</button><button data-action="plot">绘图</button><button data-action="copy">复制</button><span class="timing">2 个解 · 9 ms</span></div></section><h2 class="plot-title">看见这两个根</h2><section class="cell" data-cell="plot-cell" aria-label="二次函数图形"><div class="source-well"><button class="source-line" data-action="plot-info" aria-label="查看绘图源码"><span class="fn">plot</span>(x^<span class="number">2</span> - <span class="number">5</span>*x + <span class="number">6</span>, x, <span class="number">0</span>, <span class="number">5</span>)</button></div>${plot()}<div class="plot-footer"><span>与 x 轴交于 2、3</span><button data-action="plot-info">图形信息</button></div></section><section data-cell="conclusion"><h2>观察与结论</h2><p class="prose">图形的两个零点，与精确求解得到的根一致。</p><p class="marginal-note">代数给出精确值，图形帮助我们看清关系。</p></section></article></div>`;
}
function stepsPanel() {
  return `<div class="panel-scroll"><p class="panel-eyebrow">单元格 2 · 求解记录</p><h2 class="panel-title">二次方程的根</h2><div class="formula-summary math"><i>x</i><sup>2</sup> − 5<i>x</i> + 6 = 0</div><ol class="steps-list"><li><strong>分解因式</strong><div class="math">(<i>x</i> − 2)(<i>x</i> − 3) = 0</div><p>寻找乘积为 6、和为 5 的两个数。</p></li><li><strong>分别令因式为零</strong><div class="math"><i>x</i> − 2 = 0<br><i>x</i> − 3 = 0</div></li><li><strong>得到精确解</strong><div class="math"><i>x</i> = 2，<i>x</i> = 3</div></li></ol><div class="verification"><div class="verification-title">${icon('seal', true)}已代回原式验证</div><p>2² − 5 × 2 + 6 = 0<br>3² − 5 × 3 + 6 = 0</p></div><p class="panel-footnote">预置推导示例</p></div>`;
}
function assistantPanel(device) {
  return `<div class="panel-scroll"><p class="panel-eyebrow">当前笔记本</p><h2 class="panel-title">理解这两个根</h2><div class="user-message">这两个解和图形上的交点，有什么关系？</div><div class="agent-label">${icon('assistant', true)}助手<span>讨论 · 示例对话</span></div><p class="assistant-text">它们是同一件事的两种表达。令函数值为 0，就是在找曲线与 x 轴的交点。</p><div class="action-log"><div>${icon('book')}读取单元格 2、4<span class="success">✓</span></div><div>${icon('chart')}查看求根与图形结果<span class="success">✓</span></div><button data-action="locate" data-target="plot-cell">${icon('target', true)} 定位图形</button></div><div class="assistant-formula"><i>f</i>(2) = 0，<i>f</i>(3) = 0</div><p class="assistant-text">所以两个精确解，对应图形上的 (2, 0) 和 (3, 0)。</p></div><div class="composer"><div class="composer-box"><div class="context-chip">${icon('book', true)}当前笔记本</div><textarea aria-label="助手草稿" placeholder="继续提问，或引用单元格…">${escapeHTML(device.assistantDraft || '')}</textarea><div class="composer-tools"><button data-action="attachment" aria-label="添加附件">${icon('plus', true)}</button><button data-action="mode">讨论 ${icon('down', true)}</button><button class="model-choice" data-action="model">示例模型 ${icon('down', true)}</button><button class="send-button" data-action="send" aria-label="预览发送">${icon('arrow')}</button></div></div></div>`;
}
function variablesPanel() {
  return `<div class="panel-scroll"><p class="panel-eyebrow">当前笔记本</p><h2 class="panel-title">变量与定义</h2><table class="variable-table"><thead><tr><th>符号</th><th>状态</th></tr></thead><tbody><tr><td>x</td><td>求解自变量</td></tr></tbody></table><p class="panel-empty-note">当前文稿没有全局赋值。求解结果中的 x = 2、x = 3 是解集，不会覆盖变量。</p><p class="panel-footnote">预置状态示例</p></div>`;
}
function inspector(device, sheet = false) {
  return `<aside class="inspector" aria-label="检查面板"><header class="inspector-head"><div class="inspector-tabs" role="tablist" aria-label="检查面板内容">${[['assistant','助手'],['steps','步骤'],['variables','变量']].map(([tab, label]) => `<button role="tab" aria-selected="${device.tab === tab}" data-action="tab" data-tab="${tab}">${label}</button>`).join('')}</div>${sheet ? `<button class="sheet-close" data-action="close" aria-label="关闭检查面板">${icon('close')}</button>` : ''}</header>${device.tab === 'steps' ? stepsPanel() : device.tab === 'variables' ? variablesPanel() : assistantPanel(device)}</aside>`;
}
function bottomBar(pad = false) {
  return `<footer class="bottom-bar${pad ? ' pad-bottom' : ''}"><button data-action="outline" aria-label="目录">${icon('outline')}<span>目录</span></button><button class="add-cell" data-action="add">${icon('plus')}单元格</button><button data-action="inspector">${icon('inspector')}检查面板</button></footer>`;
}
function renderDevice(device) {
  const phone = device.id === 'phone';
  device.el.innerHTML = `${statusBar(phone)}${navigation(phone)}<div class="app-body">${phone ? '' : outline()}${notebook()}${phone ? '' : inspector(device)}</div>${bottomBar(!phone)}<span class="home-indicator"></span>`;
  // Only compact iPad shows the phone-style bottom toolbar.
  device.el.querySelector('.pad-bottom')?.setAttribute('hidden', '');
}
devices.forEach(renderDevice);

function isCompact(device) { return device.id === 'phone' || currentOrientation === 'compact'; }
function dismiss(device) {
  const draft = $('.overlay .composer textarea', device.el);
  if (draft) device.assistantDraft = draft.value;
  device.el.querySelector('.overlay')?.remove();
  device.el.querySelector('.library-view')?.remove();
  device.overlay = null;
  device.restoreFocus?.focus({ preventScroll: true });
}
function showSheet(device, content, extra = '') {
  dismiss(device);
  device.restoreFocus = document.activeElement;
  const overlay = document.createElement('div');
  overlay.className = 'overlay';
  overlay.innerHTML = `<section class="sheet ${extra}" role="dialog" aria-modal="true" aria-label="${extra === 'menu-sheet' ? '操作菜单' : '辅助面板'}"><div class="grabber"></div>${content}</section>`;
  overlay.addEventListener('click', event => { if (event.target === overlay) dismiss(device); });
  device.el.append(overlay);
  device.overlay = overlay;
  const focusable = $('[aria-selected="true"], button, textarea', overlay);
  focusable?.focus({ preventScroll: true });
}
function openInspector(device, tab = device.tab) {
  const draft = $('.app-body > .inspector .composer textarea', device.el);
  if (draft) device.assistantDraft = draft.value;
  device.tab = tab;
  if (isCompact(device)) {
    showSheet(device, inspector(device, true));
  } else {
    dismiss(device);
    const existing = $('.app-body > .inspector', device.el);
    if (existing) existing.outerHTML = inspector(device);
    else $('.app-body', device.el).insertAdjacentHTML('beforeend', inspector(device));
  }
}
function menu(device, title, items) {
  showSheet(device, `<header class="sheet-heading">${title}<button class="sheet-close" data-action="close" aria-label="关闭菜单">${icon('close')}</button></header><div class="menu-list">${items.map(([action, name, label]) => `<button data-action="${action}">${icon(name)}${label}</button>`).join('')}</div>`, 'menu-sheet');
}
let toastTimers = new Map();
function toast(device, text) {
  $('.toast', device.el)?.remove();
  clearTimeout(toastTimers.get(device.id));
  const element = document.createElement('div');
  element.className = 'toast'; element.setAttribute('role', 'status'); element.textContent = text;
  device.el.append(element);
  toastTimers.set(device.id, setTimeout(() => element.remove(), 2800));
}
function locate(device, target) {
  dismiss(device);
  const item = $(`[data-cell="${target}"]`, device.el);
  const scroll = $('.notebook-scroll', device.el);
  if (item) scroll.scrollTo({ top: item.offsetTop - scroll.offsetTop - 20, behavior: matchMedia('(prefers-reduced-motion: reduce)').matches ? 'instant' : 'smooth' });
}
function library(device) {
  dismiss(device);
  const view = document.createElement('section');
  view.className = 'library-view';
  view.setAttribute('aria-label', '文稿列表');
  view.innerHTML = `<div class="library-toolbar"><button data-action="close">${icon('back', true)} 返回笔记本</button>${tool('library-new','edit','新建文稿')}</div><div class="library-content"><h1>文稿</h1><div class="search-field">${icon('search', true)}<input aria-label="搜索文稿" placeholder="搜索文稿" style="border:0;background:none;outline:none;width:100%;font-size:14px"></div><div class="library-section">最近打开<span>存于此设备</span></div><div class="document-list">${[['二次方程与图形','x²','今天 14:02 · 5 个单元格'],['微积分练习','∫','昨天 20:16 · 8 个单元格'],['矩阵与线性变换','A','10月20日 · 6 个单元格']].map(([title, symbol, meta], index) => `<button class="document-row" data-action="${index ? 'sample-document' : 'open-document'}" data-title="${title}"><span class="document-cover">${symbol}</span><span><strong>${title}</strong><small>${meta}</small></span></button>`).join('')}</div></div><div class="library-bottom"><button data-action="browse">${icon('folder', true)} 从文件打开</button><button data-action="settings">${icon('gear', true)} 设置</button></div>`;
  device.el.append(view);
  $('input', view).addEventListener('input', event => {
    $$('.document-row', view).forEach(row => row.hidden = !row.dataset.title.includes(event.target.value));
  });
}
function keyboard() {
  const rows = ['qwertyuiop','asdfghjkl','zxcvbnm'];
  return `<div class="keyboard-area" aria-label="键盘布局示意"><div class="math-accessory">${['x','²','√','(',')','=','π'].map(key => `<button data-action="insert-key" data-key="${key === '²' ? '^2' : key}" aria-label="插入 ${key}">${key}</button>`).join('')}<button class="keyboard-done" data-action="edit-done">完成</button></div><div class="keyboard-hint"><span>数学源码</span><span>Modern</span><span>⌘ ↵ 运行</span></div>${rows.map((row,i) => `<div class="keyboard-row">${i===2 ? '<button class="wide-key" data-action="shift">⇧</button>' : ''}${[...row].map(key => `<button data-action="insert-key" data-key="${key}">${key}</button>`).join('')}${i===2 ? '<button class="wide-key" data-action="backspace" aria-label="删除">⌫</button>' : ''}</div>`).join('')}<div class="keyboard-row"><button class="wide-key" data-action="key-numbers">123</button><button class="space-key" data-action="insert-key" data-key=" ">空格</button><button class="run-key" data-action="run">运行</button></div><div class="keyboard-pad-bottom"></div></div>`;
}
function edit(device) {
  dismiss(device);
  if (device.editing) return;
  device.editing = true;
  device.el.classList.add('editing');
  const input = $('.source-editor textarea', device.el);
  input.value = device.source;
  device.el.insertAdjacentHTML('beforeend', keyboard());
  locate(device, 'solve-cell');
  input.focus({ preventScroll: true });
  input.setSelectionRange(7, 7);
}
function updateSource(device, source) {
  device.source = source;
  const stale = source !== originalSource;
  device.el.classList.toggle('stale', stale);
  $('.result-status span', device.el).textContent = stale ? '源码已更改 · 上次结果，需重新运行' : '精确解，已代入验证';
}
function endEdit(device) {
  updateSource(device, $('.source-editor textarea', device.el).value);
  device.editing = false; device.el.classList.remove('editing');
  $('.keyboard-area', device.el)?.remove();
  $('.source-line', device.el).textContent = device.source;
}
function run(device) {
  if (device.editing) endEdit(device);
  if (device.source !== originalSource) {
    toast(device, '草稿保留了输入；此处仅展示预置方程结果。');
    return;
  }
  const status = $('.result-status span', device.el);
  status.textContent = '正在计算…';
  setTimeout(() => { if (device.source === originalSource) { status.textContent = '精确解，已代入验证'; toast(device, '已展示预置结果：x = 2，x = 3'); } }, 450);
}
function addCell(device, text = false) {
  dismiss(device);
  const cell = document.createElement('section');
  cell.className = 'new-cell';
  cell.innerHTML = `<textarea aria-label="新增${text ? '文字' : '数学'}单元格" placeholder="${text ? '写下你的观察…' : '输入数学表达式…'}"></textarea>`;
  $('.notebook', device.el).append(cell);
  const scroll = $('.notebook-scroll', device.el);
  scroll.scrollTo({ top: scroll.scrollHeight, behavior: 'smooth' });
  $('textarea', cell).focus({ preventScroll: true });
}
async function handle(device, button) {
  const action = button.dataset.action;
  switch (action) {
    case 'library': library(device); break;
    case 'close': dismiss(device); break;
    case 'open-document': dismiss(device); break;
    case 'sample-document': toast(device, '草稿仅展开「二次方程与图形」示例。'); break;
    case 'library-new': toast(device, '此处将创建新的 .omnb 文稿。'); break;
    case 'browse': toast(device, '此处将打开系统「文件」选择器。'); break;
    case 'outline': {
      if (device.id === 'phone' || currentOrientation !== 'landscape') {
        showSheet(device, `<header class="sheet-heading">目录<button class="sheet-close" data-action="close" aria-label="关闭目录">${icon('close')}</button></header>${outline()}`);
      } else { const panel = $('.app-body > .outline', device.el); panel.hidden = !panel.hidden; }
      break;
    }
    case 'inspector': {
      const panel = $('.app-body > .inspector', device.el);
      if (!isCompact(device) && panel) panel.remove(); else openInspector(device);
      break;
    }
    case 'steps': openInspector(device, 'steps'); break;
    case 'tab': {
      device.tab = button.dataset.tab;
      const current = button.closest('.inspector');
      const draft = $('textarea', current)?.value;
      current.outerHTML = inspector(device, !!device.overlay);
      if (draft !== undefined) device.assistantDraft = draft;
      if (device.tab === 'assistant' && device.assistantDraft) $('textarea', device.overlay || $('.app-body > .inspector', device.el)).value = device.assistantDraft;
      $(`[data-tab="${device.tab}"]`, device.overlay || $('.app-body > .inspector', device.el))?.focus({ preventScroll: true });
      break;
    }
    case 'locate': locate(device, button.dataset.target); break;
    case 'plot': locate(device, 'plot-cell'); break;
    case 'menu': menu(device, '二次方程与图形', [['share','share','分享与导出'],['add','plus','添加单元格'],['outline','outline','文稿目录'],['settings','gear','设置']]); break;
    case 'share': menu(device, '分享与导出', [['export-omnb','book','分享 .omnb 文稿'],['export-md','share','导出 Markdown'],['export-tex','share','导出 LaTeX']]); break;
    case 'add': menu(device, '添加单元格', [['add-math','plus','数学'],['add-text','edit','文字']]); break;
    case 'add-math': addCell(device); break;
    case 'add-text': addCell(device, true); break;
    case 'edit': edit(device); break;
    case 'edit-done': endEdit(device); break;
    case 'run': run(device); break;
    case 'undo': updateSource(device, originalSource); $('.source-editor textarea', device.el).value = originalSource; if (device.editing) endEdit(device); else { $('.source-line', device.el).textContent = originalSource; $('.result-status span', device.el).textContent = '精确解，已代入验证'; $('.result', device.el).style.opacity = ''; } toast(device, '已恢复示例源码'); break;
    case 'insert-key': { const input = $('.source-editor textarea', device.el); input.setRangeText(button.dataset.key, input.selectionStart, input.selectionEnd, 'end'); updateSource(device, input.value); input.focus({ preventScroll: true }); break; }
    case 'backspace': { const input = $('.source-editor textarea', device.el); input.setRangeText('', Math.max(0, input.selectionStart - (input.selectionStart===input.selectionEnd ? 1 : 0)), input.selectionEnd, 'end'); updateSource(device, input.value); input.focus({ preventScroll: true }); break; }
    case 'shift': toast(device, '键盘为布局示意；可用电脑键盘直接编辑。'); break;
    case 'key-numbers': toast(device, '可用上方数学符号行，或电脑键盘输入数字。'); break;
    case 'copy': {
      try { await navigator.clipboard.writeText('x = 2, x = 3'); toast(device, '已复制：x = 2, x = 3'); }
      catch { toast(device, '结果：x = 2, x = 3'); }
      break;
    }
    case 'plot-info': showSheet(device, `<header class="sheet-heading">图形信息<button class="sheet-close" data-action="close" aria-label="关闭图形信息">${icon('close')}</button></header><div class="panel-scroll"><p class="panel-eyebrow">二维函数图 · 预置示例</p><div class="formula-summary math"><i>y</i> = <i>x</i><sup>2</sup> − 5<i>x</i> + 6</div>${plot()}<p class="panel-empty-note">横轴范围 [0, 5]，纵轴范围 [−1, 4]。两个零点为 2 和 3。草稿使用静态函数曲线，未连接内核采样。</p></div>`); break;
    case 'settings': menu(device, '设置', [['theme','gear','切换浅色 / 深色'],['model','assistant','模型与供应商'],['about','book','关于 OpenMath']]); break;
    case 'theme': toggleTheme(); break;
    case 'about': toast(device, 'OpenMath · iOS / iPadOS 工作台设计草稿 01'); break;
    case 'mode': toast(device, '草稿展示讨论模式；执行模式沿用现有操作记录。'); break;
    case 'model': toast(device, '实现时显示已配置模型；草稿未连接供应商。'); break;
    case 'attachment': toast(device, '此处将打开系统附件选择器。'); break;
    case 'send': { const input = $('textarea', button.closest('.composer')); toast(device, input.value.trim() ? '已保留助手草稿，未发送请求。' : '输入问题后可预览；草稿不发送请求。'); break; }
    default: if (action.startsWith('export-')) toast(device, '此处将使用系统分享面板；草稿不生成文件。');
  }
}
for (const device of devices) {
  device.el.addEventListener('input', event => {
    if (event.target.matches('.source-editor textarea')) updateSource(device, event.target.value);
    else if (event.target.matches('.composer textarea')) device.assistantDraft = event.target.value;
  });
  device.el.addEventListener('click', event => {
    const button = event.target.closest('[data-action]');
    if (button) handle(device, button);
  });
  device.el.addEventListener('keydown', event => {
    if (event.isComposing) return;
    if (event.key === 'Escape') { if (device.overlay || $('.library-view', device.el)) dismiss(device); else if (device.editing) endEdit(device); }
    if ((event.metaKey || event.ctrlKey) && event.key === 'Enter') { event.preventDefault(); run(device); }
    // Keep review keyboard focus within a sheet while it is open.
    if (event.key === 'Tab' && device.overlay) {
      const focusable = $$('button, textarea, input, select, [tabindex="0"]', device.overlay).filter(el => !el.disabled && !el.hidden);
      const first = focusable[0], last = focusable.at(-1);
      if (event.shiftKey && document.activeElement === first) { event.preventDefault(); last?.focus(); }
      else if (!event.shiftKey && document.activeElement === last) { event.preventDefault(); first?.focus(); }
    }
  });
}
function toggleTheme() {
  const dark = document.documentElement.dataset.theme !== 'dark';
  document.documentElement.dataset.theme = dark ? 'dark' : 'light';
  $('#theme-switch').textContent = dark ? '浅色' : '深色';
  $('#theme-switch').setAttribute('aria-label', `切换${dark ? '浅' : '深'}色模式`);
}
function fitDevices() {
  const phoneSlot = $('.phone-slot'), padSlot = $('.pad-slot');
  const small = innerWidth <= 760;
  const phoneAvailable = $('.phone-study').clientWidth;
  if (phoneAvailable) {
    const s = Math.min(phoneAvailable / 420, 1);
    phoneSlot.style.width = `${420*s}px`; phoneSlot.style.height = `${892*s}px`;
    $('.phone').style.transform = `scale(${s})`;
  }
  const pad = $('.device.pad');
  const w = currentOrientation === 'portrait' ? 852 : currentOrientation === 'compact' ? 658 : 1212;
  const h = currentOrientation === 'portrait' ? 1130 : 852;
  const available = $('.pad-study').clientWidth;
  if (available) {
    const s = Math.min(available/w, 1, small ? 1 : currentView === 'overview' ? 700/h : 850/h);
    padSlot.style.width = `${w*s}px`; padSlot.style.height = `${h*s}px`;
    pad.style.transform = `scale(${s})`;
  }
}
function setView(view) {
  currentView = view;
  $('#stage').dataset.view = view;
  $$('#view-switch button').forEach(button => button.setAttribute('aria-pressed', String(button.dataset.view === view)));
  requestAnimationFrame(fitDevices);
}
function setOrientation(orientation) {
  currentOrientation = orientation;
  $('.device.pad').classList.remove('portrait', 'compact');
  devices[1].el.classList.remove('portrait', 'compact');
  if (orientation !== 'landscape') { $('.device.pad').classList.add(orientation); devices[1].el.classList.add(orientation); }
  $('.pad-bottom').hidden = orientation !== 'compact';
  $$('.orientation-control button').forEach(button => button.setAttribute('aria-pressed', String(button.dataset.orientation === orientation)));
  dismiss(devices[1]);
  requestAnimationFrame(fitDevices);
}
function setScene(scene) {
  for (const device of devices) {
    dismiss(device);
    if (device.editing) endEdit(device);
    if (['steps','assistant','variables'].includes(scene)) openInspector(device, scene);
    else if (scene === 'editing') edit(device);
    else if (scene === 'library') library(device);
    else { $('.notebook-scroll', device.el).scrollTop = 0; if (device.id === 'pad') openInspector(device, 'steps'); }
  }
}
$('#view-switch').addEventListener('click', event => { if (event.target.dataset.view) setView(event.target.dataset.view); });
$('.orientation-control').addEventListener('click', event => { if (event.target.dataset.orientation) setOrientation(event.target.dataset.orientation); });
$('#state-switch').addEventListener('change', event => setScene(event.target.value));
$('#theme-switch').addEventListener('click', toggleTheme);
window.addEventListener('resize', fitDevices);
const params = new URLSearchParams(location.search);
if (params.get('theme') === 'dark') toggleTheme();
if (['landscape','portrait','compact'].includes(params.get('orientation'))) setOrientation(params.get('orientation'));
if (['overview','phone','pad'].includes(params.get('view'))) setView(params.get('view'));
if (['reading','steps','assistant','variables','editing','library'].includes(params.get('scene'))) { $('#state-switch').value = params.get('scene'); setScene(params.get('scene')); }
requestAnimationFrame(fitDevices);
