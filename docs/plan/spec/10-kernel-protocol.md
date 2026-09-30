<!-- Extracted from docs/plan/PLAN.md sections [10]. PLAN.md is authoritative; keep in sync. -->

## 10. Kernel、响应式笔记本与 JSON 协议（om-kernel）

### 10.1 Session
```rust
// crates/om-kernel/src/session.rs
pub struct Session {
    eval: om_eval::Evaluator,
    pub notebook: Notebook,
    pub config: KernelConfig,
    interrupt: Arc<AtomicBool>,
    clock: Option<Arc<dyn Clock>>,
    jobs: BTreeMap<RequestId, om_llm::Job>,   // 进行中的 LLM 任务（见 11.4）
}
impl Session {
    pub fn new(config: KernelConfig, clock: Option<Arc<dyn Clock>>) -> Self;
    pub fn interrupt_handle(&self) -> Arc<AtomicBool>;  // 前端可跨线程设置
    pub fn handle(&mut self, req: Request) -> (Response, Vec<Event>); // 同步入口；所有前端共用
}
```

### 10.2 协议（`crates/om-kernel/src/protocol.rs`，全部 `#[derive(Serialize, Deserialize, ts_rs::TS)]`，`#[ts(export, export_to = "../../app/src/kernel/generated/")]`）

```rust
pub type CellId = String;       // 前端生成的 nanoid 风格字符串
pub type RequestId = String;

#[serde(tag = "type", rename_all = "snake_case")]
pub enum Request {
    Evaluate { cell_id: CellId, source: String, dialect: Dialect },
    UpsertCell { cell: CellInput },            // 编辑时同步源码（不执行）
    DeleteCell { cell_id: CellId },
    MoveCell { cell_id: CellId, to_index: u32 },
    RunAll,
    Preview { source: String, dialect: Dialect, cursor: Option<u32> },   // 每次按键（去抖 80ms）
    Complete { source: String, cursor: u32, dialect: Dialect },
    Hover { source: String, cursor: u32, dialect: Dialect },
    SamplePlot { request: PlotRequest },
    Interrupt,                                  // 同步置位；wasm 下不可用（见 12.10）
    LoadNotebook { file: NotebookFile },
    SaveNotebook,
    GetConfig, SetConfig { config: KernelConfig },
    // —— LLM ——
    LlmTranslate { request_id: RequestId, text: String, cell_id: Option<CellId> },
    LlmExplain { request_id: RequestId, cell_id: CellId, step_id: Option<String> },
    LlmComplete { request_id: RequestId, prefix: String, suffix: String, dialect: Dialect },
    LlmChat { request_id: RequestId, messages: Vec<ChatMessage> },
    LlmFixError { request_id: RequestId, cell_id: CellId },
    LlmTestProfile { request_id: RequestId, profile: String },
    LlmCancel { request_id: RequestId },
    // 仅 wasm 驱动使用：浏览器端 fetch 的数据回灌
    LlmHttpChunk { request_id: RequestId, chunk: String },
    LlmHttpEnd { request_id: RequestId, status: u16, error: Option<String> },
}
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Response {
    Ok, Error { message: String },
    Evaluated { cell_id: CellId, output: CellOutput, reran: Vec<CellId> },
    Preview(PreviewResult), Completions { items: Vec<CompletionItem>, from: u32, to: u32 },
    Hover { info: Option<HoverInfo> }, Plot { data: PlotData }, Notebook { file: NotebookFile },
    Config { config: KernelConfig },
    LlmStarted { request_id: RequestId, http: Option<HttpRequest> }, // http 非空表示“请前端代为发起”（wasm）
}
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    CellStatus { cell_id: CellId, status: CellStatus },          // Queued | Running | Done | Error | Stale
    CellOutput { cell_id: CellId, output: CellOutput },          // 依赖 cell 自动重算的结果
    LlmDelta { request_id: RequestId, text: String },            // 流式文本
    LlmToolCall { request_id: RequestId, name: String, arguments: String, result_summary: String },
    LlmSuggestion { request_id: RequestId, suggestion: Suggestion },
    LlmHttp { request_id: RequestId, http: HttpRequest },        // wasm：工具调用后的下一轮请求
    LlmDone { request_id: RequestId },
    LlmError { request_id: RequestId, message: String },
}
pub struct Envelope<T> { pub id: u64, pub body: T }              // 请求/响应按 id 配对；事件 id=0

pub struct PreviewResult { pub latex: Option<String>, pub diagnostics: Vec<Diagnostic>, pub tokens: Vec<(Span, TokenClass)>,
    pub dialect: Dialect, pub actions: Vec<CellAction> }
pub struct CellAction { pub label_key: String /*i18n key*/, pub source: String } // 例：裸方程 → "solve(x^2+2x=3, x)"
pub struct CellOutput { pub items: Vec<OutputItem>, pub messages: Vec<Message>, pub timing_ms: f64 }
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OutputItem {
    Expr { out_index: u32, input_form: String, modern_form: String, latex: String },
    Solutions { out_index: u32, input_form: String, modern_form: String, view: SolutionSetView,
                steps: Option<StepsView>, plot: Option<PlotRequest> },
    Plot { request: PlotRequest, data: PlotData },
    Error { message: String, span: Option<Span> },
}
pub struct SolutionSetView { pub kind: SolutionKind /*finite|all|none|region*/, pub vars: Vec<String>,
    pub solutions: Vec<SolutionView>, pub region_latex: Option<String>, pub intervals: Vec<IntervalView> }
pub struct SolutionView { pub bindings: Vec<BindingView>, pub condition_latex: Option<String>, pub verified: Verification }
pub struct BindingView { pub var: String, pub latex: String, pub input_form: String, pub modern_form: String,
    pub numeric: Option<String> /*N[…, 10] 的结果；复数 a+bi 形式*/ }
pub struct IntervalView { pub lo: Option<String>, pub hi: Option<String>, pub lo_closed: bool, pub hi_closed: bool,
    pub lo_value: Option<f64>, pub hi_value: Option<f64> }      // 用于数轴可视化
```

**类型生成：** `cargo test -p om-kernel export_bindings` 通过 ts-rs 生成 `app/src/kernel/generated/*.ts`，生成结果提交进仓库；CI 检查生成结果与仓库一致（`git diff --exit-code app/src/kernel/generated`）。

### 10.3 响应式笔记本（marimo 式）
```rust
pub struct Notebook { pub cells: Vec<Cell>, pub title: String }
pub struct Cell { pub id: CellId, pub kind: CellKind /*Math|Text|Ask*/, pub source: String, pub dialect: Dialect,
    pub output: Option<CellOutput>, pub status: CellStatus, pub defines: BTreeSet<Symbol>, pub uses: BTreeSet<Symbol>,
    pub exec_count: Option<u32> }
```
执行 `Evaluate{cell_id}` 的算法：
1. 解析；`defines` = 所有 `Set`/`SetDelayed` 左侧的头符号（`x = …` 取 `x`，`f[x_] := …` 取 `f`）；`uses` = 各语句自由符号 − 本 cell 的 defines − 模式变量名 − 内置符号。
2. **冲突检查**（`config.reactive == true` 时）：若某个 define 已被**另一个** cell 定义 → 本 cell 输出错误 `"{sym}" 已在第 {n} 个 cell 中定义`（i18n key `err.multiple_definitions`），不执行。
3. 清除本 cell **上一次**的 defines（`Clear`），再按语句顺序求值。
4. 计算受影响集合：`changed = old_defines ∪ new_defines`；BFS 找所有 `uses ∩ changed ≠ ∅` 的 cell，并对它们的 defines 继续传递；按笔记本顺序做拓扑排序（Kahn，同层按文档顺序）；存在环 → 环上的 cell 标 Error `err.cycle`。
5. `config.auto_run_dependents == true` → 依次执行这些 cell，发出 `Event::CellStatus`/`Event::CellOutput`，响应里的 `reran` 列出它们；否则标记为 `Stale`。
6. `reactive == false` 时退化为 Mathematica 式的顺序执行语义（无冲突检查、无自动重算）。
**测试：** A: `let a = 2`，B: `solve(x^2 = a, x)` → 修改 A 为 `let a = 9` 并执行 → B 自动重算为 `{{x->-3},{x->3}}`；两个 cell 都定义 `a` → 第二个报错；A 用 b、B 用 a → 环。

### 10.4 输出打包规则（`crates/om-kernel/src/output.rs`）
- 语句以 `;` 结尾 → 不产生 item（消息照常输出）。
- 结果头是 `List` 且形状为 `{{Rule…}…}`、`{}` 或 `{{}}`，**并且**来自 Solve/NSolve/SolveValues/FindRoot（通过 `eval.last_steps` 或调用标记判断）→ `OutputItem::Solutions`；`SolutionView.numeric` 用 `N[rhs, 10]`（精确解才填）；`plot` 用 10.6 的规则生成。
- 结果头是 `Plot`/`ContourPlot` → 采样后生成 `OutputItem::Plot`。
- 裸方程语句（顶层 `Equal` 且未被求值成 True/False）→ 仍输出 Expr，同时在 `PreviewResult.actions` 里给出 “求解 {v}”（每个自由变量一个，最多 3 个）动作。

### 10.5 绘图采样（`crates/om-kernel/src/plot.rs`）
```rust
pub struct PlotRequest { pub kind: PlotKind /*Function | Implicit*/, pub exprs: Vec<String> /*InputForm*/, pub var_x: String,
    pub var_y: Option<String>, pub x_range: (f64, f64), pub y_range: Option<(f64, f64)>,
    pub params: BTreeMap<String, f64>, pub points: Vec<(f64, f64)> /*高亮的解点*/,
    pub shade: Vec<(f64, f64)> /*x 轴区间着色，±inf 用 ±1e308*/, pub param_ranges: BTreeMap<String, (f64, f64)> }
pub struct PlotData { pub curves: Vec<Curve>, pub x_range: (f64, f64), pub y_range: (f64, f64) }
pub struct Curve { pub label: String, pub segments: Vec<Vec<(f64, f64)>> } // 不连续处断开
```
- **编译：** `om_eval::numeric::compile_f64(expr, &[vars]) -> Result<CompiledFn, CompileError>`：把 Expr 降为栈式指令序列（`PushConst, PushVar(i), Add(n), Mul(n), Pow, Call1(fn), …`），求值时遇复数结果返回 NaN。常见函数都直接对应 f64 方法。
- **一元函数自适应采样：** 均匀 400 点；对相邻三点夹角 > 10° 或一端为 NaN 的区间二分，最大深度 6；|Δy| > 0.5·(y 视窗高度) 且两侧导数符号相反 → 视为间断，断开折线。y 视窗默认取样本值 2%–98% 分位数并扩 10%。
- **隐函数（ContourPlot 方程 f(x,y)=0）：** 在 160×160 网格上用 marching squares，线性插值求交点，再把线段拼接成折线。
- **测试：** `sin(x)` 在 [0, 2π] 至少 400 点，最大误差 < 1e-3；`1/x` 在 [-1,1] 在 0 处断开成两段；圆 `x^2+y^2=1` 隐函数所有点满足 |r−1| < 0.02。

### 10.6 由 Solve 自动生成可视化（PlotRequest）
- 一元实方程 `lhs == rhs`（变量 x，域 Reals，或所有解都是实数）：两条曲线 `lhs`、`rhs`；`points` 为实数解 `(x_i, lhs(x_i))`；x 范围 = [min−2, max+2]，宽度至少 6，并以解集为中心；无实数解时取 [−5, 5]。
- 一元不等式：曲线 `lhs − rhs`；`shade` = 解区间。
- 二元方程组（两个变量、两个方程，至少一个实数解）：`Implicit`，每个方程一条隐函数曲线，`points` 为实数解。
- 含参数（除求解变量外的自由符号）：`params` 默认值 1（若 1 使分母为 0，则取 2），`param_ranges` 为 [−5, 5]；前端显示滑块，拖动时发送 `SamplePlot`，由 kernel 用代入参数后的**数值**解点更新 `points`（先对参数代入后的方程重新调用 `NSolve`）。
- 其他情况 → `plot: None`。

### 10.7 补全、悬停与预览
- `Complete`：定位光标所在的标识符前缀（Unicode 字母数字），候选 = 内置 DocEntry（现代方言展示小写名，Wolfram 展示原名）∪ 会话符号 ∪ 当前调用函数的关键字参数（`domain:` 等，仅当光标在 `solve(`… 参数内）∪ 片段模板（`solve` → `solve(${1:equation}, ${2:x})`）。打分：前缀匹配 > 驼峰/下划线首字母匹配 > 子序列匹配；同分按使用频率（内置表中 Solving/Algebra 类优先），最多返回 50 条。
- `Hover`：内置函数 → DocEntry（签名 + 摘要 + 例子）；用户符号 → 当前值的 InputForm（截断 200 字符）与定义所在 cell。
- `Preview`：解析 → 对**未求值**的表达式做 `canonicalize`，然后输出 LaTeX（不求值，所以不会触发耗时计算）；返回诊断、token 分类与动作。要求 < 5 ms（在 1 000 字符以内的输入上）。

### 10.8 配置（`KernelConfig`，TOML 文件 + 协议共用同一结构）
```toml
# macOS: ~/Library/Application Support/org.openmath.OpenMath/config.toml（directories::ProjectDirs::from("org","openmath","OpenMath")）
[general]
language = "auto"          # auto | zh-CN | en
dialect = "auto"           # auto | modern | wolfram
constants = "math"         # math（e/i 为常量）| strict
reactive = true
auto_run_dependents = true
show_steps = true
auto_plot = true
eval_timeout_ms = 30000

[llm]
enabled = true
translate = "deepseek"     # 各功能使用的 profile 名；空字符串表示关闭该功能
explain = "deepseek"
chat = "deepseek"
complete = "deepseek-fim"
fix = "deepseek"
send_context = true        # 是否把笔记本上下文（已定义符号、前几个 cell 源码）发给模型

[[llm.profiles]]
name = "deepseek"
kind = "openai_chat"       # openai_chat | anthropic | openai_fim | ollama_fim | mistral_fim
base_url = "https://api.deepseek.com/v1"
model = "deepseek-chat"
api_key_env = "DEEPSEEK_API_KEY"   # 优先级：环境变量 > 系统钥匙串(keyring: service="openmath", user=profile 名) > api_key 字段(不推荐)
temperature = 0.2
max_tokens = 1024
supports_tools = true
supports_json_mode = true
timeout_ms = 60000
extra_headers = {}          # 例如 OpenRouter 的 HTTP-Referer

[[llm.profiles]]
name = "deepseek-fim"
kind = "openai_fim"
base_url = "https://api.deepseek.com/beta"
model = "deepseek-chat"
api_key_env = "DEEPSEEK_API_KEY"
max_tokens = 64
```
`KernelConfig` 序列化到前端时，`api_key` 字段**永远**被替换为 `"***"`（若已设置），前端提交 `"***"` 表示不修改。
