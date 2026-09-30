<!-- Extracted from docs/plan/PLAN.md sections [13]. PLAN.md is authoritative; keep in sync. -->

## 13. 里程碑与任务

> 每个任务默认包含：写测试 → 跑测试确认失败 → 实现 → 跑测试确认通过 → `cargo fmt && cargo clippy --all-targets -- -D warnings` → 提交。下面只写“测试什么”和“实现什么”，不重复这个套路。任务里引用的类型/签名请以第 6、9、10、11 节为准；算法请以第 8 节为准。**每完成一个任务就在 `docs/plan/PROGRESS.md` 打勾。**

### M0：仓库骨架（1 人日）
**目标：** `cargo build --workspace` 与 `cargo test --workspace` 能跑通一个空壳。

- [ ] **M0.1 初始化仓库结构与工具链**
  Files: 创建 5 节列出的全部目录与占位文件；`rust-toolchain.toml`；`Cargo.toml`（workspace，`[workspace.package] version="0.1.0" edition="2024" license="MIT OR Apache-2.0" rust-version="1.94"`）；`deny.toml`（见 3 节白名单）；`.github/workflows/ci.yml`（跑 `fmt --check`、`clippy -D warnings`、`test --workspace`、`build --target wasm32-unknown-unknown -p om-kernel --no-default-features`、`cargo deny check`）；把本计划文件复制为 `docs/plan/PLAN.md`；新建空的 `docs/plan/PROGRESS.md`（列出全部任务 id）、`QUESTIONS.md`、`DEVIATIONS.md`。
  Steps: 1) `rustup target add wasm32-unknown-unknown`；2) `cargo install wasm-bindgen-cli --version 0.2.129 --locked cargo-deny --locked cargo-insta --locked`；3) 建目录；4) 写 workspace `Cargo.toml`（先只含 `om-num`）；5) `cargo new --lib crates/om-num`，加 `#![forbid(unsafe_code)]`；6) `cargo build --workspace`；7) 提交 `chore: scaffold workspace`。
  Done: `cargo build --workspace` 成功，CI 文件语法正确（本地用 `act` 或直接 push 到分支验证亦可，但不强制）。

- [ ] **M0.2 建立所有 crate 的空壳与依赖关系**
  Files: 为 `om-core om-parse om-format om-poly om-simplify om-solve om-eval om-llm om-kernel om-cli om-wasm` 逐一 `cargo new --lib`（`om-cli` 用 `--bin`），写各自 `Cargo.toml` 的 `[dependencies]`（按第 4 节 DAG 与第 3 节版本表）；workspace `Cargo.toml` 加入 `[workspace.dependencies]` 统一版本号。
  Interfaces: 每个 crate 的 `src/lib.rs` 先只写 `#![forbid(unsafe_code)]` 和一个 `pub fn placeholder() {}` + 一条 `#[test]`。
  Steps: 按依赖顺序（om-num → om-core → om-poly →…）逐个创建并 `cargo check -p <crate>`。
  Done: `cargo test --workspace` 全绿（都是占位测试）；`cargo tree -p om-poly` 显示它不传递依赖 `om-core`（验证 C1）。

- [ ] **M0.3 前端骨架**
  Files: `app/package.json`（脚本：`dev build lint test typecheck`）、`app/vite.config.ts`（含 `vite-plugin-wasm`）、`app/tsconfig.json`、`app/index.html`、`app/eslint.config.js`、`app/src/main.tsx` + `App.tsx`（渲染 "OpenMath"）、`app/src-tauri/`（`cargo tauri init` 或手写 `tauri.conf.json` + `Cargo.toml`，crate 名 `om-desktop`，加入 workspace）。
  Steps: 1) `cd app && npm install`（版本锁定见 3 节）；2) `npm run build` 产出 `dist/`；3) 把 `src-tauri` 加入根 `Cargo.toml` 的 `[workspace.members]`；4) `cargo check -p om-desktop`。
  Done: `npm run build` 与 `cargo check -p om-desktop` 都成功。

### M1：om-num（3–4 人日）
- [ ] **M1.1 Number 类型与算术**（6.1 节接口；测试见 8.1.5 表中数字相关用例的数值部分）
  Files: `crates/om-num/src/number.rs`（Number/Real/Complex + normalize/add/mul/neg/recip/pow_int/precision/cmp_real）、`src/lib.rs` 重导出。
  Steps 覆盖：精确算术（Int/Rat）、传染规则（8.1.0）、`1/0`→NumError::DivByZero、`recip(0.)` 同样返回 `NumError::DivByZero`；无穷由上层 Power 转成符号特殊值，数值层不制造 inf/NaN。
  Done: `cargo test -p om-num number::` 通过，包含至少 20 条覆盖传染规则的用例。

- [ ] **M1.2 数论工具**（ntheory.rs：gcd/ext_gcd/isqrt/exact_root/perfect_power/is_probable_prime/factor_integer/extract_root_factor）
  Steps: 先写 Miller-Rabin 确定性测试（<2^64 用固定底数表），BPSW 用于更大的数；`factor_integer` 先试除到 1e4，再 Pollard-Brent rho（用 `SplitMix64` 作随机源）。
  测试向量：`gcd(48,18)=6`；`isqrt(10^18)`；`exact_root(64,3)=Some(4)`；`perfect_power(12)=None`，`perfect_power(64)=Some((2,6))`；`is_probable_prime` 对前 1000 个已知素数/合数正确；`factor_integer(2^64+1)` 能在预算内分解（已知因子 274177 × 67280421310721）。
  Done: proptest：`factor_integer(n)` 各因子相乘（含重数）等于原 n。

- [ ] **M1.3 SplitMix64、Fp、Ball 球算术骨架**
  Files: `rng.rs`（SplitMix64）、`modp.rs`（Fp：add/sub/mul/inv/pow，u128 中间值）、`ball.rs`（6.1 节最后一段接口：`exact/add/sub/mul/div/contains_zero/excludes_zero/to_f64`；`sqrt`）。
  Steps: 先确认 `dashu-float 0.6.1` 提供哪些函数（跑 `cargo doc -p dashu-float --open` 或查 docs.rs），有 exp/ln/powf 就直接用，误差半径按“≤1 ulp”加；没有的话本任务只交付 sqrt（用牛顿迭代），exp/ln/sin/cos/atan 移到 M1.4。
  Done: `Ball::exact(&Rational::from(2), 64).sqrt()` 严格包围 sqrt(2)；`a.mul(&b).sub(&b.mul(&a)).contains_zero()` 对测试的任意有限球 a,b 成立。

- [ ] **M1.4 Ball 初等函数**（若 M1.3 未完成 exp/ln/sin/cos/atan，在此实现；否则本任务改为“用 dashu-float 包一层误差边界”）
  Steps: pi(bits) 用 Machin 公式并按精度缓存（`OnceLock<Mutex<BTreeMap<u32, BigFloat>>>`）；exp/ln/sin/cos/atan 按 8 节前言约定的参数约简 + Taylor，误差上界显式计入 rad。
  Done: 与 f64 标准库比较误差 < 1e-15；`pi(3400 bits)` 前 1000 位十进制与内置常量字符串比对（常量存 `tests/pi_1000.txt`）。

### M2：om-core（4–5 人日，本项目最关键的正确性基石）
- [ ] **M2.1 Symbol 驻留与内置符号表**（6.2 节 Symbol、6.2 节 define_builtins! 宏）
  Files: `symbol.rs`、`builtins.rs`。
  Done: `Symbol::intern("Plus") == Symbol::intern("Plus")`；`BUILTIN::PLUS.name() == "Plus"`；线程间共享驻留表的测试（spawn 两个线程各驻留 100 个符号，检查无重复/无丢失）。

- [ ] **M2.2 Expr/ExprNode 与访问器**（6.2 节其余部分，不含 canon 构造器——此时 `Expr::normal` 是唯一构造方式）
  Done: `Expr::int(1) == Expr::int(1)`（结构相等）；`Expr::real(1.0) != Expr::int(1)`；`free_symbols`、`replace_all`（先不做规范化，占位直接返回结构替换结果）、`leaf_count` 各有测试。

- [ ] **M2.3 canonical_cmp 全序**（8.1.1 节）
  Done: 反自反、反对称、传递性的 proptest；`canonical_cmp(a,a) == Equal`；数排在非数之前的用例。

- [ ] **M2.4 add()/Plus 规范构造器**（8.1.2 节 + 8.1.5 表中 #1,2,3,6,7,8,28,29,34,35,36,38 等 Plus 相关用例）
  Done: 全部对应测试向量通过；O(n log n) 的性能测试（1000 项的 Plus 在 <10ms 内完成，release 模式）。

- [ ] **M2.5 mul()/Times 规范构造器**（8.1.3 节 + 表中 #4,5,9,10,11,12,13,14,15,16,17,27,30,31,32,33,37,39 等）
  Done: 全部对应测试向量通过。

- [ ] **M2.6 pow()/Power 规范构造器**（8.1.4 节 + 表中 #18–26,40 及 M2.4/M2.5 未覆盖的其余项）
  Done: 8.1.5 全部 40 条测试向量通过；8.1.5 节的 proptest（幂等性 + 数值一致性）通过。

- [ ] **M2.7 canonicalize() 与其余构造器**（neg/sub/div/sqrt/exp/func；func 的 E^Log[z]→z 规则）
  Done: `canonicalize(Expr::normal(...未规范化的树...))` 与手写规范树结构相等的用例（至少 10 条，覆盖嵌套 Plus/Times/Power）。

- [ ] **M2.8 Interrupt/Clock/Message**（6.4 节）
  Done: `Interrupt::tick()` 在 flag 置位后返回 `Err(Abort::Interrupted)`；在 steps_left 耗尽后返回 `Err(Budget)`；wasm target 下编译通过（`cargo build -p om-core --target wasm32-unknown-unknown`，此时不注入 Clock，`deadline_ms` 恒为 None 分支）。

### M3：om-parse + om-format（4 人日）
- [ ] **M3.1 词法分析器**（两种方言共用 token 流；7.2/7.3 的 token 种类）
  Files: `crates/om-parse/src/lexer.rs`。
  Done: 数字（含 `1e-3`、`0x1F`、`1_000_000`、Wolfram 的 `1.5*^-3` 与 `` 1.5`30 ``）、标识符（含 Unicode/希腊字母）、字符串、注释（`#`、`(* … *)` 可嵌套）、所有运算符 token 的用例。

- [ ] **M3.2 现代方言 Pratt 解析器**（7.2 节全部表格 + 优先级 + 歧义裁决 W001/W002/E010）
  Done: 7.2 节每一行语法各至少 1 条测试；隐式乘法歧义的 3 条裁决用例；`detect_dialect`（7.3 最后一段）的正反例。

- [ ] **M3.3 Wolfram 方言解析器**（7.3 节列出的全部构造）
  Done: 覆盖 `f[x]`、`v[[1]]`、`_`/`x_`/`x_Integer`/`x__`/`x___`、`#`/`#1`/`&`、`/.`/`//.`、`f'[x]`、`\[Pi]` 等的用例；第 14 节验收语料里全部 Wolfram 语法输入都能无诊断错误地解析（先只测试“能解析成某个 Expr”，语义在 M7/M8 测）。

- [ ] **M3.4 诊断与 Fix**（7.2 节 W001/W002/E010 + 通用括号不匹配等）
  Done: 每种诊断都有 `span` 精确指向问题位置的测试；`Fix` 应用后重新解析不再报错。

- [ ] **M3.5 om-format：FullForm/InputForm/modern_form**
  Done: 8.1.5 全部 40 条测试向量的 FullForm 输出字符串完全匹配；`input_form`/`modern_form` 对同一组 Expr 输出符合 7.2/7.3 语法的字符串，且**反解析后与原 Expr 结构相等**（round-trip 测试）。

- [ ] **M3.6 om-format：LaTeX 与 Unicode**
  Done: `latex(Expr::rational(1,2)) == "\\frac{1}{2}"`；分数、根号、上标、希腊字母、函数名（`\sin`）等至少 15 条用例；`unicode_form` 的上标数字、`√`、`π` 等用例。

### M4：om-eval 基础层（3 人日）
- [ ] **M4.1 Evaluator 骨架与求值循环**（9.1/9.2 节步骤 1–4，先不含模式匹配与属性线程化）
  Done: `2+2` 求值为 `4`；`x` 求值为 `x`（未定义）；`Set`/`Unset`/`Clear` 生效；递归深度超限的测试。

- [ ] **M4.2 属性与 Hold**（HoldAll/HoldFirst/HoldRest/Listable 线程化）
  Done: `Hold[1+1]` 不求值；`HoldForm` 显示不求值但可展开；`{1,2}+{3,4}` Listable 线程化为 `{4,6}`；长度不一致时发消息且原样返回。

- [ ] **M4.3 模式匹配**（9.2 节 pattern.rs：Blank/Pattern/Condition/BlankSequence/BlankNullSequence）
  Done: `f[x_] := x^2` 定义后 `f[3]` 求值为 `9`；`f[x_Integer] := ...` 类型约束；`BlankSequence` 匹配变长参数；同名 Pattern 一致性检查（`f[x_,x_]:=x` 只匹配两个相等参数）。

- [ ] **M4.4 基础内置函数（M4 清单）**（9.3 节 M4 分类的全部函数 + DocEntry 登记）
  Done: 每个函数至少 2 条测试；`Evaluator::all_docs()` 数量与清单一致；参数个数检查（Arity）的错误消息测试。

- [ ] **M4.5 N[] 与球算术求值**（9.4 节）
  Done: `N[Pi, 50]` 前 50 位正确；`N[1/3]` 机器精度；精度加倍收敛测试；溢出转 BigFloat 的测试。

### M5：om-poly 基础（GCD、平方free、多项式运算，5 人日）
- [ ] **M5.1 UPoly/MPoly 类型与 Ring/Field/EuclideanRing trait**（8.2 节开头）
  Done: `UPoly<IBig>`/`UPoly<RBig>`/`UPoly<FpElem>` 的加减乘、`divrem`（域上）编译通过并有基础用例。

- [ ] **M5.2 除法/伪余式/容量**（8.2a）— 测试向量见该节。
- [ ] **M5.3 GCDHEU + subresultant PRS 兜底**（8.2b）— 测试向量 + proptest 一致性。
- [ ] **M5.4 Yun 无平方分解（Q 与 Fp）**（8.2c）— 测试向量。
- [ ] **M5.5 结式与判别式**（8.2e）— 测试向量。

### M6：om-poly 因式分解与实根隔离（6 人日，高风险，预留缓冲）
- [ ] **M6.1 DDF/EDF（Cantor-Zassenhaus）**（8.2d 相关部分）
- [ ] **M6.2 二次 Hensel 提升**（8.2d hensel_step/hensel_lift）
  Done: `hensel_lift` 结果模 `p^l` 等于原多项式（proptest，随机选 f 与素数 p）。
- [ ] **M6.3 Zassenhaus 重组 + Mignotte 界**（8.2d zassenhaus，含常数项剪枝与 tick 预算）
  Done: 8.2d 全部测试向量 + proptest（乘积分解回原式）。
- [ ] **M6.4 多元 GCD 与多元因式分解（Kronecker 代换）**（8.2b 多元部分 + 8.2d 最后一段）
- [ ] **M6.5 实根隔离（Descartes + 二分 VCA）**（8.2f 前半）
  Done: 8.2f 测试向量。
- [ ] **M6.6 Aberth-Ehrlich 复根 + 球算术认证**（8.2f 后半）
  Done: 认证圆盘不相交时结果与已知根比对（用 6.5 中同批多项式的复根扩展版）。
- [ ] **M6.7 om-poly/alg.rs：RealAlg/ComplexAlg 与 RootReduce**（8.2i）
  Done: 8.2i 测试向量；Root 编号顺序（8.2g）测试向量。

### M7：Gröbner 基与线性代数（4 人日）
- [ ] **M7.1 Buchberger + Gebauer-Möller + sugar**（8.2h 前半）— 测试向量。
- [ ] **M7.2 FGLM**（8.2h 后半）— 测试向量；与直接 lex Buchberger 结果一致性 proptest。
- [ ] **M7.3 零维/维数判定**
- [ ] **M7.4 Bareiss 无分数消元**（8.8.1）— 测试向量。
- [ ] **M7.5 整数线性方程的 Hermite 标准形**（8.8.3 Integers 部分）

### M8：om-simplify（4 人日）
- [ ] **M8.1 生成元归一化与 Expr↔多项式转换**（8.3 节）— 测试向量。
- [ ] **M8.2 together/cancel/expand/factor**（8.5 节相关部分）
- [ ] **M8.3 is_zero 判定器（L0–L3）**（8.4 节）— 测试向量 + proptest（L1 对有理表达式不返回 Unknown）。
- [ ] **M8.4 simplify 最佳优先搜索**（8.5 节其余部分）
- [ ] **M8.5 初等函数特殊值表**（8.1.6 节）— 全部测试向量。

### M9：om-solve（本项目的核心，8 人日，预留充分缓冲）
> 严格按 8.6–8.9 节的顺序实现；每个子任务都要接入 `StepSink`（8.10 节）并至少产出对应的 `rule_id`。
- [ ] **M9.1 Steps 数据模型与 StepRecorder**（8.10 节）
- [ ] **M9.2 输入归一化 P0/P1**（8.6 节）
- [ ] **M9.3 poly_uni：因式分解路径 + 线性/二次公式**（8.7.1、8.7.2 前三条）
- [ ] **M9.4 poly_uni：二项式/回文/p(x^k) 代换**（8.7.2 中间部分）
- [ ] **M9.5 poly_uni：Cardano 三次式与 Ferrari 四次式（含 Root 兜底）**（8.7.2 后半）
- [ ] **M9.6 实根判定（Reals 域）**（8.7.2 最后一段）
- [ ] **M9.7 根式方程**（8.7.3）
- [ ] **M9.8 超越方程：核统一 + 反函数表**（8.7.4）
- [ ] **M9.9 线性方程组**（8.8.1，复用 M7.4）
- [ ] **M9.10 多项式方程组**（8.8.2，复用 M7.1/M7.2）
- [ ] **M9.11 非多项式方程组（代换法）**（8.8.2 最后一段）
- [ ] **M9.12 Integers/Rationals 定义域**（8.8.3，复用 M7.5）
- [ ] **M9.13 验证策略**（8.8.4）— 贯穿全部路径接入
- [ ] **M9.14 Reduce-lite 不等式**（8.9）
- [ ] **M9.15 NSolve/FindRoot/Eliminate**（8.8.2 最后三段）
- [ ] **M9.16 Solve 验收语料第一轮**：跑第 14 节全部 P0 用例，逐条修 bug 直到全绿。

### M10：om-eval 代数层 + Solve 接入（3 人日）
- [ ] **M10.1 代数类内置函数**（9.3 节 M10.1 分类：Expand/Factor/Together/Cancel/Simplify/Collect/D 等）
- [ ] **M10.2 Solve/NSolve/FindRoot/Reduce/Eliminate/SolveValues 内置函数**（9.3 节 M10.2 分类，把 om-solve 接入求值循环；`Unsupported` 错误的降级处理见 6.6 节最后一段）
  Done: 第 14 节全部 P0 + P1 用例通过（通过 `om-cli -e` 端到端跑）。

### M11：om-kernel（6 人日）
- [ ] **M11.1 协议类型与 ts-rs 导出**（10.2 节，先只做类型定义与导出，不做业务逻辑）
- [ ] **M11.2 Session::handle 基础请求**（Evaluate/GetConfig/SetConfig/LoadNotebook/SaveNotebook）
- [ ] **M11.3 响应式笔记本依赖图**（10.3 节）— 该节两条测试场景。
- [ ] **M11.4 输出打包**（10.4 节）
- [ ] **M11.5 绘图采样（一元自适应 + 隐函数 marching squares）**（10.5 节）— 该节测试。
- [ ] **M11.6 Solve 自动可视化**（10.6 节）
- [ ] **M11.7 补全/悬停/预览**（10.7 节）— 性能测试（<5ms）。
- [ ] **M11.8 配置持久化**（10.8 节，TOML 读写 + directories）

### M12：om-llm（4 人日）
- [ ] **M12.1 请求构造 + SSE/NDJSON 解码器**（11.1/11.2 节）— fixture 随机切块测试（11.5 节）。
- [ ] **M12.2 FIM 请求与响应解析**（11.2 节 FIM 部分）
- [ ] **M12.3 Job 状态机**（11.4 节）— 两轮工具调用 fixture 测试。
- [ ] **M12.4 translate/explain/complete/fix/chat 提示词与后处理**（11.3 节，含解析失败重试逻辑）
- [ ] **M12.5 native 驱动（reqwest + wiremock 端到端）**（11.1 节 drive_native）
- [ ] **M12.6 kernel 接入 LLM 请求/事件**（10.2 节 Llm* 系列 Request/Response/Event）

### M13：前端与打包（8 人日）
- [ ] **M13.1 KernelClient 三种实现**（12.10 节 wasmClient/tauriClient + om-wasm bindgen 导出、12.11 节 Tauri commands）
- [ ] **M13.2 笔记本核心 UI**（TopBar/Notebook/Cell/编辑器高亮+诊断+补全，12.2/12.3 节）
- [ ] **M13.3 输出渲染**（ExprView/SolutionCards/NumberLine/Messages，12.4 节）
- [ ] **M13.4 步骤面板**（12.5 节）
- [ ] **M13.5 绘图组件**（12.6 节，含滑块交互）
- [ ] **M13.6 幽灵文本补全**（12.7 节）— mock kernel 测试。
- [ ] **M13.7 AI 界面（Ask cell/助手面板/错误修复/隐私提示）**（12.8 节）
- [ ] **M13.8 设置对话框**（12.9 节）
- [ ] **M13.9 CLI 完整实现**（12.12 节）
- [ ] **M13.10 桌面打包 + Web 构建**（`npx tauri build`；`npm run build` 产出可部署的静态站点）
- [ ] **M13.11 端到端验证**（见第 15 节）与文档收尾（`docs/language.md`、`docs/solve.md`、`docs/llm.md`、双语 README）

**总预估：约 62 人日**（单人全职约 3 个月；用 subagent-driven-development 并行执行可显著缩短，但 M9/M6 在关键路径上，建议优先保证这两个里程碑的正确性而非速度）。
