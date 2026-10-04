# OpenMath（om）实施计划 —— 以 Solve 为核心的开源 Rust 计算机代数系统

> **给执行代理（agentic workers）：** 必须使用 superpowers:subagent-driven-development（推荐）或 superpowers:executing-plans 逐任务执行本计划。步骤使用复选框（`- [ ]`）跟踪。**先完整读完第 0 节，再开始任何任务。**

**Goal（目标）：** 用 Rust 写一个开源的、兼容 Mathematica `Solve` 语义的符号方程求解系统，提供比 Mathematica 更现代的笔记本界面（桌面 Tauri + 浏览器 WASM + 终端 REPL），内置可自定义接入的 LLM 接口，用于自然语言转公式、解题步骤讲解和智能补全。

**Architecture（架构）：** 分层的 Cargo workspace：`om-num`（数）→ `om-core`（表达式与规范形式）→ `om-poly`（多项式算法）→ `om-simplify` → `om-solve`（Solve/NSolve/FindRoot/Reduce/Eliminate 与步骤记录）→ `om-eval`（求值器与内置函数）→ `om-kernel`（会话、响应式笔记本、JSON 协议、LLM 编排）→ 三个前端（`om-cli`、`om-wasm`+Web、`om-desktop`=Tauri）。LLM 层 `om-llm` 采用 sans-IO 设计（请求构造与流解码是纯函数，传输层可替换）。**原则：LLM 负责提议，CAS 负责验证；LLM 输出只能作为文本经由 CAS 解析器进入系统。**

**Tech Stack（技术栈）：** Rust 1.94（edition 2024）、纯 Rust 依赖（可编译到 `wasm32-unknown-unknown`）、Tauri 2、React 19 + TypeScript + Vite、CodeMirror 6、KaTeX、reedline。具体版本见第 3 节。

**Spec（规格）：** 本文件即规格与计划合一。第 6–12 节是规格，第 13 节是逐任务计划，第 14 节是验收语料。仓库 `docs/plan/PLAN.md` 是实施时的权威版本。

**2026-10-05 用户授权扩展：** M0–M13及已公开 `.1/.2` 的历史规格保留。下一版 `.3` 使用 [NEXT_RELEASE.md](NEXT_RELEASE.md)、[现代语言设计](../design/modern-language.md) 和 [全景目录](../reference/README.md) 的新增裁决；范围外条目由新计划明确逐项覆盖，不能因历史 §17 而误判移动端尚未实现。本轮仅R3.0文档/审计，R3.1以后的实现按新账本接续，规划不冒充当前能力。

---

## 目录

- 0. 执行守则（低级模型必读）
- 1. 背景与调研结论
- 2. 已确认的产品决策
- 3. 全局约束（Global Constraints）
- 4. 架构总览
- 5. 仓库结构
- 6. 核心数据类型与接口契约
- 7. 输入语言规格（现代方言 + Wolfram 方言）
- 8. 算法规格（规范形式、多项式、Solve 管线、零判定）
- 9. 求值器规格
- 10. Kernel、响应式笔记本与 JSON 协议
- 11. LLM 层规格
- 12. UI/UX 规格
- 13. 里程碑与任务（M0–M13）
- 14. Solve 验收语料
- 15. 端到端验证
- 16. 风险与降级策略
- 17. 范围之外（未来工作）

---

## 0. 执行守则（低级模型必读）

1. **一次只做一个任务。** 每个任务都有：Files / Interfaces / Steps / Done 标准。不要提前实现后续任务的功能（YAGNI）。
2. **测试先行（TDD）。** 每个任务先写失败测试 → 运行确认失败 → 最小实现 → 运行通过 → 提交。测试向量已在计划中给出，**不许为了让测试通过而修改期望值**；如果你认为期望值错了，停下来在 `docs/plan/QUESTIONS.md` 记录理由，并跳到下一个不依赖它的任务。
3. **接口是契约。** 第 6、9、10、11 节给出的 Rust 类型名、函数签名、JSON 字段名必须逐字使用。需要新增公共接口时，先在 `docs/plan/DEVIATIONS.md` 记录。
4. **每个任务完成前必须运行：**
   ```bash
   cargo fmt --all
   cargo clippy --workspace --all-targets -- -D warnings
   cargo test --workspace
   ```
   前端任务另外运行 `cd app && npm run lint && npm run test && npm run build`。
5. **进度账本。** 每完成一个任务，在同一次提交的 `docs/plan/PROGRESS.md` 里把对应行改成 `[x]` 并记录验证命令。该提交哈希在下一次任务提交中补记（避免提交引用自身哈希的循环）。新会话开始时先读 `PROGRESS.md` 找到下一个任务。
6. **禁止事项：** 库代码中不用 `unwrap()`/`expect()`（测试除外；确属不变量时用 `expect("invariant: …")` 并写清原因）；不用 `unsafe`（所有 crate 顶部 `#![forbid(unsafe_code)]`）；输出中不得依赖 `HashMap` 迭代顺序（用 `BTreeMap`/`IndexMap` 或排序）；不引入第 3 节白名单以外的依赖（需要时记录到 `DEVIATIONS.md`）。
7. **文件大小：** 单个源文件超过约 600 行就拆分模块。
8. **确定性：** 所有随机性（数值探测、素数选择）使用固定种子的 `SplitMix64`（`om-num/src/rng.rs`），测试结果必须可复现。
9. **卡住时：** 同一个测试连续 3 次修复失败，使用 superpowers:systematic-debugging；仍失败就记录到 `QUESTIONS.md` 并继续其他任务。
10. **Git：** 在 `dev` 分支上工作，每个任务一次提交，提交信息格式 `<type>(<crate>): <summary>`，例如 `feat(om-poly): add Yun square-free factorization`。每个里程碑验收通过后把 `dev` 快进合并到 `main`。

11. **联网与代理：** 本机有 HTTP 代理 `http://127.0.0.1:7890`（环境变量已设置），`cargo`/`npm` 下载失败时先确认代理可用；测试代码**绝不**访问真实网络（LLM 测试一律用录制的 fixture 或本地 mock server）。

---

## 1. 背景与调研结论

### 1.1 需求
用户主要使用 Mathematica 的 `Solve`，希望有一个：(1) 开源；(2) Rust 实现；(3) 交互界面与逻辑比 Mathematica 更现代；(4) 可自定义接入 LLM（自然语言辅助、智能补全）的系统。

### 1.2 现有开源项目（2026-09 核实）

| 项目 | 语言 | 许可证 | 状态 | 与本项目关系 |
|---|---|---|---|---|
| **Woxi** (ad-si/Woxi) | Rust | **AGPL-3.0** | 活跃（923★，2026-09-28 有提交） | Wolfram 语言解释器。有 `woxi-reduce`、`to_radicals.rs`、`integer_solve.rs`。**只可阅读学习思路，不可复制代码**（AGPL 与我们的 MIT/Apache 不兼容）。 |
| **Mathics3** (mathics-core) | Python | GPL-3.0 | 活跃 | 开源 Mathematica 内核，`Solve` 主要委托给 SymPy。可参考其**输出格式与内置函数语义**，不复制代码。 |
| **SymPy** | Python | BSD-3-Clause | 非常活跃 | 求解管线（`solve`/`solveset`/`roots`/`polysys`/`unrad`）是最重要的**算法参考**。BSD 与 MIT 兼容：允许移植算法，移植大段逻辑时在文件头注明“algorithm adapted from SymPy (BSD-3-Clause)”并在 `THIRD_PARTY_NOTICES.md` 附 SymPy 许可证。 |
| **Symbolica** 3.0 | Rust | 非 OSI（source-available，商用需授权） | 活跃 | 高性能，但**不能作为依赖**，也不要复制代码。 |
| **feanor-math** 3.6 | Rust | MIT | 活跃 | 数论/多项式库。**只作算法参考**（为保证 wasm、可控性以及生成步骤记录，om-poly 自研）。 |
| Expreduce | Go | MIT | 基本停更（2024-12） | Mathematica 风格的规则重写引擎，可以参考。 |
| Maxima / Giac / REDUCE | Lisp/C++ | GPL 系 | 成熟 | 参考 `algsys`、`to_poly_solve` 的思路。 |
| savage | Rust | — | 停更（2023） | 规模太小，忽略。 |

**结论：** 没有一个满足“宽松许可证 + Rust + 高质量 Solve + 现代 UI + LLM”的现成项目。我们自研内核：大整数/有理数/任意精度浮点用 `dashu`；多项式代数、求解器、求值器、解析器全部自研，算法参照 SymPy（BSD）和经典教材。

### 1.3 算法参考书（执行者遇到算法细节问题时查阅）
- Geddes, Czapor, Labahn, *Algorithms for Computer Algebra*（GCD、Hensel 提升、Zassenhaus、结式）
- von zur Gathen & Gerhard, *Modern Computer Algebra*（Cantor–Zassenhaus、快速算法、实根隔离）
- Cox, Little, O'Shea, *Ideals, Varieties, and Algorithms*（Buchberger、消元理论、FGLM）
- SymPy 源码：`sympy/polys/polyroots.py`、`sympy/polys/factortools.py`、`sympy/polys/euclidtools.py`、`sympy/polys/groebnertools.py`、`sympy/solvers/solvers.py`（`unrad`）、`sympy/solvers/solveset.py`、`sympy/solvers/polysys.py`、`sympy/solvers/inequalities.py`
- Wolfram 文档：`reference.wolfram.com/language/ref/Solve.html`、`/NSolve.html`、`/Reduce.html`、`/Root.html`、`/ConditionalExpression.html`、`tutorial/EquationsAndInequalities`（**以文档中的输入输出示例为准**）

---

## 2. 已确认的产品决策（用户已拍板，不得更改）

| 决策 | 取值 |
|---|---|
| 界面形态 | **Tauri 2 桌面应用 + 浏览器 Web 应用（引擎编译为 WASM）+ 终端 REPL（CLI）**，三者共享同一个 kernel 协议 |
| 输入语法 | **现代语法（默认）+ Wolfram 语法兼容**，两种方言解析到同一棵内部表达式树 |
| 差异化特性 | **解题步骤/推导过程**（结构化步骤记录是求解器的一等输出）、**解的交互式可视化**（函数图像、交点、不等式区间、参数滑块）、**现代交互理念**（响应式笔记本、实时公式预览、命令面板、解卡片、自然语言输入、AI 讲解、幽灵文本补全） |
| 不在 v1 范围 | MCP 服务器、Jupyter 内核（列入第 17 节未来工作；架构上预留，JSON 协议保证日后容易加上） |
| 许可证 | **MIT OR Apache-2.0**（双许可证），只允许依赖宽松许可证（MIT/Apache/BSD/ISC/Zlib/Unicode） |
| 仓库名 / crate 前缀 | 仓库 `openmath`，所有 crate 以 `om-` 开头，可执行文件名 `om`，展示名 “OpenMath” |

---

## 3. 全局约束（Global Constraints）

每个任务都隐式包含本节全部要求。

- **Rust：** stable 1.94，`edition = "2024"`，`rust-version = "1.94"`。workspace 共享 `[workspace.package]` 与 `[workspace.dependencies]`。
- **WASM：** `om-num`、`om-core`、`om-parse`、`om-format`、`om-poly`、`om-simplify`、`om-solve`、`om-eval`、`om-kernel`（关闭 `native` feature 时）、`om-llm`（关闭 `http` feature 时）必须能编译到 `wasm32-unknown-unknown`。所以：这些 crate 不得依赖 `std::time::Instant`（在 wasm32 上会 panic）、线程、文件系统、`getrandom`、`tokio`。时钟与取消通过 `om_num::ctx::Interrupt` 注入（`om_core::ctx` 重导出相同类型，见 6.4），使 om-poly 不依赖表达式层。CI 里有专门任务：`cargo build -p om-kernel --no-default-features --target wasm32-unknown-unknown`。
- **许可证：** 每个 crate 的 `license = "MIT OR Apache-2.0"`；根目录放 `LICENSE-MIT`、`LICENSE-APACHE`；`deny.toml`（cargo-deny）只允许 `MIT, Apache-2.0, Apache-2.0 WITH LLVM-exception, BSD-2-Clause, BSD-3-Clause, ISC, Zlib, Unicode-3.0, CC0-1.0, MPL-2.0`（MPL 不作通用许可，只通过 deny.toml 中逐包例外允许必要的传递依赖）。**禁止：malachite（LGPL）、rug/gmp-mpfr-sys（LGPL）、algebraics（LGPL）、symbolica（非 OSI）、flint 绑定。**
- **依赖白名单**（版本为 2026-09-28 在 crates.io 查到的最新稳定版，Cargo.toml 中写 `"0.6"` 这种兼容版本即可）：

  | 用途 | crate | 版本 | 许可证 |
  |---|---|---|---|
  | 大整数/有理数/任意精度浮点 | `dashu`（`dashu-int`, `dashu-ratio`, `dashu-float`） | 0.6.1 | MIT OR Apache-2.0 |
  | 数值 trait | `num-traits` | 0.2.19 | MIT OR Apache-2.0 |
  | 小向量 | `smallvec` | 1.16 | MIT OR Apache-2.0 |
  | 快速哈希 | `rustc-hash` | 2.1 | MIT OR Apache-2.0 |
  | 有序 map | `indexmap` | 2.14 | MIT OR Apache-2.0 |
  | 错误类型 | `thiserror` | 2.0 | MIT OR Apache-2.0 |
  | 序列化 | `serde`（derive）、`serde_json` | 1.0.229 / 1.0.151 | MIT OR Apache-2.0 |
  | TS 类型生成 | `ts-rs` | 12.0 | MIT |
  | 配置 | `toml` 1.1、`directories` 6.0 | | MIT OR Apache-2.0 |
  | 密钥存储（仅 native） | `keyring` | 4.2 | MIT OR Apache-2.0 |
  | HTTP（仅 native） | `reqwest`（features: `json`, `stream`, `rustls`），版本 0.13 | 0.13.5 | MIT OR Apache-2.0 |
  | 异步（仅 native） | `tokio` 1.53、`futures` 0.3、`tokio-util` 0.7 | | MIT |
  | 终端 REPL | `reedline` =0.49.0（0.50+ 要求 Rust 1.95）、`nu-ansi-term` 0.50 | | MIT |
  | 诊断美化（CLI） | `ariadne` | 0.6 | MIT |
  | 日志 | `tracing` 0.1 | | MIT |
  | WASM 绑定 | `wasm-bindgen` 0.2.129、`js-sys` 0.3.106、`console_error_panic_hook` 0.1.7 | | MIT OR Apache-2.0 |
  | 桌面 | `tauri` 2.12、`tauri-build` 2.7、`tauri-plugin-dialog` 2.8、`tauri-plugin-fs` 2.6、`tauri-plugin-opener` 2.6 | | Apache-2.0 OR MIT |
  | 测试 | `proptest` 1.11、`insta` 1.48（features: `json`, `yaml`）、`pretty_assertions` 1.4、`criterion` 0.8、`wiremock` 0.6、`wasm-bindgen-test` 0.3 | | 宽松 |

  **有意不用：** 解析器生成器（chumsky/pest/lalrpop/logos），因为手写 Pratt 解析器在错误恢复、源码位置和两种方言切换上更可控；`egg`/`egglog`（v1 化简用启发式，e-graph 列入未来工作）；`async-openai`/`genai`/`rig-core`（LLM 层需要 sans-IO，同时支持 wasm 和 FIM 端点，所以自己写，大约 800 行）；`nalgebra`/`faer`（v1 数值线代规模很小，自研 LU/QR 即可）。
- **前端（npm，禁止 pnpm/yarn/bun）：** `react`/`react-dom` 19.3、`vite` 8.3、`typescript` 6.0（typescript-eslint 8.70 仅支持 <6.1；见 DEVIATIONS P13）、`@vitejs/plugin-react` 6.1、`vite-plugin-wasm` 3.6、`@codemirror/{state 6.7, view 6.43, commands 6.11, autocomplete 6.20, lint 6.9, language 6.12, search 6.7}`、`@lezer/highlight` 1.2、`katex` 0.18、`zustand` 5.0、`cmdk` 1.1（命令面板）、`@tauri-apps/api` 2.12、`@tauri-apps/plugin-dialog` 2.8、`@tauri-apps/plugin-fs` 2.6、`@tauri-apps/cli` 2.12（devDependency）、测试 `vitest` 5.0、`@testing-library/react` 16.3、`jsdom` 30、`@playwright/test` 1.63、lint `eslint` 10 + `typescript-eslint` 8.70。绘图**不用第三方库**，自写 SVG 组件（见 12.6），以便精确控制解点高亮、区间着色和主题。
  > 如果执行时发现某个版本号与上表不一致（例如大版本升级导致 API 变化），**锁定到上表中的版本**，不要追新。
- **工具安装（一次性，M0 完成）：** `rustup target add wasm32-unknown-unknown`；`cargo install wasm-bindgen-cli --version 0.2.129 --locked`（版本**必须**与 `wasm-bindgen` crate 完全一致）；`cargo install cargo-deny --locked`；`cargo install cargo-insta --locked`。
- **代码风格：** 所有公共项写 `///` 文档注释；模块内部注释只解释“为什么”。错误类型用 `thiserror`；库 crate 不用 `anyhow`（`om-cli`、`om-desktop` 可以用）。
- **国际化：** UI 与 CLI 文案提供 `zh-CN`（默认跟随系统）与 `en`；步骤记录（Steps）里的文案**不写死自然语言**，只存结构化数据，渲染时按语言模板生成（见 8.6）。
- **性能底线（验收时测）：** 第 14 节语料里除标注 `slow` 的用例外，单条 `Solve` 在 M 系列 Mac 上的 release 构建里 < 200 ms；WASM 版本 < 1 s。
- **安全：** LLM 输出**只能**作为源码字符串交给 `om-parse`，从不以任何形式执行（无 `eval`、无 shell）；API 密钥不写入笔记本文件、不写日志、不发送到配置之外的主机。

---

## 4. 架构总览

```
                 ┌────────────── 前端 ──────────────┐
  om-cli (reedline REPL)   app/ (React+CodeMirror+KaTeX)   app/src-tauri = om-desktop
        │                    │  KernelClient 接口                │
        │                    ├── WasmClient (Web Worker + om-wasm)│
        │                    └── TauriClient (invoke + Channel) ──┘
        ▼                                     ▼
  ┌─────────────────────────── om-kernel ────────────────────────────┐
  │ Session · Notebook(响应式依赖图) · Protocol(JSON Request/Event)   │
  │ Completion/Hover/Preview · PlotSampler · LLM 编排(Assistant)       │
  └───────┬──────────────────────────────────────┬───────────────────┘
          ▼                                      ▼
      om-eval (求值器、内置函数表、模式匹配)      om-llm (sans-IO: 请求构造 + SSE/NDJSON 解码
          │                                           + 提示词模板；feature "http" 提供 reqwest 传输)
          ▼
      om-solve (Solve/NSolve/FindRoot/Reduce/Eliminate、分派管线、验证、steps 步骤模型)
          ▼
      om-simplify (convert: Expr↔多项式与生成元归一化；numeval: Expr 的球算术/N 求值(含 Root)；
                   special: 初等函数特殊值表；is_zero 零判定；expand/together/cancel/factor/simplify)
          ▼                                    ▼
      om-core (Expr、Symbol 驻留、规范构造器、      om-poly (**只依赖 om-num**：UPoly/MPoly 泛型环、GCD、
      排序、消息、重导出 Interrupt)                       无平方分解、Z[x] 因式分解、结式、实根隔离、Aberth、
          ▲         ▲                              Gröbner/FGLM、alg: 实/复代数数与 RootReduce)
   om-parse   om-format                                   ▼
          ▼                                             om-num (Interrupt/Clock/Abort、Integer/Rational/Real/Complex、Ball 球算术、
      om-num ◄──────────────────────────────────────── 初等函数任意精度实现、数论、Fp、SplitMix64)
```

**依赖方向严格自上而下**，禁止反向依赖：
- `om-num` ← `om-core` ← {`om-parse`, `om-format`}；
- `om-num` ← `om-poly`（**不依赖 om-core**，这样多项式内层循环里不会有 Expr 的 Arc/哈希开销，也能独立测试）；
- {`om-core`, `om-poly`} ← `om-simplify` ← `om-solve` ← `om-eval` ← `om-kernel` ← {`om-cli`, `om-wasm`, `om-desktop`}；`om-llm` 只依赖 `om-num`（为了 SplitMix64）和 serde，由 `om-kernel` 使用；`om-kernel` 还依赖 `om-parse`/`om-format`。
- `Root[...]` 在 `om-core` 里只是语法（`Root[Function[poly(#1)], k]`）；语义（最小多项式 + 隔离区间/圆盘）在 `om-poly::alg`，由 `om-simplify::numeval` 通过旁路缓存（`HashMap<u64 /*Expr hash*/, AlgNum>`）关联。规范构造器**永远不会**对 Root 求数值。
- 初等函数的特殊值（`Sin[Pi/6] = 1/2`、`ArcSin[1/2] = Pi/6`）放在 `om-simplify::special`，因为 `om-solve` 需要用它化简结果；`om-eval` 调用同一张表。**规范构造器里不做特殊值**，唯一例外是纯结构规则 `E^Log[z] → z`。

**数据流（一次 Solve）：** 源码 →（om-parse）→ `Expr` →（om-eval 求值参数）→ `Solve` 内置函数 → `om_solve::solve(eqs, vars, opts, ctx)` → `SolveOutcome { solutions, steps, messages }` → 转回 `Expr`（`{{x->1},{x->2}}`）并把 `steps` 挂到求值上下文 → kernel 把结果打包成 `CellOutput`（含 InputForm/LaTeX/解卡片/步骤/绘图建议）→ 前端渲染。

---

## 5. 仓库结构

```
openmath/
├── Cargo.toml                 # workspace
├── rust-toolchain.toml        # channel = "1.94", targets = ["wasm32-unknown-unknown"]
├── deny.toml  rustfmt.toml  clippy.toml  .editorconfig  .gitignore
├── LICENSE-MIT  LICENSE-APACHE  THIRD_PARTY_NOTICES.md  README.md  README.zh-CN.md
├── docs/
│   ├── plan/PLAN.md PROGRESS.md QUESTIONS.md DEVIATIONS.md
│   ├── language.md            # 两种方言的用户文档
│   ├── solve.md               # Solve 行为与 Mathematica 差异表
│   └── llm.md                 # LLM 配置指南
├── crates/
│   ├── om-num/  om-core/  om-parse/  om-format/  om-poly/  om-simplify/
│   ├── om-solve/  om-eval/  om-kernel/  om-llm/  om-cli/  om-wasm/
├── app/                       # 前端 (npm)
│   ├── package.json  vite.config.ts  tsconfig.json  index.html  eslint.config.js
│   ├── src/ (见 12.9)
│   └── src-tauri/             # om-desktop crate（workspace 成员）
├── tests/corpus/              # 第 14 节验收语料 (solve.toml, eval.toml, parse.toml)
├── benches/                   # criterion
└── .github/workflows/ci.yml
```

每个 crate 的内部布局：`src/lib.rs` 只写 `mod` 声明与 `pub use`；测试放 `src/**` 内的 `#[cfg(test)] mod tests`（单元测试）和 `tests/*.rs`（集成测试）；快照放 `tests/snapshots/`（insta）。

---

## 6. 核心数据类型与接口契约

> 本节代码是**契约**：名字、字段、签名必须一致。函数体由各任务实现。

### 6.1 om-num

```rust
// crates/om-num/src/lib.rs
pub use dashu::integer::IBig;           // 重导出，其他 crate 不直接依赖 dashu
pub use dashu::rational::RBig;

pub type Integer = IBig;
pub type Rational = RBig;               // 不变量：已约分，分母 > 0（dashu 保证）

/// 实数：机器精度或任意精度（二进制，精度以 bit 计）
#[derive(Clone, Debug)]
pub enum Real {
    Machine(f64),
    Big(BigFloat),                      // BigFloat = dashu::float::FBig<HalfEven, 2>
}

/// 精确或近似的数；Complex 的实部虚部类型相同类别（都精确或都近似）
#[derive(Clone, Debug)]
pub enum Number {
    Integer(Integer),
    Rational(Rational),                 // 不变量：分母 != 1（否则归一为 Integer）
    Real(Real),
    Complex(Box<Complex>),              // 不变量：im != 0（否则归一为实部）
}
#[derive(Clone, Debug)]
pub struct Complex { pub re: Number, pub im: Number }   // re/im 不能是 Complex

impl Number {
    pub fn normalize(self) -> Number;               // 执行上述不变量
    pub fn is_exact(&self) -> bool;
    pub fn is_zero(&self) -> bool; pub fn is_one(&self) -> bool; pub fn is_negative(&self) -> bool; // 仅对实数有意义，Complex 返回 false
    pub fn add(&self, o: &Number) -> Number; pub fn mul(&self, o: &Number) -> Number;
    pub fn neg(&self) -> Number; pub fn recip(&self) -> Result<Number, NumError>;  // 0 的倒数 -> Err(DivByZero)
    pub fn pow_int(&self, e: &Integer) -> Result<Number, NumError>;
    pub fn to_f64(&self) -> Option<f64>;            // Complex -> None
    pub fn to_complex_f64(&self) -> (f64, f64);
    pub fn precision(&self) -> Precision;           // Exact | Machine | Bits(u32)
    pub fn cmp_real(&self, o: &Number) -> Option<std::cmp::Ordering>; // 任一为 Complex -> None
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Precision { Exact, Machine, Bits(u32) }

// 数论（crates/om-num/src/ntheory.rs）
pub fn gcd(a: &Integer, b: &Integer) -> Integer;
pub fn ext_gcd(a: &Integer, b: &Integer) -> (Integer, Integer, Integer); // (g, s, t)，s*a+t*b=g
pub fn isqrt(n: &Integer) -> Integer;                        // floor(sqrt(n)), n>=0
pub fn exact_root(n: &Integer, k: u32) -> Option<Integer>;   // n 是完全 k 次幂时返回根（负 n 且 k 奇数返回负根）
pub fn perfect_power(n: &Integer) -> Option<(Integer, u32)>; // 最大的 k
pub fn is_probable_prime(n: &Integer) -> bool;               // <2^64 用确定性 Miller-Rabin 底 {2,3,5,7,11,13,17,19,23,29,31,37}；更大用 BPSW
pub fn factor_integer(n: &Integer, ctx_budget: &mut u64) -> Vec<(Integer, u32)>; // 试除到 10^4 + Pollard-Brent rho；预算耗尽返回部分分解，最后一项可能是合数（标 is_probable_prime=false）
pub fn extract_root_factor(n: &Integer, k: u32) -> (Integer, Integer); // n = a^k * b，b 不含 k 次幂因子，返回 (a,b)，用于 Sqrt[12] -> 2 Sqrt[3]

// crates/om-num/src/modp.rs —— u64 素数域
pub struct Fp { pub p: u64 }  // 方法：add, sub, mul, inv, pow（mul 用 u128 中间值）

// crates/om-num/src/rng.rs
pub struct SplitMix64(u64);   // new(seed), next_u64(), next_range(lo, hi)

// crates/om-num/src/ball.rs —— 严格的球算术（is_zero 与验证要用“可证明”的包围）
#[derive(Clone, Debug)]
pub struct Ball { pub mid: BigFloat, pub rad: BigFloat /*低精度，向上舍入*/, pub prec: u32 }
#[derive(Clone, Debug)]
pub struct CBall { pub re: Ball, pub im: Ball }
impl Ball {
    pub fn exact(q: &Rational, prec: u32) -> Ball;
    pub fn add/sub/mul/div(&self, o: &Ball) -> Ball;   // div 在分母包含 0 时返回 rad = +inf 的“全实数球”
    pub fn sqrt/exp/ln/sin/cos/atan(&self) -> Ball;     // 参数约简 + Taylor，截断误差显式加入 rad
    pub fn pi(prec: u32) -> Ball;                       // Machin 公式，按精度缓存
    pub fn contains_zero(&self) -> bool;
    pub fn excludes_zero(&self) -> bool;                // |mid| > rad —— 可证明非零
    pub fn to_f64(&self) -> f64;
}
// CBall 同样提供 add/sub/mul/div/pow_int/exp/ln/sqrt(主值)/sin/cos，由实数 Ball 组合
// ctx 模块（Clock/Interrupt/Abort）亦属于 om-num，具体签名见 6.4。
```
> **执行前核对：** `dashu-float 0.6.1` 已经提供哪些初等函数（exp/ln/sqrt/powf）？已提供的直接用于 `mid` 的计算，但误差半径 `rad` 仍须自己按“结果误差 ≤ 1 ulp(prec)”加上。sin/cos/atan 必须自己实现。

### 6.2 om-core：表达式

```rust
// crates/om-core/src/expr.rs
#[derive(Clone)]
pub struct Expr(std::sync::Arc<ExprNode>);
pub struct ExprNode { pub kind: ExprKind, pub hash: u64 }   // hash 构造时计算（FxHasher），结构相等则 hash 相等

pub enum ExprKind {
    Number(Number),
    Symbol(Symbol),
    String(Box<str>),
    Normal(Normal),
}
pub struct Normal { pub head: Expr, pub args: smallvec::SmallVec<[Expr; 3]> }

impl PartialEq for Expr  // 先比 Arc 指针，再比 hash，最后结构比较；数值比较是“结构相等”：1 与 1.0 不相等，1.0 与 1.0 相等
impl Eq, Hash (用 hash 字段), Debug (输出 FullForm)

impl Expr {
    pub fn int(i: i64) -> Expr; pub fn integer(i: Integer) -> Expr;
    pub fn rational(n: i64, d: i64) -> Expr;      // 会约分，d==0 panic（仅测试辅助）
    pub fn number(n: Number) -> Expr;             // 先 normalize
    pub fn real(f: f64) -> Expr;
    pub fn sym(s: Symbol) -> Expr;
    pub fn symbol(name: &str) -> Expr;            // 驻留
    pub fn string(s: &str) -> Expr;
    pub fn normal(head: Expr, args: impl IntoIterator<Item=Expr>) -> Expr; // **不做**任何化简（原始构造）
    pub fn call(head: Symbol, args: impl IntoIterator<Item=Expr>) -> Expr; // 同上，head 是符号
    // 访问器
    pub fn kind(&self) -> &ExprKind;
    pub fn as_number(&self) -> Option<&Number>; pub fn as_symbol(&self) -> Option<Symbol>;
    pub fn head(&self) -> Expr;                   // Number->Integer/Rational/Real/Complex 符号；Symbol->Symbol；String->String
    pub fn head_symbol(&self) -> Option<Symbol>;  // Normal 且 head 是符号时
    pub fn args(&self) -> &[Expr];                // 非 Normal 返回 &[]
    pub fn is_head(&self, s: Symbol) -> bool;
    pub fn is_zero(&self) -> bool; pub fn is_one(&self) -> bool; // 仅字面数
    pub fn free_of(&self, x: &Expr) -> bool;      // 子树中不含 x
    pub fn free_symbols(&self) -> std::collections::BTreeSet<Symbol>; // 排除内置常量（Pi, E, I, Infinity…）
    pub fn replace_all(&self, rules: &[(Expr, Expr)]) -> Expr; // 结构替换（非模式），结果**经过规范化**
    pub fn map_args(&self, f: impl FnMut(&Expr) -> Expr) -> Expr; // 重建时用规范构造器
    pub fn leaf_count(&self) -> usize;            // 复杂度度量
}
```

**Symbol 驻留（`crates/om-core/src/symbol.rs`）：** `#[derive(Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)] pub struct Symbol(u32);`（`Ord` 按 id，**只**用于 BTreeMap 等容器；**规范排序与输出排序一律按名字比较**，因为驻留顺序在不同会话、线程中可能不同。哈希用固定种子的 `rustc_hash::FxHasher`，禁止 `RandomState`。） 全局驻留表 `static INTERNER: std::sync::OnceLock<std::sync::RwLock<Interner>>`（wasm 下 RwLock 可用，单线程无竞争）。`Symbol::intern(&str)`、`symbol.name() -> &'static str`（驻留字符串用 `Box::leak`，可以接受）。**内置符号**在 `crates/om-core/src/builtins.rs` 用宏一次性定义为常量，保证 id 固定：

```rust
define_builtins! {
    LIST = "List", PLUS = "Plus", TIMES = "Times", POWER = "Power", RULE = "Rule", RULE_DELAYED = "RuleDelayed",
    EQUAL = "Equal", UNEQUAL = "Unequal", LESS = "Less", LESS_EQUAL = "LessEqual", GREATER = "Greater",
    GREATER_EQUAL = "GreaterEqual", INEQUALITY = "Inequality", AND = "And", OR = "Or", NOT = "Not",
    TRUE = "True", FALSE = "False", NULL = "Null", PI = "Pi", E = "E", I = "I", INFINITY = "Infinity",
    COMPLEX_INFINITY = "ComplexInfinity", DIRECTED_INFINITY = "DirectedInfinity", INDETERMINATE = "Indeterminate",
    SIN = "Sin", COS = "Cos", TAN = "Tan", COT = "Cot", SEC = "Sec", CSC = "Csc",
    ARCSIN = "ArcSin", ARCCOS = "ArcCos", ARCTAN = "ArcTan", ARCCOT = "ArcCot", ARCSEC = "ArcSec", ARCCSC = "ArcCsc",
    SINH = "Sinh", COSH = "Cosh", TANH = "Tanh", COTH = "Coth", SECH = "Sech", CSCH = "Csch",
    ARCSINH = "ArcSinh", ARCCOSH = "ArcCosh", ARCTANH = "ArcTanh",
    EXP = "Exp", LOG = "Log", SQRT = "Sqrt", ABS = "Abs", SIGN = "Sign", RE = "Re", IM = "Im", ARG = "Arg",
    CONJUGATE = "Conjugate", FLOOR = "Floor", CEILING = "Ceiling", ROUND = "Round", PRODUCT_LOG = "ProductLog",
    FACTORIAL = "Factorial", BINOMIAL = "Binomial", MOD = "Mod", GCD = "GCD", LCM = "LCM",
    ROOT = "Root", FUNCTION = "Function", SLOT = "Slot", C = "C", CONDITIONAL_EXPRESSION = "ConditionalExpression",
    ELEMENT = "Element", NOT_ELEMENT = "NotElement", REALS = "Reals", INTEGERS = "Integers", COMPLEXES = "Complexes",
    RATIONALS = "Rationals", ALGEBRAICS = "Algebraics", PRIMES = "Primes", BOOLEANS = "Booleans",
    INTEGER = "Integer", RATIONAL = "Rational", REAL = "Real", COMPLEX = "Complex", SYMBOL = "Symbol", STRING = "String",
    SOLVE = "Solve", NSOLVE = "NSolve", FIND_ROOT = "FindRoot", REDUCE = "Reduce", ELIMINATE = "Eliminate",
    SOLVE_VALUES = "SolveValues", N = "N", SET = "Set", SET_DELAYED = "SetDelayed", CLEAR = "Clear",
    REPLACE_ALL = "ReplaceAll", COMPOUND_EXPRESSION = "CompoundExpression", PART = "Part", SPAN = "Span",
    PATTERN = "Pattern", BLANK = "Blank", BLANK_SEQUENCE = "BlankSequence", BLANK_NULL_SEQUENCE = "BlankNullSequence",
    CONDITION = "Condition", HOLD = "Hold", HOLD_FORM = "HoldForm", SEQUENCE = "Sequence", MESSAGE_NAME = "MessageName",
    EXPAND = "Expand", FACTOR = "Factor", TOGETHER = "Together", CANCEL = "Cancel", APART = "Apart", SIMPLIFY = "Simplify",
    FULL_SIMPLIFY = "FullSimplify", COLLECT = "Collect", D = "D", PLOT = "Plot", CONTOUR_PLOT = "ContourPlot",
    GRAPHICS = "Graphics", OUT = "Out", ASSUMPTIONS = "Assumptions", AUTOMATIC = "Automatic", ALL = "All", NONE = "None",
    METHOD = "Method", CUBICS = "Cubics", QUARTICS = "Quartics", VERIFY_SOLUTIONS = "VerifySolutions",
    MAX_EXTRA_CONDITIONS = "MaxExtraConditions", GENERATED_PARAMETERS = "GeneratedParameters",
    WORKING_PRECISION = "WorkingPrecision", MAX_ITERATIONS = "MaxIterations", ACCURACY_GOAL = "AccuracyGoal",
    PRECISION_GOAL = "PrecisionGoal", TABLE = "Table", RANGE = "Range", MAP = "Map", APPLY = "Apply", LENGTH = "Length",
    // …执行者可以追加，但已列出的**顺序不可改变**（id 由顺序决定）
}
```

### 6.3 om-core：规范构造器（Canonical constructors）

```rust
// crates/om-core/src/canon.rs  —— 详细规则见 8.1
pub fn add(terms: impl IntoIterator<Item = Expr>) -> Expr;   // Plus
pub fn mul(factors: impl IntoIterator<Item = Expr>) -> Expr; // Times
pub fn pow(base: Expr, exp: Expr) -> Expr;                   // Power
pub fn neg(x: Expr) -> Expr;                  // mul([-1, x])
pub fn sub(a: Expr, b: Expr) -> Expr;         // add([a, neg(b)])
pub fn div(a: Expr, b: Expr) -> Expr;         // mul([a, pow(b, -1)])
pub fn sqrt(x: Expr) -> Expr;                 // pow(x, 1/2)
pub fn exp(x: Expr) -> Expr;                  // pow(E, x)
pub fn func(head: Symbol, args: Vec<Expr>) -> Expr; // 对已知初等函数做**最小**自动化简（见 8.1.6），其余等同 Expr::call
pub fn canonicalize(e: &Expr) -> Expr;        // 自底向上用上述构造器重建整棵树
// 排序
pub fn canonical_cmp(a: &Expr, b: &Expr) -> std::cmp::Ordering;  // 见 8.1.1
```

**铁律：** 除解析器输出和 `Expr::normal` 以外，所有代码构造 Plus/Times/Power 时**必须**使用上述构造器，以保证所有 `Expr` 都是规范的，结构相等 ⟺ 规范相等。

### 6.4 om-core：求值上下文、中断与消息

```rust
// 定义于 crates/om-num/src/ctx.rs；crates/om-core/src/ctx.rs 重导出 Clock/Interrupt/Abort。
// Message/MsgLevel/Messages 仍定义于 om-core。
pub trait Clock: Send + Sync { fn now_ms(&self) -> f64; }   // native: Instant；wasm: js Date.now()（由 om-wasm 注入）
pub struct Interrupt {
    pub flag: std::sync::Arc<std::sync::atomic::AtomicBool>, // 外部置 true 即中止
    pub deadline_ms: Option<f64>, pub clock: Option<std::sync::Arc<dyn Clock>>,
    pub steps_left: std::cell::Cell<u64>,                      // 计算预算（默认 5e8 次 tick）
}
#[derive(Debug, Clone, thiserror::Error)]
pub enum Abort { #[error("interrupted")] Interrupted, #[error("time limit exceeded")] Timeout, #[error("step budget exceeded")] Budget }
impl Interrupt {
    /// 所有可能耗时的循环（多项式乘法外层循环、Gröbner 每对 S-多项式、Hensel 每步、求值器每次重写）都要调用
    #[inline] pub fn tick(&self) -> Result<(), Abort>;          // steps_left -=1；每 4096 次检查一次 flag 与时钟
}
#[derive(Clone, Debug, serde::Serialize)]
pub struct Message { pub symbol: String, pub tag: String, pub text: String, pub level: MsgLevel } // 例：Solve::svars
#[derive(Clone, Copy, Debug, serde::Serialize)] pub enum MsgLevel { Info, Warning, Error }
pub struct Messages(Vec<Message>);  // push(), take()
```

所有可能长时间运行的公共函数签名最后一个参数是 `ctx: &Interrupt`，返回 `Result<T, Abort>` 或 `Result<T, XxxError>`（其中 `XxxError: From<Abort>`）。

### 6.5 om-parse 与 om-format（接口）

```rust
// crates/om-parse/src/lib.rs
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Dialect { Modern, Wolfram, Auto }
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)] pub struct Span { pub start: u32, pub end: u32 } // UTF-8 字节偏移
#[derive(Clone, Debug, serde::Serialize)]
pub struct Diagnostic { pub span: Span, pub severity: Severity, pub code: &'static str, pub message: String, pub fix: Option<Fix> }
#[derive(Clone, Debug, serde::Serialize)] pub struct Fix { pub span: Span, pub replacement: String, pub label: String }
#[derive(Clone, Copy, Debug, serde::Serialize)] pub enum Severity { Error, Warning, Hint }
pub struct ParseOutput {
    pub statements: Vec<Stmt>,           // 一个 cell 可以有多条语句（换行或 ; 分隔）
    pub diagnostics: Vec<Diagnostic>,
    pub dialect: Dialect,                // Auto 时给出检测结果
    pub tokens: Vec<(Span, TokenClass)>, // 供语法高亮
}
pub struct Stmt { pub expr: Expr, pub span: Span, pub suppress_output: bool } // 以 ; 结尾则 suppress
#[derive(Clone, Copy, Debug, serde::Serialize)]
pub enum TokenClass { Number, Identifier, Builtin, Operator, Bracket, String, Comment, Keyword, Error }
pub fn parse(src: &str, dialect: Dialect) -> ParseOutput;       // 永不 panic；有错误时 statements 中含 `$Failed`/部分结果
pub fn parse_expr(src: &str, dialect: Dialect) -> Result<Expr, Vec<Diagnostic>>; // 便捷：必须恰好一条语句
pub fn detect_dialect(src: &str) -> Dialect;                    // 规则见 7.3

// crates/om-format/src/lib.rs
pub fn full_form(e: &Expr) -> String;          // Plus[1, Times[2, x]]
pub fn input_form(e: &Expr) -> String;         // Wolfram InputForm: 1 + 2*x
pub fn modern_form(e: &Expr) -> String;        // 现代方言：1 + 2x，sin(x)，[1, 2]
pub fn latex(e: &Expr) -> String;              // KaTeX 可渲染
pub fn unicode_form(e: &Expr) -> String;       // 终端：x² + 2x − 3，√2，π
pub struct FormatOptions { pub display_order: bool /*默认 true：降幂显示*/ }
```

### 6.6 om-solve 输出（接口；步骤数据模型详见 8.10）

```rust
// crates/om-solve/src/types.rs
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum Domain { Complexes, Reals, Integers, Rationals }
#[derive(Clone, Debug)]
pub struct SolveOptions {
    pub domain: Domain /*默认 Complexes*/,
    pub cubics: bool   /*默认 false：不可约且非二项/非回文的三次 → Root 对象；true → Cardano 根式*/,
    pub quartics: bool /*默认 false：同上，true → Ferrari*/,
    pub verify: VerifyMode /*默认 Auto*/, pub max_extra_conditions: MaxExtra /*默认 Zero*/,
    pub generated_parameter: Symbol /*默认 C*/, pub inverse_functions: bool /*默认 true*/,
    pub record_steps: bool /*kernel 默认 true*/, pub seed: u64 /*默认 0x0A5E_ED00_0000_0001，确定性*/,
}
#[derive(Clone, Debug)] pub enum VerifyMode { Auto, Always, Never }
#[derive(Clone, Debug)] pub enum MaxExtra { Zero, All, Count(u32) }
/// 一个解 = 一组变量赋值 + 可选条件 + 生成参数
#[derive(Clone, Debug)]
pub struct Solution {
    pub rules: Vec<(Expr /*变量*/, Expr /*值*/)>,   // 自由变量不出现
    pub condition: Option<Expr>,                    // ConditionalExpression 的条件（不含 C[k] ∈ Integers）
    pub constants: Vec<(Expr /*C[k]*/, Domain)>,    // 生成参数及其所属集合
    pub multiplicity: u32,                          // Wolfram 输出时同一规则重复 m 次
    pub verification: Verification,
    pub numeric: Option<Vec<(f64, f64)>>,           // 每个变量的复数近似值（排序、可视化用）
}
#[derive(Clone, Copy, Debug, serde::Serialize)]
pub enum Verification { Exact, ByConstruction, Numeric { digits: u32 }, Unverified }
#[derive(Clone, Debug)]
pub enum SolutionSet {
    Finite(Vec<Solution>),     // {} 即 Finite(vec![])
    All,                       // {{}}：对任意值成立
    Region { cond: Expr, intervals: Vec<Interval> }, // Reduce/不等式：布尔条件（如 -2 < x < 2）+ 供 UI 的区间
    Unevaluated,               // 无法求解：Solve 内置函数原样返回输入
}
pub struct Interval { pub lo: Bound, pub hi: Bound }
pub enum Bound { NegInf, PosInf, Closed(Expr), Open(Expr) }
pub struct SolveOutcome { pub set: SolutionSet, pub steps: Option<Steps>, pub messages: Vec<Message> }
pub fn solve(eqs: &Expr, vars: &[Expr], opts: &SolveOptions, ctx: &Interrupt) -> Result<SolveOutcome, SolveError>;
pub fn nsolve(eqs: &Expr, vars: &[Expr], precision: Precision, ctx: &Interrupt) -> Result<SolveOutcome, SolveError>;
pub fn find_root(eqs: &Expr, starts: &[(Expr, Number)], opts: &FindRootOptions, ctx: &Interrupt) -> Result<SolveOutcome, SolveError>;
pub fn reduce(expr: &Expr, vars: &[Expr], domain: Domain, ctx: &Interrupt) -> Result<SolveOutcome, SolveError>;
pub fn eliminate(eqs: &Expr, elim: &[Expr], ctx: &Interrupt) -> Result<Expr, SolveError>;
impl SolutionSet { pub fn to_expr(&self) -> Expr; }   // Wolfram 形状：{{x -> 1}, {x -> 2}}
#[derive(Debug, thiserror::Error)]
pub enum SolveError { #[error(transparent)] Abort(#[from] Abort), #[error("unsupported: {0}")] Unsupported(String), #[error("invalid input: {0}")] Invalid(String) }
```

`Unsupported` 不是崩溃：`Solve` 内置函数捕获它，发出消息 `Solve::nsmet`（"This system cannot be solved with the methods available to Solve."），并**原样返回未求值的 `Solve[...]`**，与 Mathematica 行为一致。

---

## 7. 输入语言规格

### 7.1 共同的内部语义
两种方言产生**完全相同**的 `Expr`。例如现代方言 `solve(x^2 + 2x = 3, x)` 和 Wolfram 方言 `Solve[x^2 + 2 x == 3, x]` 都得到 `Solve[Equal[Plus[Power[x,2], Times[2,x]], 3], x]`（解析器输出的是**未规范化**树；求值器第一步会 `canonicalize`）。

### 7.2 现代方言（默认）

| 构造 | 语法 | 内部表示 |
|---|---|---|
| 数字 | `42`、`3.14`、`1e-3`、`1.5e10`、`0x1F`、`1_000_000` | Integer；十进制小数 → Real::Machine；有效数字 > 16 位 → Real::Big(精度=ceil(digits·log2(10)))|
| 精确分数 | `1/3`（求值时变成 Rational） | Times[1, Power[3,-1]] → 规范化后是 Rational |
| 标识符 | 字母/`_`/Unicode 字母开头，后接字母数字；`xy` 是**一个**符号 | Symbol |
| 希腊字母 | `α β θ π` 等直接可用；`pi`、`π` 均为 Pi | |
| 常量 | `pi` `π` → Pi；`e` → E；`i` → I；`inf` `∞` `infinity` → Infinity | 笔记本设置 `constants = "strict"` 时关闭 `e`、`i` 的映射 |
| 函数调用 | `f(x, y)`：标识符与 `(` **之间无空格** | 函数名按 7.4 映射表转内置名，否则保留原名 |
| 隐式乘法 | `2x`、`2 x`、`x y`、`2(x+1)`、`(x+1)(x-1)`、`x (y+1)`（有空格）、`2sin(x)`、`3π` | Times |
| 幂 | `x^2`、`x**2`、`x²`（Unicode 上标数字 ⁰¹²³⁴⁵⁶⁷⁸⁹ 与 ⁻）、右结合 | Power |
| 根号 | `sqrt(x)`、`√x`、`√(x+1)`、`cbrt(x)`（→ Power[x,1/3]，实数分支见 7.5）、`root(x, n)` | |
| 方程 | `a = b` 与 `a == b` **都**是 Equal（`=` 只在 `let` 语句中是赋值） | Equal |
| 不等 | `!=` `≠` `<` `<=` `≤` `>` `>=` `≥`；链式 `0 < x <= 1` | Inequality 规范化为 And[Less[0,x], LessEqual[x,1]] |
| 逻辑 | `and` `&&` `∧`；`or` `\|\|` `∨`；`not` `!` `¬` | And/Or/Not |
| 列表 | `[1, 2, 3]` 或 `{1, 2, 3}` | List |
| 下标取值 | `v[1]`（标识符后紧跟 `[` 且无空格） | Part[v, 1]（现代方言 1 起始） |
| 规则 | `x -> 1`、`x → 1` | Rule |
| 替换 | `expr where x = 2, y = 3` 或 `expr /. x -> 2` | ReplaceAll |
| 赋值 | `let a = 5`；`let f(x) = x^2`（→ SetDelayed[f[x_], x^2]） | Set / SetDelayed |
| 关键字参数 | `solve(x^2 < 4, x, domain: reals)`、`nsolve(..., precision: 30)` | 见 7.4 “关键字参数映射” |
| 注释 | `# 到行尾` | 丢弃 |
| 语句 | 换行或 `;` 分隔；以 `;` 结尾的语句不显示输出 | |
| 上次输出 | `%`、`%3`、`out(3)` | Out[] / Out[3] |
| 阶乘 | `5!`（后缀）；前缀 `!` 仍是 Not | Factorial |
| 绝对值 | `abs(x)` 或 `|x|`（`|` 成对出现且不在 `||` 中） | Abs |

**优先级（从低到高）：** `;` < `where` < `->` < `or` < `and` < `not` < 比较（`= == != < <= > >=`，可链式）< `+ -` < `* /` 与隐式乘法 < 一元负号 < `^`（右结合；`-x^2` = `-(x^2)`；`2^-1` 合法）< 后缀 `!` < 调用/下标 `f(x)` `v[1]` < 原子。

**关键歧义的裁决（写进测试）：**
- `2x^2` = `2*(x^2)`；`2^x y` = `(2^x)*y`；`x^2y` = `(x^2)*y`（隐式乘法优先级低于 `^`）。
- `f (x)`（有空格）= `f*x`，同时给出 Hint 诊断 `W001`：“`f (x)` 被当作乘法；如果想调用函数请去掉空格”。
- `sin x` → 诊断 Error `E010`：“函数需要括号：sin(x)”，附带 Fix。
- `a(b+c)`：`a` 未知且紧跟 `(` → **函数调用** `a[b+c]`；诊断 Hint `W002`：“`a` 不是已知函数；如果想表示乘法，写成 `a*(b+c)`”，附带 Fix。已知函数 = 内置函数表 ∪ 本会话用 `let` 定义过的函数（kernel 在解析时传入 `known_functions` 集合，见 `parse_with(src, dialect, &ParseEnv)`）。
- `x = 3` 单独成句：解析为 Equal；kernel 在 UI 上提供动作按钮 “求解 x” 与 “赋值 let x = 3”（见 12.3）。
- `|x| + |y|`：用栈式配对；`|` 后面是“表达式起始”时视为开，否则视为闭。

### 7.3 Wolfram 方言
支持 Mathematica 语法的子集，并**保证第 14 节语料中所有 Wolfram 语法都能解析**：
- 调用 `f[x]`，Part `v[[1]]`，列表 `{}`，`==` `!=` `===`（SameQ）`<` `<=` `>` `>=`，`&&` `||` `!`，`->` `:>` `/.` `//.`，`=` `:=`（Set/SetDelayed）`=.`（Unset），`;`（CompoundExpression），`/;`（Condition），`_` `x_` `x_Integer` `x__` `x___`，`#` `#1` `&`（纯函数），`@`（前缀调用）、`//`（后缀调用）、`f@@x`（Apply）、`f/@x`（Map），`'`（导数 `f'[x]` → `Derivative[1][f][x]`），`(* 注释 *)`（可嵌套），字符串 `"..."`，数字 `1.5`、`1.5*^-3`、`1.5`30`（精度标记）、`2^^1011`（基数，可选实现），`\[Pi]` 等具名字符（至少支持 Pi, Alpha…Omega 希腊字母, Infinity, Element, Equal, LessEqual, GreaterEqual, NotEqual, Rule）。
- 隐式乘法：**只有**空白或并置（`2 x`、`2x`、`x y`、`(a)(b)`），与 Mathematica 一致。
- 官方优先级按 Wolfram 文档 “Operator Input Forms” 表格实现；v1 只需要上面列出的运算符。
- **方言自动检测 `detect_dialect`：** 按顺序：(1) 首行是 `%wl` 或 `%modern` 注释标记 → 相应方言（该行从源码中去掉）；(2) 正则 `[A-Za-z][A-Za-z0-9]*\[` 且方括号**不是**紧跟已知小写函数名的下标用法，或出现 `==` `->` `:=` `/.` `(*` 中任意两种 → Wolfram；(3) 否则 → Modern。检测结果在 UI 的 cell 角标上显示，用户可以点击切换并固定。

### 7.4 名称映射表（现代 → 内置）
大小写不敏感，另外所有 Wolfram 原名（如 `Solve`、`Sin`）在现代方言中也可以直接用。

```
solve→Solve  nsolve→NSolve  findroot/find_root→FindRoot  reduce→Reduce  eliminate→Eliminate  solvevalues→SolveValues
sin cos tan cot sec csc → Sin…; asin/arcsin→ArcSin, acos/arccos→ArcCos, atan/arctan→ArcTan (2 参数 atan2(y,x)→ArcTan[x,y]，注意参数顺序)
sinh cosh tanh asinh acosh atanh → 对应; exp→Exp; ln→Log; log(x)→Log[x]; log(b, x)→Log[b,x]; log10→Log[10,·]; log2→Log[2,·]
sqrt→Sqrt  cbrt→CubeRoot(实分支)  abs→Abs  sign/sgn→Sign  re→Re  im→Im  conj→Conjugate  arg→Arg
floor ceil/ceiling round mod gcd lcm → 对应; factorial→Factorial; binom/binomial/choose→Binomial
expand factor simplify together cancel apart collect → 对应; diff/d/derivative→D; n/numeric→N
plot→Plot  implicitplot→ContourPlot(方程)  table→Table  range→Range  length/len→Length  map→Map
lambertw/productlog→ProductLog  root→(2 参数) Power[x,1/n]
```

**关键字参数映射：** `domain: reals|integers|complexes|rationals|positives` → 作为 Solve/Reduce 的第三个位置参数（`positives` → 追加条件 `v > 0` 并使用 Reals）；`precision: n` → `WorkingPrecision -> n`；`steps: false` → 关闭步骤记录；`method: "..."` → `Method -> "..."`；其余 `key: v` → `Rule[Key 的 Wolfram 驼峰名, v]`（例如 `max_iterations: 100` → `MaxIterations -> 100`）。

### 7.5 两种方言都成立的约定
- `cbrt(x)`/`CubeRoot[x]` 取**实数立方根**（`CubeRoot[-8] = -2`），而 `(-8)^(1/3)` 取主值（复数），与 Mathematica 相同。
- 求解变量未给出时（`solve(x^2 = 1)`）：使用方程中全部自由符号，按字母序排列；如果超过方程个数，发出 `Solve::svars` 警告并对前 n 个变量求解（这与 Mathematica 不同；Mathematica 会报错或给出参数解。差异写进 `docs/solve.md`）。


---

## 8. 算法规格

> 本节是最难的部分。**严格按伪代码实现**。标 **[不确定]** 的地方表示 Mathematica 的确切行为未经核实：这类情况的测试一律用数值验证（`~`），不做字符串比较；我们自己的选择写在 `docs/solve.md` 的“差异”一节。

### 8.1 规范形式（om-core/canon.rs）

#### 8.1.0 数与传染规则
- `Rational` 必须约分且分母 > 0，分母为 1 时归一为 `Integer`。`Complex` 的实部和虚部属于同一类（`Complex[1, 2.]` → `Complex[1., 2.]`）。精确的 `im == 0` 归一为实部；近似的 `0.` 虚部则保留为复数。`I = Complex[0, 1]`。
- 传染：精确 ∘ Real → Real（取该 Real 的精度）；Machine ∘ Big → Machine；Big(p1) ∘ Big(p2) → Big(min(p1, p2))；Machine 运算溢出时改用 53 bit 的 Big 重算（**永远不产生** inf/NaN）。
- 特殊值：`DirectedInfinity[z]`（`Infinity = DirectedInfinity[1]`，`-Infinity = DirectedInfinity[-1]`），`ComplexInfinity = DirectedInfinity[]`，`Indeterminate`。解析器把 `Infinity` 符号直接转成 `DirectedInfinity[1]`。

#### 8.1.1 全序 `canonical_cmp(a, b)`（仅供内部使用；打印器另有显示顺序）
1. 两个都是数：先比实部（数值），再比虚部。数值相等时精确数在前，其次精度低的在前。
2. 数排在非数之前。
3. 其余情况用 `cmp_term`：令 `factors(e)` 为 `Times` 去掉首个数值系数后的参数列表（非 Times 时为 `[e]`）。**从最后一个元素往前**逐个用 `cmp_factor` 比较；全部相等时短的在前；仍相等再比数值系数（缺省视为 1）。
4. `cmp_factor(p, q)`：把各自看作 `(base, exp)`（非 Power 时 exp = 1）。先用 `cmp_atom` 比 base，再用 `canonical_cmp` 比 exp。
5. `cmp_atom`：Symbol < String < Normal。Symbol 按 `(name.to_lowercase(), name)` 比较（小写优先）；String 按字节比较；Normal 先用 `canonical_cmp` 比 head，再从左到右逐个比参数，最后比参数个数。

#### 8.1.2 `add(args)`（Plus）
1. 展平嵌套的 Plus。
2. 任一参数是 `Indeterminate` → 返回 `Indeterminate`。
3. 无穷：收集所有 `DI[z]` 项的方向。方向不同，或 `ComplexInfinity` 与其他无穷同时出现 → `Indeterminate`；否则无穷吸收所有有限项和符号项（`DI[z] + x → DI[z]`）。
4. 把所有数加总为 `c`（按传染规则）。
5. 每个非数项 t 拆成 `(coef, rest)`：`Times[k, r...] → (k, Times[r...])`（rest 只有一个因子时就是该因子），其他 → `(1, t)`。
6. 按 `canonical_cmp(rest)` 排序；rest 相同的合并（系数相加）；丢弃**精确 0** 系数的项。Real 系数保留（`x − 1.0x → 0. x`，再由 Times 规则变成 `0.`）。
7. 每项用 `mul([coef, rest])` 重建。`c` 为精确 0 时丢弃；`0.` 保留（`x + 0.` → `Plus[0., x]`，与 Mathematica 相同）。
8. 用 `canonical_cmp` 排序；0 项 → `0`，1 项 → 该项，否则 → `Plus[...]`。
**复杂度要求：** 先排序再合并，O(n log n)。禁止 O(n²) 的两两比较。

#### 8.1.3 `mul(args)`（Times）
1. 展平；应用 Indeterminate 规则。
2. 把所有数乘为 `c`。
3. 无穷：精确 0 × 无穷 → `Indeterminate`；否则 `c · DI[z] → DI[sign(c·z)]`，其中 `sign(w) = w/|w|`；ComplexInfinity 吸收。
4. `c` 为精确 0 → 返回 `0`；`c` 为 `0.` → 返回 `0.`。
5. 按 base 分组（`(base, exp)` 的含义同 `cmp_factor`），同一 base 的指数相加，用 `pow(base, sum)` 重建。结果可能是数（例如 `Sqrt[2]^2 = 2`），把它乘回 `c`。迭代到不动点（因子个数只减不增，所以一定终止）。
6. **数值根式合并：** 在形如 `Power[r, e]`（r 是正有理数，e 是非整数有理数）的因子中：e 相同的两个 → `pow(r1·r2, e)`；指数互为相反数（e 与 −e，e > 0）→ `pow(r1/r2, e)`。重复到没有可合并项为止。
7. **−1 分配：** `c == −1` 且除它以外只剩一个 Plus 因子时，分配进去：`-(1+x) → -1 - x`。Mathematica 只对 −1 这样做，`-2(1+x)` 保持不变。
8. 按 `canonical_cmp` 排序；`c` 不是精确 1 时放在最前面；0 个参数 → `1`，1 个参数 → 该参数。

#### 8.1.4 `pow(b, e)`（Power），按顺序判定
1. 任一为 Indeterminate → Indeterminate；`0^0`、`DI^0`、`1^DI` → Indeterminate。
2. `e` 为精确 0 → 1；`e == 1` → b；`b == 1` → 1。
3. `b == 0`：e 为数且 Re(e) > 0 → 0；e 为数且 Re(e) < 0 → `ComplexInfinity`，并发出消息 `Power::infy`；e 为符号 → 保持不求值。
4. `b = DI[z]`：e 为负数 → 0；e 为正整数 → `DI[z^e]`。
5. 两个都是数：Int/Rat 的 Int 次幂精确计算，结果超过 `MAX_EXACT_BITS = 2^24` bit 时保持不求值并发出 `General::ovfl`；任一方为 Real/Complex → 数值主值幂；**有理数的有理数次幂** `r^(p/q)`（q > 1）→ 第 6 步。
6. **根式提取**，`r = ±a/b`，指数 `p/q`：
   - r < 0 → `mul([pow(-1, p/q), pow(a/b, p/q)])`。
   - `(-1)^(p/q)`：把 p/q 对 2 取模，得到 e ∈ [0, 2)。e = 0 → 1；e = 1 → −1；e = 1/2 → I；e = 3/2 → −I；e ∈ (1, 2) → `-(−1)^(e−1)`；e ∈ (0, 1) → 保持 `Power[-1, e]`。
   - 把 p 写成 `sgn·(k·q + s)`，其中 0 ≤ s < q；结果 = `(a/b)^(sgn·k) · (a/b)^(sgn·s/q)`。
   - 提取 q 次方因子：`a = c^q · a'`，`b = d^q · b'`（`om_num::extract_root_factor`，只做有界试除（2^16 以下的素数）加上精确完全幂检验，**构造时不做完整因式分解**）；系数为 `(c/d)^(sgn·s)`，根式为 `(a'/b')^(sgn·s/q)`。
   - 完全幂约化：若 `a' = g^m` 且 `gcd(m, q) > 1`，改写后递归（`4^(1/4) → 2^(1/2)`）；b' 同理。
   - 输出：b' = 1 → `Power[a', t]`；a' = 1 → `Power[b', −t]`；否则 → `Power[Rational[a', b'], t]`。
7. `b = E` 且 `e = Log[z]` → z。`e = Times[Complex[0, r], Pi]`（r 是分母为 1 或 2 的有理数）→ ±1 或 ±I。
8. `b = Power[x, a]`：当 e 是整数，或者 a 是满足 −1 < a < 1 的实有理数（主值分支安全）时 → `pow(x, a·e)`；否则保持。
9. `b = Times[...]`：e 为整数 → `mul(每个因子 ^ e)`；否则把正有理数值因子提出来（`(2x)^(1/2) → Sqrt[2]·x^(1/2)`），剩余部分保持 `Power[Times[rest], e]`。
10. 其他 → `Power[b, e]`。

#### 8.1.5 规范形式测试向量（输入以 Wolfram 语法解析后 canonicalize，比较 FullForm）
| # | 输入 | FullForm |
|---|---|---|
|1| `a-b` | `Plus[a, Times[-1, b]]` |
|2| `x+x` | `Times[2, x]` |
|3| `2x+3x` | `Times[5, x]` |
|4| `x*x` | `Power[x, 2]` |
|5| `x^2*x^-2` | `1` |
|6| `y+x+2` | `Plus[2, x, y]` |
|7| `x^2+x` | `Plus[x, Power[x, 2]]` |
|8| `0*x` | `0` |
|9| `Sqrt[12]` | `Times[2, Power[3, Rational[1, 2]]]` |
|10| `8^(1/3)` | `2` |
|11| `Sqrt[2]Sqrt[3]` | `Power[6, Rational[1, 2]]` |
|12| `Sqrt[2]Sqrt[2]` | `2` |
|13| `Sqrt[2]/Sqrt[3]` | `Power[Rational[2, 3], Rational[1, 2]]` |
|14| `Sqrt[1/2]` | `Power[2, Rational[-1, 2]]` |
|15| `Sqrt[12/5]` | `Times[2, Power[Rational[3, 5], Rational[1, 2]]]` |
|16| `2^(-3/2)` | `Times[Rational[1, 2], Power[2, Rational[-1, 2]]]` |
|17| `4^(1/4)` | `Power[2, Rational[1, 2]]` |
|18| `(-1)^(1/2)` | `Complex[0, 1]` |
|19| `(-8)^(1/3)` | `Times[2, Power[-1, Rational[1, 3]]]` |
|20| `(-1)^(4/3)` | `Times[-1, Power[-1, Rational[1, 3]]]` |
|21| `(x^2)^(1/2)` | `Power[Power[x, 2], Rational[1, 2]]` |
|22| `(x^(1/2))^2` | `x` |
|23| `(x^(1/2))^(1/3)` | `Power[x, Rational[1, 6]]` |
|24| `(x y)^2` | `Times[Power[x, 2], Power[y, 2]]` |
|25| `(x y)^(1/2)` | `Power[Times[x, y], Rational[1, 2]]` |
|26| `(2x)^(1/2)` | `Times[Power[2, Rational[1, 2]], Power[x, Rational[1, 2]]]` |
|27| `1/2+1/3` | `Rational[5, 6]` |
|28| `1+2.5` | `3.5` |
|29| `x+1.0x` | `Times[2., x]` |
|30| `I^2` | `-1` |
|31| `(1+I)(1-I)` | `2` |
|32| `1/0` | `ComplexInfinity`（并发出 `Power::infy`） |
|33| `0/0` 与 `0^0` | `Indeterminate` |
|34| `Infinity-Infinity` | `Indeterminate` |
|35| `-2*Infinity` | `DirectedInfinity[-1]` |
|36| `I*Infinity` | `DirectedInfinity[Complex[0, 1]]` |
|37| `E^Log[x]` | `x` |
|38| `-(1+x)` | `Plus[-1, Times[-1, x]]` |
|39| `-2(1+x)` | `Times[-2, Plus[1, x]]` |
|40| `x^0` | `1` |

另外加一个 **proptest 属性测试**：随机生成深度 ≤ 4、由 `{x, y, 整数 −3..3, 1/2}` 组成的 Plus/Times/Power 树 t，检查 (a) `canonicalize(canonicalize(t)) == canonicalize(t)`（幂等）；(b) 把 x、y 代入随机有理数后，t 与 `canonicalize(t)` 的数值相等（相对误差 1e−9，跳过出现除零的样本）。

#### 8.1.6 初等函数特殊值（om-simplify/special.rs；om-eval 与 om-solve 共用）
- `Sin`/`Cos`/`Tan` 在 `r·Pi` 处取值，r 的分母 ∈ {1, 2, 3, 4, 6, 12}：先用周期性和对称性约化到 [0, π/2]，再查表（例如 `Sin[Pi/12] = (Sqrt[6]-Sqrt[2])/4`）。
- 奇偶性：`Sin[-x] = -Sin[x]`、`Cos[-x] = Cos[x]`，仅当参数是 `Times[负数, …]` 时才用。
- `ArcSin`/`ArcCos`/`ArcTan` 在 `{0, ±1/2, ±Sqrt[2]/2, ±Sqrt[3]/2, ±1, ±Sqrt[3], ±1/Sqrt[3]}` 处取值。
- `Log[1] = 0`、`Log[E] = 1`、`Log[E^k] = k`（k 为实数值）；`Exp` 就是 `Power[E, x]`。
- 精确参数是负实数时：`Log[-2] = Log[2] + I Pi`。
- `Sqrt` 就是 `Power[_, 1/2]`，已由 canon 处理。
- `Abs`/`Re`/`Im`/`Conjugate` 作用于数值时直接计算；`Abs[-x] = Abs[x]`。
- `ProductLog[0] = 0`、`ProductLog[E] = 1`、`ProductLog[-1/E] = -1`。
- 对数值参数（Real）一律调用数值实现求值。
**测试：** 表中每一项各写一条测试，另外对每一项在 50 位精度下做数值交叉验证。

### 8.2 多项式层（om-poly，只依赖 om-num）

**类型：**
```rust
pub trait Ring: Clone + PartialEq + Debug { fn zero() -> Self; fn one() -> Self; fn is_zero(&self) -> bool;
    fn add(&self, o:&Self)->Self; fn sub(&self,o:&Self)->Self; fn mul(&self,o:&Self)->Self; fn neg(&self)->Self; }
pub trait EuclideanRing: Ring { fn divrem(&self, o:&Self) -> (Self, Self); fn exact_div(&self, o:&Self) -> Option<Self>; }
pub trait Field: Ring { fn inv(&self) -> Option<Self>; }
// 实现：IBig(Ring+EuclideanRing)、RBig(Field)、FpElem{v:u64, p:u64}(Field)、MPoly<IBig>(Ring，用于 Z[参数])
pub struct UPoly<R: Ring> { pub coeffs: Vec<R> }   // 低次在前，末尾无 0；零多项式 = 空 Vec
pub struct Monomial { pub exps: SmallVec<[u32; 4]>, pub deg: u32 }
pub enum MonoOrder { Lex, GrevLex }
pub struct MPoly<R: Ring> { pub nvars: usize, pub terms: Vec<(Monomial, R)>, pub order: MonoOrder } // 按 order 降序，无零系数
```
所有可能耗时的函数最后一个参数都是 `ctx: &om_num::ctx::Interrupt`，返回 `Result<_, om_num::ctx::Abort>`（om-core 重导出相同类型）。

#### 8.2a 除法、伪余式、容量
- `divrem(f, g)`（在域上）：教科书长除法。
- `prem(f, g)`（在 Z 上）：返回 `lc(g)^(deg f − deg g + 1) · f mod g`。循环 `r ← lc(g)·r − lc(r)·x^(deg r − deg g)·g`，直到 `deg r < deg g`，记录迭代次数 k，最后乘以 `lc(g)^(δ+1−k)`。
- `content(f)`：各系数的 gcd，符号取得使 `pp(f) = f/content` 的首项系数为正。
- **测试：** `(x³−2x²−4) ÷ (x−3)` 得 `q = x²+x+3`、`r = 5`；`prem(x²+1, 2x+1) = 5`；`content(6x²+4x+2) = 2`，pp 为 `3x²+2x+1`；`content(−6x+3) = −3`，pp 为 `2x−1`。

#### 8.2b GCD：v1 = GCDHEU 快速路径 + 子结式 PRS 兜底（二者一致性用 proptest 验证）
```
heugcd(f, g):                      # f, g ∈ Z[x1..xn]，非零；对最后一个变量递归
  if n == 0: return igcd(f, g)
  gc = igcd(content(f), content(g)); f /= content(f); g /= content(g)
  B = 2*min(maxnorm(f), maxnorm(g)) + 29
  ξ = max(min(B, 99*isqrt(B)), 2*min(maxnorm(f)/|lc(f)|, maxnorm(g)/|lc(g)|) + 2)
  重复 6 次:
    ff = f(xn=ξ); gg = g(xn=ξ)                 # n−1 元多项式或整数
    if ff != 0 and gg != 0:
      h = heugcd(ff, gg)（递归；失败向上传播）
      for cand in [ interp(h, ξ), f / interp(ff/h, ξ), g / interp(gg/h, ξ) ]:   # "/" 为精确试除，除不尽就跳过
        c = pp(cand)
        if c | f and c | g: return gc * c（归一化：lc > 0）
    ξ = (ξ * isqrt(isqrt(ξ)) * 73794) / 27011
  失败 -> subresultant_gcd(f, g)

interp(h, ξ):   # 对称的 ξ 进制展开，对整数系数递归
  out = []; while h != 0: d = h mod ξ（对称剩余，取值 (−ξ/2, ξ/2]）; out.push(d); h = (h − d)/ξ
  return Σ out[i]·xn^i
```
`subresultant_gcd`：在 `Z[x1..x_{n−1}][xn]` 上做递归 Collins PRS（更新规则与 8.2e 结式循环相同），结果 = `pp(最后一个非零余式) · gcd(contents)`。
**测试：** `gcd(x²−1, x²−3x+2) = x−1`；`gcd(2x²+4x+2, 4x+4) = 2x+2`；`gcd(x⁴−1, x⁶−1) = x²−1`；`gcd(x²−y², x²+2xy+y²) = x+y`；`gcd(x, 0) = x`。

#### 8.2c 无平方分解
在 Q 上用 Yun 算法（先取本原部分）：
```
g = gcd(f, f'); c = f/g; d = f'/g − c'; i = 1
while deg c > 0: a = gcd(c, d); if deg a > 0: out.push((a, i)); c = c/a; d = d/a − c'; i += 1
```
在 Fp 上（f 首一）：
```
sqf_fp(f): if f' == 0: return [(h, m·p) for (h, m) in sqf_fp(pth_root(f))]   # pth_root：x^(pk) 的系数 → x^k（Fp 中 a^(1/p) = a）
  c = gcd(f, f'); w = f/c; i = 1
  while deg w > 0: y = gcd(w, c); z = w/y; if deg z > 0: out.push((z, i)); i += 1; w = y; c = c/y
  if deg c > 0: out += [(h, m·p) for (h, m) in sqf_fp(pth_root(c))]
```
**测试：** `(x−1)(x−2)²(x−3)³` → `[(x−1,1),(x−2,2),(x−3,3)]`；`x²+2x+1` → `[(x+1,2)]`；在 F3 上 `x³+1 = (x+1)³` → `[(x+1,3)]`。

#### 8.2d Z[x] 上的因式分解（Zassenhaus）
```
factor_Z(f): cont, f = content(f), pp(f); 提出 x^k; for (g, m) in yun(f): for h in zassenhaus(g): push (h, m)

zassenhaus(f):  # 无平方、本原、lc>0，n = deg f
  if n <= 1: return [f]
  从固定素数表（3,5,7,11,13,17,19,23,29,31,37,41,43,47,53,59,61,67,71,73…，跳过 2）中取前 5 个满足
     p ∤ lc(f) 且 gcd(f̄, f̄') = 1 (mod p) 的素数；分别做 ddf+edf 得到因子数 r_p；取 r_p 最小者（并列取最小的 p）
  if r_p == 1: return [f]
  B  = (isqrt(n+1)+1) * 2^n * maxnorm(f) * |lc(f)|      # Mignotte 界 × lc
  l  = 最小的使 p^l > 2B 的 l；M = p^l
  L  = hensel_lift(p, f, mod p 首一因子列表, l)         # f ≡ lc(f)·ΠL_i (mod M)
  T = [0..r); s = 1; out = []; tried = 0
  while 2s <= |T|:
    found = false
    for S in combinations(T, s)（字典序）:
      ctx.tick()?; tried += 1; if tried > 65536: return out + [f]，并标记 PossiblyReducible
      tc = symmod(lc(f) * Π_{i∈S} L_i(0), M); if tc == 0 or (lc(f)*f(0)) % tc != 0: continue   # 常数项剪枝
      G = pp(symmod(lc(f) * Π_{i∈S} L_i, M))
      if G | f（在 Z 上精确整除）:
        out.push(G); f = f/G; T = T \ S; 从 L 中删去 S; found = true; break
    if !found: s += 1
  out.push(f); return out
```
Hensel 提升（二次收敛；von zur Gathen & Gerhard 算法 15.10；在平衡二叉因子树上进行）：
```
hensel_step(m, f, g, h, s, t):  # 前提 f ≡ gh (mod m)，sg + th ≡ 1 (mod m)，h 首一，lc(g) = lc(f)；运算都 mod m²
  e = f − g h;  (q, r) = divrem(s e, h);  g* = g + t e + q g;  h* = h + r
  b = s g* + t h* − 1;  (c, d) = divrem(s b, h*);  s* = s − d;  t* = t − t b − c g*
  return (g*, h*, s*, t*)        # 现在 mod m² 成立
hensel_lift(p, f, [f1..fr], l):
  if r == 1: return [ lc(f)^{-1}·f mod p^l（首一化） ]
  k = r/2; g = lc(f)·Π f1..fk, h = Π f(k+1)..fr (mod p); (s, t) = mod p 扩展 gcd
  m = p; 重复 ceil(log2 l) 次: (g, h, s, t) = hensel_step(m, f, g, h, s, t); m = m²
  mod p^l 约化; return hensel_lift(p, g, [f1..fk], l) ++ hensel_lift(p, h, [f(k+1)..fr], l)
```
Fp 上的 DDF/EDF（p 为奇素数）：
```
ddf(f): i = 1; h = x; out = []
  while deg f >= 2i: h = powmod(h, p, f); g = gcd(f, h − x); if g != 1: out.push((g, i)); f /= g; h = h mod f; i += 1
  if deg f > 0: out.push((f, deg f))
edf(f, d): if deg f == d: return [f]
  loop: a = 随机多项式(deg < deg f, 使用 SplitMix64); g = gcd(a, f); if 0 < deg g < deg f: return edf(g,d) ++ edf(f/g,d)
        g = gcd(powmod(a, (p^d − 1)/2, f) − 1, f); if 0 < deg g < deg f: return edf(g,d) ++ edf(f/g,d)
```
**陷阱：** Swinnerton-Dyer 多项式（如 `x⁴−10x²+1`）在**每个**素数下都分解成 1、2 次因子，重组是指数级的。所以有上限：超过上限就返回一个正确但未必不可约的分解，并标记 `PossiblyReducible`（LLL/van Hoeij 放到 v2）。`symmod` 必须取对称区间 (−M/2, M/2]。
**测试：** `x⁴+4 = (x²−2x+2)(x²+2x+2)`；`x⁶−1 = (x−1)(x+1)(x²−x+1)(x²+x+1)`；`6x²+x−2 = (2x−1)(3x+2)`；`x⁴+1` 不可约；`x⁴−10x²+1` 不可约；在 F3 上 `x⁴+1 = (x²+x+2)(x²+2x+2)`；在 F5 上 `ddf(x⁴−1) = [(x⁴−1, 1)]`。proptest：随机取 2–4 个次数 1–3、系数在 −5..5 的多项式 g_i，`factor_Z(Π g_i)` 各因子的乘积（带重数和 content）等于原多项式，且每个因子都整除原多项式。
**多元因式分解（Factor[x^2 − y^2]）：** v1 使用“Kronecker 代换 + 单元分解 + 试除”：把 y 代换为 x^D（D 大于 x 的次数上界），对单元多项式做分解，再对所有因子子集尝试逆代换并试除。上限 2^12 个子集，超出就只提取 content 和 gcd 公因子。v2 再换成 EEZ/Wang 算法。**测试：** `x²−y² = (x−y)(x+y)`；`x³−y³ = (x−y)(x²+xy+y²)`；`x²y+xy² = xy(x+y)`。

#### 8.2e 结式与判别式（Cohen 算法 3.3.7；D 可以是 Z 或 Z[参数]）
```
res(A, B): if A == 0 or B == 0: return 0
  a = cont(A); b = cont(B); A = pp(A); B = pp(B); g = h = 1; s = 1; t = a^deg B * b^deg A
  if deg A < deg B: swap(A, B); if deg A odd and deg B odd: s = −s
  loop:
    δ = deg A − deg B; if deg A odd and deg B odd: s = −s
    R = prem(A, B); A = B; B = R / (g * h^δ)          # 精确除法
    g = lc(A); h = g^δ / h^(δ−1)                        # 精确
    if B == 0: return 0
    if deg B == 0: break
  h = lc(B)^deg A / h^(deg A − 1); return s * t * h
disc(f) = (−1)^(n(n−1)/2) * res(f, f') / lc(f)
```
**测试：** `res(x²+1, x−1) = 2`；`res_y(x²+y²−1, y−x) = 2x²−1`；`disc(x²+bx+c) = b²−4c`；`disc(x³+px+q) = −4p³−27q²`；`res_y(y²−2, (x−y)²−3) = x⁴−10x²+1`。

#### 8.2f 实根隔离（Descartes 符号法则 + 二分 VCA）与 Aberth 复根
```
isolate(f):  # f 无平方，∈ Z[x]；返回按 lo 排序的 Vec<RootInterval{lo, hi: Rational, exact: bool}>
  out = []; if f(0) == 0: out.push(exact 0); f = f/x
  k = 满足 2^k > 1 + max_i |a_i/lc| 的最小整数          # Cauchy 界
  for sign in [+1, −1]: g = f(sign · 2^k · x)（取本原部分）
     stack = [(g, 0, 1)]                                 # g 在 (0,1) 中的根 t 对应原变量 sign·2^k·(a + (b−a)t)
     while pop (p, a, b):
       ctx.tick()?
       v = sign_variations( taylor_shift_1( reverse(p) ) )   # (0,1) 内根个数的 Descartes 上界
       if v == 0: continue; if v == 1: out.push(map(a, b)); continue
       m = (a+b)/2; pl = 2^deg(p) · p(x/2); pr = taylor_shift_1(pl)
       if pr(0) == 0: out.push(exact map(m)); pr = pr/x
       push (pr, m, b); push (pl, a, m)
  按 lo 排序后返回
refine(f, I, bits): 当 hi − lo > 2^−bits 时根据 sign(f(mid)) 二分；f(mid) == 0 时标记为精确根
```
**陷阱：** 映射负根时区间端点会翻转，存储时必须保证 `lo < hi`。
**测试：** `x²−2` 得到两个区间，分别包含 −1.4142 和 1.4142；`x³−2x` 有 3 个根，中间的是精确 0；`x⁴−10x²+1` 有 4 个区间，分别包含 ±0.31784 和 ±3.14626；`x²+1` 没有实根。

**Aberth–Ehrlich**（NSolve 与复根排序使用）：初值 `z_j = R·exp(i(2πj/n + 0.4))`，R 取 Cauchy 界；迭代 `w_j = (f/f')(z_j) / (1 − (f/f')(z_j)·Σ_{k≠j} 1/(z_j − z_k))`，`z_j −= w_j`，直到 `max|w_j| < 2^−(prec−4)`；最多 200 轮，停滞时精度加倍。**认证：** 用球算术计算圆盘 `D(z_j, n·|f(z_j)/f'(z_j)|)`，每个圆盘内至少有一个根；n 个圆盘两两不相交时，每个圆盘恰好含一个根。认证失败就提高精度重来。

#### 8.2g `Root[f, k]` 的编号顺序
Mathematica 文档：先是实根（升序），然后是非实根，共轭对相邻，并且 `Root[#^2+1&, 1] = −I`（虚部为负的在前）。不同共轭对之间的顺序 **[不确定]**（观察结果与“按实部升序”一致）。**我们的确定性规则：** 非实根按 `(Re 升序, |Im| 升序, Im 为负的在前)` 排序，用认证过的圆盘比较，直到键值能分开为止；如果精度到 `2^−400` 仍然重叠，就判定为并列，并按 |Im| 排序。
**测试：** `x³−2`：Root 1 ≈ 1.26，Root 2 ≈ −0.63−1.091i，Root 3 ≈ −0.63+1.091i；`x⁵−x+1`：Root 1 ≈ −1.1673，接着依次是 ≈ −0.1812∓1.0840i 和 ≈ 0.7649∓0.3525i 两对。

#### 8.2h Q 上的 Gröbner 基
算法是 Buchberger，配合 Gebauer–Möller 更新和 sugar 选择策略。系数取整数，每次约化后取本原部分，防止系数膨胀。多项式放在 arena 中，按 id 索引。
```
update(G, B, h):                     # Becker–Weispfenning
  C = [(h,g) for g in G]; D = []
  while C: p = (h, g1) = C.pop_front()
     if coprime(LM h, LM g1) or (C ∪ D 中不存在 (h, g2) 使 lcm(h,g2) | lcm(h,g1)): D.push(p)
  E = [(h,g) in D if not coprime(LM h, LM g)]
  B' = [(g1,g2) in B if not (LM h | lcm(g1,g2) and lcm(g1,h) != lcm(g1,g2) and lcm(g2,h) != lcm(g1,g2))] ++ E
  G' = [g in G if not LM h | LM g] ++ [h]; return (G', B')
groebner(F, ord):
  G = B = []; for f in F（按 LM 升序）: (G, B) = update(G, B, pp(f))
  while B: p = argmin_B (sugar, lcm 按 ord, 创建序号); h = NF(spoly(p), G)
     if h != 0: (G, B) = update(G, B, pp(h)); if h 是常数: return [1]
  return reduce(G)       # 去掉 LM 可被其他 LM 整除的元素；对每个元素做尾部约化；在 Q 上首一化；按 LM 升序排列
sugar(输入) = 全次数; sugar(spoly(f,g)) = max(sug f + deg(L/LM f), sug g + deg(L/LM g))，其中 L = lcm
```
- **零维判定：** 对每个变量 x_i，都存在某个 LM(g) 是纯幂 `x_i^k`。
- **维数：** 最大的变量子集 U，使得没有任何 LM 完全落在 `K[U]` 中（按子集大小从大到小搜索）。
- **策略：** 先计算 grevlex 基；若理想是零维且需要 lex 基，用 **FGLM** 转换；正维时直接计算 lex 基。
```
fglm(Ggr, lex):
  Blex = []; V = []（已化为行阶梯形的 NF 向量）; Glex = []; L = [1]
  while L: m = lex_min(L); L.remove(m)
    if 某个 LM(Glex) | m: continue
    v = NF_Ggr(m)，表示为 grevlex 标准单项式上的向量
    if v ∈ span(V): 解 v = Σ c_i V_i; Glex.push(m − Σ c_i Blex_i)
    else: Blex.push(m); V.push(v); L ∪= {x_j·m for all j}
  return Glex（排序后）
```
**测试（lex，x > y > z）：** `{x²+y²−1, x−y} → {x−y, y²−1/2}`；`{xy−1, x²−y} → {x−y², y³−1}`；cyclic-3 `{x+y+z, xy+yz+zx, xyz−1} → {x+y+z, y²+yz+z², z³−1}`；`{xy}` 不是零维，维数为 1；`{x+y, x+y+1} → {1}`。FGLM 的结果必须与直接计算 lex 得到的约化基完全一致（用 proptest 比较随机零维小系统）。

#### 8.2i 代数数（om-poly/alg.rs）
- `RealAlg { minpoly: UPoly<IBig>（本原、不可约、lc>0）, iv: (Rational, Rational) }`，1 次时直接是精确有理数；`ComplexAlg { minpoly, disk: CBall, index }`。
- 运算（RootReduce）：α+β 的零化多项式是 `res_y(p(y), q(x−y))`；αβ 的是 `res_y(p(y), y^deg q · q(x/y))`；1/α 的是 `reverse(p)`。对得到的多项式做因式分解，然后加细 α、β 的区间，直到 α∘β 的区间包围中恰好只含一个因子的一个根，由此选出正确的因子和根。
- 符号：一直加细到 0 不在区间内（最小多项式次数 ≥ 2 且不可约，所以值一定非零）。
- 比较：最小多项式相同时比较根的编号，否则加细到两个区间不相交。
- 次数上限 64，超过就返回 Unknown。
- **测试：** √2+√3 的最小多项式是 `x⁴−10x²+1`，根在 (3,4) 中；√2·√3 的零化多项式是 `(x²−6)²`，分解后得到 `x²−6`，根在 (2,3) 中；`Root[x³−2,1]³ − 2 = 0`。

### 8.3 Expr ↔ 多项式转换（om-simplify/convert.rs）
```rust
pub struct PolyView { pub gens: Vec<Expr> /*生成元：变量 + 非多项式子项*/, pub num: MPoly<RBig>, pub den: MPoly<RBig> }
pub fn to_rational_function(e: &Expr, vars: &[Expr]) -> PolyView;  // 分子分母形式（不约分）
pub fn from_mpoly(p: &MPoly<RBig>, gens: &[Expr]) -> Expr;           // 输出时使用规范构造器
```
**生成元归一化（最常见的静默 bug，必须测试）：** 同一个底数的分数次幂要合并成一个生成元，例如 `x^(1/2)` 与 `x^(1/3)` 合并为 `t = x^(1/6)`，并记录 `x = t^6`。`Sin[x]`、`E^x`、`Sqrt[2]` 以及参数符号都作为独立的生成元。`E^(2x)` 与 `E^x` **不在这里**合并，由 8.7 的核统一负责。
**测试：** `Sqrt[x] + x^(1/3)` 只有一个生成元 `x^(1/6)`，多项式为 `t³ + t²`；`(x^2-1)/(x-1)` 的 num 为 `x²−1`、den 为 `x−1`；`together(1/x + 1/y) = (x+y)/(x y)`。

### 8.4 零判定 `is_zero(e) -> Tri { Zero, NonZero, Unknown(ProbablyZero | NoInfo) }`（om-simplify/zero.rs）
- **L0（结构）：** 规范形式为 `0` 或 `0.` → Zero；非零数 → NonZero。
- **L1（有理标准形）：** 经过生成元归一化后做 together，分子多项式恒为 0 → Zero；若没有生成元且分子是非零常数 → NonZero。
- **L2（代数）：** 若所有生成元都是代数数（有理数的根式或 Root）：先做根式去嵌套，`Sqrt[a + b Sqrt[c]] = Sqrt[(a+r)/2] + sgn(b) Sqrt[(a−r)/2]`，条件是 `a² − b²c = r²`、r 为有理数且 a > 0；再按关系 `y^q − a` 约化并检查是否为 0；仍无法判定就用 RootReduce 求最小多项式（次数上限 64），最小多项式为 `x` 时才是 Zero。
- **L3（数值）：** 没有自由符号时，用 64/256/1024 bit 的球算术求值。某个球不含 0 → NonZero（这是严格结论）；所有球都含 0 且半径 < 2^−(p−20) → `Unknown(ProbablyZero)`。有自由符号时，在 3 个随机的高斯有理点上求值：只要有一个点 NonZero 就判 NonZero（说明表达式不恒为零）；全部都是 ProbablyZero → `Unknown(ProbablyZero)`。
- 不含超越函数的有理表达式，经过 L1 后**不会**得到 Unknown。
- **测试：** `Sqrt[2]*Sqrt[3] - Sqrt[6]` → Zero；`Sqrt[3+2 Sqrt[2]] - 1 - Sqrt[2]` → Zero；`Sin[x]^2+Cos[x]^2-1` → Unknown(ProbablyZero)；`Pi - 355/113` → NonZero；`(x+1)^2 - x^2 - 2x - 1` → Zero。

### 8.5 simplify（om-simplify/simplify.rs）
最佳优先搜索。可用变换：`together`、`cancel`、`expand`、`factor`、`factor_terms`、`denest`、`RootReduce`（仅作用于代数数）、`PowerExpand`（仅在明确假设下）、少量三角规则（`Sin²+Cos² → 1`，倍角公式双向）。代价为 `LeafCount`，其中整数计为 `1 + ⌈log10|n|⌉/4`，Root 计为 3。保留最优结果，按 hash 做记忆化，最多扩展 50 个节点，每个节点都调用 `ctx.tick()`。debug 构建中断言结果与输入的差经 L3 检验为零。
`expand`：乘法对加法完全分配，正整数次幂用多项式展开（二项式定理）。`together`：通分。`cancel`：通分后用 GCD 约去公因子。`factor`：`factor_Z`，结果按因子次数升序排列，常数在最前面（与 Mathematica 的 `Factor` 输出一致，例如 `Factor[x^2-1] = (-1 + x) (1 + x)`）。

### 8.6 Solve 分派（om-solve/dispatch.rs）
```
solve(input, vars?, dom, ctx):
 P0 归一化:
   把 List/And 展平为合取式；Or → 各个析取分支分别求解，结果取并集并去重（结构比较 + 对差值调用 is_zero）
   Equal[a,b] → 方程 a−b；链式 a==b==c → a−b 与 b−c；True → 丢弃；False → 返回 {}
   Unequal[a,b] → 排除条件 (a−b ≠ 0)；Element[v, D] → 该变量的定义域
   Less/LessEqual/...：若为一元且 dom 是 Reals（或由不等式隐含）→ reduce_ineq（8.9）；否则 Unevaluated，并发出 Solve::ineq
   未给出 vars：取 {Pi, E, I, C[_], 内置符号} 以外的自由符号，按名字排序；个数多于方程数时发出消息（见 7.5）
   没有方程：返回 All（仍受排除条件约束）
 P1 排除条件（作用于**原始**表达式，在任何化简之前）:
   对每个子项 Power[b, e]（e 为数值且 Re(e) < 0）：b ≠ 0
   Log[b]: b ≠ 0；Tan[u]、Sec[u]: Cos[u] ≠ 0；Cot[u]、Csc[u]: Sin[u] ≠ 0
   eq_i := numerator(together(eq_i))（分子分母的公因子**不约去**，由排除条件处理）
 P2 分类:
   poly    = 经过 P1 后每个方程都是 vars 的多项式（系数可以含参数）
   linear  = poly 且关于 vars 的全次数 ≤ 1
   方程数 = 1 且变量数 = 1 → univariate(eq, x)（8.7）
   linear → linear_system（8.8）；poly → poly_system（8.8）；否则 → nonpoly_system（8.8）
 P3 验证（8.9 之前的 8.8.4）、定义域过滤（8.8.3）、排除条件过滤、排序（8.6.1）、附上数值、写入步骤
```
#### 8.6.1 解的输出顺序（确定性规则）
按各变量规则右端的数值，对变量按字典序比较；每个值的比较顺序为：实数在非实数之前，然后按 Re 升序，再按 Im 升序；仍然并列就用 `canonical_cmp`。重根按重数重复输出（`Solve[(x-1)^2==0,x] = {{x->1},{x->1}}`）。带 C[k] 的解族按 C = 0 时的数值排序。测试中顺序 **[不确定]** 的用例按多重集比较。

### 8.7 一元方程
```
univariate(e, x):
  if e 是 x 的多项式: return poly_uni(e, x)
  K = e 中依赖 x 的最大非多项式“核”（生成元归一化之后）
  if K 全是 Power[b, p/q] 且 b 是 x 的有理式: return radical_path(e, x)（8.7.3）
  if 通过 8.7.4 的核统一能把 K 化为单个核 k(x): y 为新变量; P = e[k → y]
       if P 不含 x: ys = univariate(P, y); 对每个 y_i 求解 k(x) == y_i（8.7.4 反函数表）; 取并集返回
  if e = f(g(x)) − c 且 f 在反函数表中（从外往里剥）: 对每个分支 return univariate(g(x) − f^{-1}(c))
  if dom == Reals 且出现 Abs[u]: 分情况 u ≥ 0（Abs[u] → u）与 u < 0（Abs[u] → −u），分别求解后按各自的条件过滤
  else: 发出消息 Solve::nsmet; return Unevaluated
```
#### 8.7.1 `poly_uni(p, x)`
1. p ≡ 0 → All；p 是非零常数 → {}。含参数时，假定首项系数非零，并记录 `GenericAssumption` 步骤。
2. 系数全为数值（Q）时：`yun` → 对每个无平方部分做 `factor_Z` → 对每个不可约因子 h 调用 `roots_irr(h)`，每个根带上所在无平方部分的重数。
3. 含参数时：只用 content、x^k、次数 ≤ 2 的求根公式、二项式 `a x^n + b`、以及 `p(x^k)` 代换；其他情况返回符号形式的 `Root[p(#1)&, k]`。

#### 8.7.2 `roots_irr(h)`，d = deg h
- **d = 1**：`−h0/h1`。
- **d = 2**：`(−b ∓ Sqrt[D])/(2a)`，`D = b² − 4ac` 经过规范化（Sqrt 会提出平方因子，D < 0 时给出 `I·Sqrt[−D]`）。
- **二项式** `a x^d + b`：`c = −b/a`，根为 `c^(1/d) · (−1)^(2k/d)`（k = 0..d−1），用 `pow`/`mul` 构造，这样形式与 Mathematica 一致（例如 `-(−1)^(1/3) 2^(1/3)`）。
- **h(x) = g(x^m)**（取最大的 m > 1）：递归解 `g(y) = 0`（仅当 g 的根全部能用根式表示时），再对每个 `x^m = y_j` 用二项式规则。双二次方程属于这种情况。
- **回文多项式**（偶数次 2m）：令 `z = x + 1/x`，得到关于 z 的 m 次方程；解出 z_j 后，再解 `x² − z_j x + 1 = 0`。
- **d = 3**：`Cubics → False`（默认）时返回 `Root[h, 1..3]`；`Cubics → True` 时用 Cardano：代换 `x = t − b/(3a)` 得到 `t³ + pt + q`，`Δ = −4p³ − 27q²`，`u = (−q/2 + Sqrt[q²/4 + p³/27])^(1/3)`，`v = −p/(3u)`，三个根为 `u+v`、`ωu + ω̄v`、`ω̄u + ωv`，其中 `ω = (−1)^(2/3)`。Δ > 0（不可约情形）时中间量为复数，这是允许的。**[不确定]** Mathematica 的默认是否就是 Root，测试用数值验证。
- **d = 4**：`Quartics → False`（默认）时返回 `Root`；为 True 时用 Ferrari（预解三次式 → 两个二次式）。**[不确定]**
- **d ≥ 5**：返回 `Root[h(#1)&, k]`，k = 1..d，编号按 8.2g。
- **UI 补充**（“比 Mathematica 更现代”）：kernel 对输出中的 Root 对象，额外尝试 `ToRadicals`（即 cubics/quartics = true 重新求解），成功时在解卡片上提供“根式形式”切换，并始终给出 20 位数值。
- **Reals 下的实根判定：** 用 `isolate` 数出 h 的实根个数 r。对根式形式的根用认证的 Ball 求值：虚部球不含 0 的根是非实根；恰好剩下 r 个时，它们就是实根。否则提高精度，最多重试 3 次，仍失败就判为 Unknown，保留该根并发出消息。`Root[h, k]` 是实根当且仅当 k ≤ r。

#### 8.7.3 根式方程
- **隔离后乘方**（根式个数 ≤ 2 时使用，这样步骤可读）：循环最多 4 次：选出 q 最大的根式 `r = b^(1/q)`，把 e 写成 `c·r + rest`（利用 `r^q = b` 约化 r 的幂），发出 `IsolateTerm` 步骤；构造 `c^q·b − (−rest)^q` 并展开，发出 `RaiseToPower(q)` 步骤。
- **一般情况**：由内到外为每个根式引入 `y_i`，关系为 `y_i^{q_i} − b_i(x)`，令 `P = e[r_i → y_i]`；按逆序依次做 `P = res_{y_i}(P, y_i^{q_i} − b_i)`；最后用 poly_uni 解 `P(x)`。
- **必须执行：** 每个候选解都代入**原始** e（使用主值分支），用 is_zero 检验；只有 Zero（或按 8.8.4 通过数值验证）才保留，其余丢弃并发出 `DropExtraneous` 步骤。
- **测试：** `Sqrt[x+2] − x` 平方后得 `x² − x − 2`，候选 {−1, 2}，丢弃 −1；`Sqrt[x] + Sqrt[x−5] − 5` 得 {9}。

#### 8.7.4 超越方程
**核统一**（返回代换，或失败）：
- (i) 所有核都是 `E^(a_i x + b_i)`，a_i 为有理数：令 `d = lcm(a_i 的分母)`，`y = E^(x/d)`，每个核改写为 `E^{b_i} y^{a_i d}`。
- (ii) 核为 `c_i^(a_i x + b_i)`（c_i 是整数），且都是同一个最小底 g 的幂（`c_i = g^{m_i}`，用完全幂检测）：同理令 `y = g^(x/d)`。
- (iii) 否则把 `c^u` 改写为 `E^(u Log c)`，以 `Log c` 为系数重试 (i)；仅当最终只有一族核时才算成功。
- (iv) 同一个 u 的 Sin 与 Cos 同时出现：令 `y = E^(I u)`，`Sin → (y − 1/y)/(2I)`，`Cos → (y + 1/y)/2`，反解时 `u = −I Log[y] + 2π C`，之后做一次 simplify。

**反函数表**（`k(u) = v`；C = C[n] ∈ Integers，n 取下一个未用的编号）：
| k | 分支 |
|---|---|
| Sin | `ArcSin[v] + 2πC`、`π − ArcSin[v] + 2πC` |
| Cos | `−ArcCos[v] + 2πC`、`ArcCos[v] + 2πC` |
| Tan / Cot | `ArcTan[v] + πC`（v ≠ ±I）/ `ArcCot[v] + πC` |
| Sec / Csc | `±ArcSec[v] + 2πC` / `ArcCsc[v] + 2πC`、`π − ArcCsc[v] + 2πC` |
| E^u | `Log[v] + 2πI C`（v = 0 时无解） |
| a^u（a 为常数） | `(Log[v] + 2πI C)/Log[a]` |
| Log[u] | 当 −π < Im v ≤ π 时为 `E^v`（数值检验；符号情形用 ConditionalExpression），否则无解 |
| u^n（n 为整数） | 二项式根 |
| u^(p/q) | 候选 `v^(q/p)`，再按主值分支验证 |
| Sinh / Cosh / Tanh | `ArcSinh[v] + 2πI C`、`Iπ − ArcSinh[v] + 2πI C` / `±ArcCosh[v] + 2πI C` / `ArcTanh[v] + πI C` |
| ArcSin / ArcCos / ArcTan | 当 −π/2 ≤ Re v ≤ π/2 时为 `Sin[v]` / 当 0 ≤ Re v ≤ π 时为 `Cos[v]` / 当 −π/2 < Re v < π/2 时为 `Tan[v]`（边界细节 **[不确定]**） |
| u·E^u | Complexes：`ProductLog[v]`，并发出消息 `Solve::ifun`（**[不确定]**）；Reals：v ≥ −1/E 时为 `ProductLog[v]`，−1/E < v < 0 时再加上 `ProductLog[−1, v]` |

**输出形式：** `x -> ConditionalExpression[expr, Element[C[1], Integers]]`；有多个常数时条件为 `And[Element[C[1], Integers], ...]`。之后对 expr 应用特殊值表（`ArcSin[1/2] → Pi/6`）。
**Reals 下的解族过滤：** 形如 `u0 + k·C` 的解族，若 k 为纯虚数且 u0 为实数，则令 C → 0 并去掉条件；若 u0 对所有整数 C 都不是实数（u0 为数值，虚部非零，且 k 为实数），整族丢弃；其他情况保留，条件加上 `Element[expr, Reals]`。
**`Solve::ifun` 消息：** 只要用了反函数（Complexes 下 ProductLog、或 inverse_functions 路径未给出完整解族时），就发出 `Solve::ifun: Inverse functions are being used by Solve, so some solutions may not be found; use Reduce for complete solution information.`

### 8.8 方程组、定义域与验证

#### 8.8.1 线性方程组
构造增广矩阵 `[A | b]`，环 D 取 Z（先清分母）或 `Z[参数]`（MPoly<IBig>）。用 Bareiss 无分数消元：
```
prev = 1
对每个 k: 选主元 = 第一个 ≥ k 的行中 M[r][col] 不恒为零者（按列从左到右）
  有参数时：记录 GenericAssumption(pivot ≠ 0)
  for i > k, j > col: M[i][j] = (M[k][col]*M[i][j] − M[i][col]*M[k][j]) / prev   （精确除法）
  prev = M[k][col]
```
A 部分全为 0 而 b 部分非零的行说明方程组矛盾 → {}。然后在 `Q(参数)` 上回代并 cancel：主元变量用自由（非主元）变量表示，排在后面的变量作为自由变量（Mathematica 惯例）。存在自由变量时发出 `Solve::svars`。**测试：** `det [[2,1],[1,3]] = 5`；3×3 方程组唯一解；欠定方程组；矛盾方程组；带参数 `{a x + y == 1, x - y == 0}` → `{{x -> 1/(1 + a), y -> 1/(1 + a)}}`，并有 GenericAssumption `1 + a ≠ 0`。

#### 8.8.2 多项式方程组
```
poly_system(F, vars):         # lex 序：vars[0] > ... > vars[n−1]
  G = 零维 ? fglm(groebner(F, grevlex), lex) : groebner(F, lex)
  if G == [1]: return {}
  if 非零维: U = 最大独立集（优先选靠后的变量），把 U 当作参数；
     对每个所需的 g，若它关于其 LM 变量的次数 ≤ 2，就解这个三角形的 lex 基得到其余变量；否则发出 Solve::svars 并返回部分结果
  g = G 中只含 vars[n−1] 的一元元素; 对 g 的每个不可约因子 f（factor_Z）:
     Gf = groebner(G ∪ {f}, lex)                      # 拆分出分支
     if Gf 具有形状 {x_i − h_i(x_n)}_{i<n} ∪ {f}: 对 f 的每个根 ρ: x_i = h_i(ρ)（先 mod f 约化，再化简）
     else: 引入 t = x_n + Σ c_i x_i，c 取自固定序列 (1, −1, 2, −2, ...)；加入 t − Σ...，按 t 为最小变量的 lex 序重算，再检查形状（最多试 5 组）
```
f 的根来自 `roots_irr`；如果是 Root 对象，x_i 就是关于该 Root 对象的多项式（Mathematica 也是这样返回的）。
- **Eliminate：** 取 lex 基中不含被消元变量的元素，输出为 `And[... == 0]` 并化简成 `lhs == rhs` 的形式。
- **NSolve：** 走相同路径，但求根用 Aberth，结果全部取数值；工作精度默认为机器精度，也可以通过 `WorkingPrecision` 指定。
- **FindRoot：** 带 Armijo 回溯的阻尼 Newton（能求符号 D 时用解析 Jacobian，否则用数值 Jacobian）；给出区间 `{x, a, b}` 时用 Brent 方法。

**非多项式方程组：** 用代换法。反复选取 `(分支数, 表达式大小)` 最小、且把其他变量视为参数时 univariate 能求解的那一对（方程, 变量），把解代入其余方程。每一步消去一个变量，所以最多 n 层；总分支数上限 64。仍然失败时，把核当作新变量并加入已知关系（同一参数的 Sin/Cos 加入 `s² + c² − 1`），然后调用 `poly_system`。

#### 8.8.3 定义域
- **Reals：** 按 8.7.2 与 8.7.4 处理。
- **Rationals：** 先在 Complexes 上求解，只保留 RootReduce 后次数为 1 的解。
- **Integers：**
  - 一元：只保留满足 `a | b` 的一次因子 `a x + b`。
  - 线性方程 `Σ a_i x_i = c`：若 `g = gcd(a) ∤ c` → {}；否则用扩展 gcd 列变换求出幺模矩阵 U，使 `A U = [H | 0]`（列 Hermite 标准形），在整数中解 `H z = c`，剩余的 z 设为 `C[1], C[2], ...`，最后 `x = U z`。输出 `x_i -> ConditionalExpression[..., Element[C[1], Integers]]`。
  - 线性方程组：对整个矩阵做相同的 HNF。

#### 8.8.4 验证
对每个候选 σ 和每个**原始**方程 e，计算 `z = is_zero(e[σ])`：
- **Zero** → 保留，标记 `Exact`。
- **NonZero** → 丢弃，发出 `DropExtraneous`。
- **Unknown：**
  - 来自 poly_uni 因式分解、线性、FGLM 形状回代路径的候选 → 按构造正确，保留为 `ByConstruction`。
  - 来自根式、超越路径的候选 → 用 128 与 512 bit 的球算术求值；两个球都含 0 且半径 < 2^−(prec−16) 时，保留为 `Numeric{digits}`；否则丢弃，并发出 `Solve::verify`。
  - 解族：在 C = 0、1、−1 处检验。
- **含参数时：** 用 SplitMix64 取 3 组随机的小有理数代入参数，跳过使排除条件或 GenericAssumption 为零的点。
- **排除条件：** 若某个排除表达式在候选处为 Zero，丢弃该候选；为 Unknown 时，用数值检验的反面判定。
- **绝不**把根式、超越路径得到的根作为 `Unverified` 输出。

### 8.9 不等式（Reduce-lite，一元有理不等式）
```
reduce_ineq(f rel 0, x): 用 together 得到 f = n/d（不约分）
  crit = sqf(n) ∪ sqf(d) 的实代数根，排序后去重（用精确的 RealAlg 比较去重）
  在每个间隙里取一个有理测试点（相邻隔离区间之间；两端取 ±(界+1)）；s = sign(f(测试点))（精确计算）
  若 rel 在符号 s 下成立，则包含该区间；若 rel 非严格、且 n(c) = 0、d(c) ≠ 0，则包含端点 c
  合并相邻片段；输出 Wolfram 形式 Inequality/Or（例如 -2 < x < 2、x < -2 || x >= 1）
  以及供 UI 使用的区间形式 Vec<Interval>
```
发出 `SignChart` 步骤。`Reduce[eq, x]` 对纯方程等价于 Solve 的结果转成 `x == a || x == b` 的形式；`Reduce[ineq && ineq2, x]` 对两个区间集合求交集。v1 **不支持**多元不等式（返回 Unevaluated，并发出 `Reduce::nsmet`）。
**测试：** `Reduce[x^2<4,x]` → `-2 < x < 2`；`Reduce[(x-1)/(x+2)>=0,x]` → `x < -2 || x >= 1`；`Reduce[x^2>=0,x,Reals]` → `True`；`Reduce[x^2<0,x,Reals]` → `False`；`Reduce[x^3-x>0,x]` → `-1 < x < 0 || x > 1`。

### 8.10 步骤（Steps）数据模型（om-solve/steps.rs）
```rust
pub struct Steps { pub root: Vec<Step> }
pub struct Step {
    pub id: String,                 // "S1"、"S1.2"：稳定的层级编号，LLM 讲解时引用 [S3]
    pub rule_id: &'static str,      // 稳定的规则 ID，例如 "quadratic_formula"、"drop_extraneous"（UI 模板和 LLM 都靠它，与 Rust 枚举的布局无关）
    pub kind: StepKind,
    pub before: Vec<Expr>, pub after: Vec<Expr>,
    pub level: Level,               // Major | Minor（UI 默认只展开 Major）
    pub children: Vec<Step>,
}
pub enum StepKind {
  Normalize, RecordExclusion { cond: Expr, reason: ExclReason }, GenericAssumption { cond: Expr },
  ClearDenominators { factor: Expr }, Expand, Factor { factors: Vec<(Expr, u32)> }, ZeroProduct,
  SquareFree { parts: Vec<(Expr, u32)> }, Substitute { new_var: Expr, def: Expr }, BackSubstitute { var: Expr, value: Expr },
  ApplyFormula { formula: Formula /* Linear|Quadratic|Cardano|Ferrari|Binomial|Palindromic */, bindings: Vec<(String, Expr)>, results: Vec<Expr> },
  Discriminant { value: Expr, sign: Option<Sign> }, IsolateTerm { term: Expr }, RaiseToPower { n: u32 },
  Resultant { var: Expr, result: Expr }, InvertFunction { func: Symbol, branches: Vec<Expr>, constants: Vec<Expr> },
  RowReduce { op: RowOp, matrix: Vec<Vec<Expr>> }, Groebner { order: MonoOrder, basis: Vec<Expr> },
  Eliminant { var: Expr, poly: Expr }, SplitComponent { factor: Expr }, RootObjects { poly: Expr, real_count: u32 },
  Verify { candidate: Vec<(Expr, Expr)>, outcome: Tri, residual: Option<Expr> }, DropExtraneous { candidate: Vec<(Expr, Expr)>, why: String },
  DomainFilter { domain: Domain, kept: usize, dropped: usize }, SignChart { points: Vec<Expr>, signs: Vec<Sign> },
  Branch { label: String }, Note { msg: Message },
}
pub trait StepSink { fn push(&mut self, s: Step); fn enter(&mut self, label: &str); fn exit(&mut self); fn enabled(&self) -> bool; }
pub struct NoSteps; pub struct StepRecorder { /* 维护层级栈，自动分配 id */ }
```
- 步骤**必须**由真正执行计算的那段代码发出，不能事后重构。关闭步骤记录时用 `NoSteps`（零开销：先检查 `enabled()` 再构造 Step）。
- kernel 把 Steps 转为 `StepsView`（JSON）：每步给出 `id, rule_id, level, title_key（i18n 键 = "step." + rule_id）, params（字符串化的 LaTeX 映射，例如 {"a":"1","b":"2","c":"-3","disc":"16"}）, before_latex[], after_latex[], children[]`。前端按 `title_key` 查找 i18n 模板，例如 `"step.quadratic_formula": "应用求根公式：a = {a}, b = {b}, c = {c}"`。
- **每个 rule_id 都必须有 zh-CN 和 en 两套模板**，CI 用测试检查模板是否齐全（`app/src/i18n/steps.test.ts` 读取 `crates/om-solve/rule_ids.txt`，该文件由 `cargo test -p om-solve export_rule_ids` 生成）。
- **步骤快照测试（insta）：** `x^2+2x-3==0`（Factor → ZeroProduct → 2 个 Linear）、`Sqrt[x+2]==x`（IsolateTerm → RaiseToPower → Quadratic → Verify → DropExtraneous）、`Sin[x]==1/2`（InvertFunction）、线性 3×3（RowReduce×n → BackSubstitute）、`(x-1)/(x+2)>=0`（SignChart）。

---

## 9. 求值器规格（om-eval）

### 9.1 核心结构

```rust
// crates/om-eval/src/lib.rs
pub struct Evaluator {
    pub defs: Definitions,                 // 用户定义：OwnValues / DownValues
    builtins: &'static BuiltinTable,       // 进程级只读表（OnceLock 初始化）
    pub messages: Messages,
    pub history: Vec<(Expr /*In*/, Expr /*Out*/)>,
    pub last_steps: Option<om_solve::Steps>, // 最近一次 Solve 类调用的步骤（kernel 取走）
    pub settings: EvalSettings,            // iteration_limit=4096, recursion_limit=1024, record_steps=true
    depth: u32,
}
pub struct Definitions {
    own: BTreeMap<Symbol, Expr>,                         // x = 5
    down: BTreeMap<Symbol, Vec<Rule>>,                   // f[x_] := x^2（按插入顺序；同 LHS 替换旧规则）
    attrs: BTreeMap<Symbol, Attributes>,                 // 用户设置的属性（v1 仅 Protected 检查用）
}
pub struct Rule { pub lhs: Expr, pub rhs: Expr, pub delayed: bool }
bitflags-like: pub struct Attributes(u16) // HOLD_ALL, HOLD_FIRST, HOLD_REST, LISTABLE, NUMERIC_FUNCTION, PROTECTED, ORDERLESS, FLAT, ONE_IDENTITY（后三个由 canon 实现，这里仅作标记）
// 不引入 bitflags crate：手写 const 与 contains()

pub type BuiltinFn = fn(&mut Evaluator, &[Expr], &Interrupt) -> Result<Option<Expr>, EvalError>;
pub struct BuiltinSpec { pub symbol: Symbol, pub f: BuiltinFn, pub attrs: Attributes, pub arity: Arity, pub doc: DocEntry }
pub enum Arity { Exactly(u8), Range(u8, u8), AtLeast(u8), Any }
#[derive(serde::Serialize, Clone)]
pub struct DocEntry { pub name: &'static str, pub modern: &'static str /*"solve(eqs, vars, domain: …)"*/,
    pub wolfram: &'static str /*"Solve[eqs, vars, dom]"*/, pub summary_zh: &'static str, pub summary_en: &'static str,
    pub examples: &'static [&'static str], pub category: &'static str /*"Solving" | "Algebra" | …*/ }

impl Evaluator {
    pub fn new() -> Self;
    pub fn evaluate(&mut self, e: &Expr, ctx: &Interrupt) -> Result<Expr, EvalError>;
    pub fn evaluate_statement(&mut self, e: &Expr, ctx: &Interrupt) -> Result<Expr, EvalError>; // 记录 history、Out[n]
    pub fn doc(sym: Symbol) -> Option<&'static DocEntry>; pub fn all_docs() -> impl Iterator<Item=&'static DocEntry>;
    pub fn fork_readonly(&self) -> Evaluator;   // 复制 defs，供 LLM 工具调用（禁止 Set）
}
#[derive(Debug, thiserror::Error)]
pub enum EvalError { #[error(transparent)] Abort(#[from] Abort), #[error("recursion limit {0} exceeded")] Recursion(u32), #[error("{0}")] Other(String) }
```

### 9.2 求值算法（每一步都要有对应测试）
`evaluate(e)`：
1. `ctx.tick()?`；`depth += 1`，超过 `recursion_limit` 返回 `EvalError::Recursion`（kernel 显示为消息 `$RecursionLimit::reclim`）。
2. **原子**：Number/String 原样返回；Symbol：若 `defs.own` 有值则返回 `evaluate(值)`，否则原样返回。
3. **Normal**：
   a. `h = evaluate(head)`。
   b. 取 `h` 的属性（内置表 ∪ 用户属性）。按 HoldAll/HoldFirst/HoldRest 决定哪些参数**不**求值；其余参数依次 `evaluate`。
   c. 展开参数中的 `Sequence[...]`（除非 HoldAll）。
   d. Listable 且某参数是 List：按 Mathematica 规则逐元素线程化（长度不一致 → 消息 `Thread::tdlen`，原样返回）。
   e. 用 `canon::rebuild(h, args)` 重建（对 Plus/Times/Power 调用规范构造器，其余直接构造）。
   f. 若 `h` 是符号且 `defs.down[h]` 非空：按顺序对每条规则调用 `pattern::match_rule(rule.lhs, expr)`，首个匹配 → `rhs` 代入绑定（`pattern::substitute`）→ `evaluate` 该结果并返回。
   g. 若 `h` 是内置符号：调用 `f(self, args, ctx)`。`Ok(Some(r))` 且 `r != expr` → 返回 `evaluate(r)`（循环计数 +1，超过 `iteration_limit` 发出 `$IterationLimit::itlim` 并返回 `Hold[r]`）；`Ok(None)` → 返回当前 expr。
   h. 否则原样返回。
4. **参数个数检查**：内置函数调用前检查 `arity`，不符合 → 消息 `F::argx`/`F::argrx`（文案与 Mathematica 相同，如 `Solve::argrx: Solve called with 4 arguments; between 1 and 3 arguments are expected.`），原样返回。

**模式匹配（`crates/om-eval/src/pattern.rs`）：** 支持 `Blank[]`、`Blank[h]`、`Pattern[name, p]`（同名绑定必须一致）、`Condition[p, test]`（test 代入绑定后求值为 `True`）、`BlankSequence`/`BlankNullSequence`（用回溯尝试所有长度）、字面量结构匹配。**不支持** Orderless 交换匹配、`Optional`、`Alternatives`、`PatternTest`（v1 遇到时发出 `Pattern::unsup` 警告并视为不匹配）。

### 9.3 内置函数清单（分里程碑实现；每个函数都要登记 DocEntry）
- **M4（基础）：** `Plus Times Power Subtract Divide Minus Sqrt Exp Log Abs Sign Re Im Conjugate Arg Floor Ceiling Round Mod Quotient GCD LCM Factorial Binomial FactorInteger PrimeQ Numerator Denominator N`、三角/反三角/双曲/反双曲（精确特殊值表见 8.1.6）、`List Part Length First Last Rest Append Table Range Map Apply Sum(有限) Product(有限)`、`Equal Unequal Less LessEqual Greater GreaterEqual And Or Not SameQ`、`Rule RuleDelayed ReplaceAll ReplaceRepeated Set SetDelayed Unset Clear CompoundExpression Hold HoldForm Out`、`Element`（仅保持）、`Function Slot`（纯函数应用）。
- **M10.1（代数）：** `Expand Factor Together Cancel Apart(仅 Q 上一元) Simplify FullSimplify(=Simplify 加更多变换) Collect Coefficient CoefficientList Exponent PolynomialQ PolynomialGCD PolynomialLCM PolynomialQuotient PolynomialRemainder Resultant Discriminant Variables D RootReduce ToRadicals`。
- **M10.2（求解）：** `Solve NSolve FindRoot Reduce Eliminate SolveValues NSolveValues Roots(=Solve 后转 Or 形式) Root ConditionalExpression`。
- **M11.5（绘图，kernel 侧）：** `Plot ContourPlot` 仅返回 HoldAll 的原样表达式，kernel 识别后采样（见 10.5）。

### 9.4 数值求值 `N`
- `N[e]` → 机器精度：递归把精确数转 f64（复数用 `(f64, f64)`），已知函数调用 `om_num::elem` 的 f64 实现；溢出或非有限值 → 自动切换到 BigFloat（精度 64 bit）重算。
- `N[e, d]`（d 为十进制位数）：工作精度 `bits = ceil(d·3.3219) + 32`；计算两次（`bits` 和 `2·bits`），若两次结果在 d 位上一致则返回，否则把精度加倍重试，最多 4 轮；最终结果四舍五入到 `ceil(d·3.3219)` bit。
- `om_num::elem`（任意精度初等函数，自研，放 `crates/om-num/src/elem/`）：`pi(bits)`（Machin 公式 + 缓存）、`exp`（参数约简 + Taylor）、`ln`（AGM 或牛顿迭代 exp）、`sin/cos`（约简到 [−π/4, π/4] + Taylor）、`atan`（约简 + Taylor）、其余由这些组合（`tan = sin/cos`，`asin(x) = atan(x/√(1−x²))`，双曲由 exp 组合）；复数版本由实数版本组合；`lambert_w0/w_m1`（Halley 迭代）。**测试：** 与 f64 标准库对比 1e-15；`pi(3400 bits)` 的前 1000 位十进制与常量字符串比对（常量放测试文件中）。
  > 如果 `dashu-float` 已提供 `exp`/`ln`/`sqrt`，直接调用并用上述测试验证。

---

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

---

## 11. LLM 层规格（om-llm）

### 11.1 设计：sans-IO
`om-llm` 核心不做任何 IO：它负责 **构造 HTTP 请求**（`HttpRequest { method, url, headers, body }`）、**增量解码响应字节流**（SSE / JSON）、**管理多轮对话与工具调用状态机**。传输由调用方完成：
- native（CLI、Tauri）：`om-llm` 的 `http` feature 提供 `drive_native(job, &reqwest::Client, on_event)` 异步驱动器。
- 浏览器（WASM）：TypeScript 用 `fetch` 获取流（`app/src/kernel/llmDriver.ts`），把文本块通过 `LlmHttpChunk` 回灌给 kernel。
这样同一套逻辑在所有平台上都一样，并且可以用录制的 fixture 完整测试。

```rust
// crates/om-llm/src/lib.rs
pub struct Profile { pub name: String, pub kind: ProviderKind, pub base_url: String, pub model: String,
    pub api_key: Option<String> /*运行时解析后的真实 key*/, pub temperature: f32, pub max_tokens: u32,
    pub supports_tools: bool, pub supports_json_mode: bool, pub timeout_ms: u64, pub extra_headers: BTreeMap<String,String> }
#[serde(rename_all = "snake_case")] pub enum ProviderKind { OpenaiChat, Anthropic, OpenaiFim, OllamaFim, MistralFim }
pub struct HttpRequest { pub method: String, pub url: String, pub headers: Vec<(String, String)>, pub body: String, pub stream: bool }
pub struct ChatMessage { pub role: Role /*system|user|assistant|tool*/, pub content: String,
    pub tool_calls: Vec<ToolCall>, pub tool_call_id: Option<String> }
pub struct ToolCall { pub id: String, pub name: String, pub arguments: String /*JSON 文本*/ }
pub struct ToolSpec { pub name: &'static str, pub description: &'static str, pub parameters: serde_json::Value /*JSON Schema*/ }

pub fn build_chat_request(p: &Profile, msgs: &[ChatMessage], tools: &[ToolSpec], json_mode: bool, target: Target) -> HttpRequest;
pub fn build_fim_request(p: &Profile, prefix: &str, suffix: &str) -> HttpRequest;
pub enum Target { Native, Browser }   // Browser 时 Anthropic 需加 anthropic-dangerous-direct-browser-access: true
pub enum StreamEvent { Text(String), ToolCallDelta { index: u32, id: Option<String>, name: Option<String>, args_fragment: String },
    Finish { reason: String }, Error(String) }
pub struct SseDecoder { buf: String }         // feed(&str) -> Vec<SseMessage{event: Option<String>, data: String}>；处理 \r\n、多行 data、注释行 ':'
pub fn decode_openai_chunk(data: &str) -> Vec<StreamEvent>;      // "[DONE]" -> Finish{reason:"done"}
pub fn decode_anthropic_event(event: &str, data: &str) -> Vec<StreamEvent>;
pub fn parse_fim_response(kind: ProviderKind, body: &str) -> Result<String, LlmError>;
```

### 11.2 各提供商的 HTTP 形状（照此实现，并用 fixture 测试）
- **OpenAI 兼容 Chat**（OpenAI、DeepSeek、通义千问 DashScope 兼容模式、小米 MiMo 兼容接口、Ollama `/v1`、LM Studio、vLLM、OpenRouter）：
  `POST {base_url}/chat/completions`；头 `Authorization: Bearer {key}`（key 为空时不加）、`Content-Type: application/json`；
  body `{"model","messages":[{"role","content"}…],"stream":true,"temperature","max_tokens","tools":[{"type":"function","function":{"name","description","parameters"}}],"response_format":{"type":"json_object"}}`（后两者按需）。
  流：SSE，每条 `data: {"choices":[{"index":0,"delta":{"content":"…","tool_calls":[{"index":0,"id":"call_1","type":"function","function":{"name":"solve","arguments":"{\"eq"}}]},"finish_reason":null}]}`；结束 `data: [DONE]`。`delta.reasoning_content`（DeepSeek 推理模型）忽略。工具参数按 `index` 累加拼接。助手带工具调用的消息回填为 `{"role":"assistant","content":null,"tool_calls":[…]}`，工具结果为 `{"role":"tool","tool_call_id":"call_1","content":"…"}`。
- **Anthropic Messages**：`POST {base_url}/v1/messages`（base 默认 `https://api.anthropic.com`）；头 `x-api-key: {key}`、`anthropic-version: 2023-06-01`、`content-type: application/json`；body `{"model","max_tokens","system":"…","messages":[…],"stream":true,"tools":[{"name","description","input_schema"}]}`（system 从消息列表中抽出）。SSE 事件：`message_start`、`content_block_start`（`content_block.type` 为 `text` 或 `tool_use{id,name}`）、`content_block_delta`（`delta.type` 为 `text_delta.text` 或 `input_json_delta.partial_json`）、`content_block_stop`、`message_delta`（`delta.stop_reason`：`end_turn`/`tool_use`/`max_tokens`）、`message_stop`、`ping`、`error`。工具结果回填为 user 消息 `{"role":"user","content":[{"type":"tool_result","tool_use_id","content"}]}`；助手工具调用消息为 `{"role":"assistant","content":[{"type":"tool_use","id","name","input":{…}}]}`。
- **FIM（幽灵补全）：**
  - `openai_fim`（DeepSeek beta 等）：`POST {base_url}/completions`，body `{"model","prompt":prefix,"suffix":suffix,"max_tokens","temperature":0,"stop":["\n"],"stream":false}` → `choices[0].text`。
  - `ollama_fim`：`POST {base_url}/api/generate`，body `{"model","prompt":prefix,"suffix":suffix,"stream":false,"options":{"temperature":0,"num_predict":max_tokens,"stop":["\n"]}}` → `response`。
  - `mistral_fim`：`POST {base_url}/v1/fim/completions`，body `{"model","prompt","suffix","max_tokens","temperature":0,"stop":["\n"]}` → `choices[0].message.content`。
  - 如果 `complete` 功能配置的是 chat 类 profile：用非流式 chat 请求加专用提示词（“只输出应插入光标处的文本，不要解释”），并把 `prefix⟨CURSOR⟩suffix` 放进用户消息。
- **预设（设置界面的一键填充）：** OpenAI `https://api.openai.com/v1`；DeepSeek `https://api.deepseek.com/v1`（FIM: `https://api.deepseek.com/beta`）；通义千问 `https://dashscope.aliyuncs.com/compatible-mode/v1`；OpenRouter `https://openrouter.ai/api/v1`；Ollama `http://localhost:11434/v1`（FIM: `http://localhost:11434`）；LM Studio `http://localhost:1234/v1`；Anthropic `https://api.anthropic.com`；小米 MiMo 及其他：“OpenAI 兼容（自定义）”，由用户填 base_url 和 model。**模型名只作为可编辑的默认值**，不要硬编码进逻辑。

### 11.3 功能与提示词（提示词文件：`crates/om-llm/prompts/*.md`，用 `include_str!` 编译进二进制，`{{name}}` 占位符替换）

1. **translate（自然语言 → 表达式）** `prompts/translate.md`：
   ```
   You translate math requests (Chinese or English) into ONE Wolfram Language expression for the OpenMath CAS.
   Rules:
   - Output ONLY a JSON object: {"wolfram": "<expression>", "explanation": "<one short sentence in {{lang}}>"}.
   - Use only these functions: {{function_list}}.
   - Equations use ==. Multiplication may be written with * or a space. Use Sqrt[], Pi, E, I, Log[] (natural log).
   - For "solve"/"求解"/"解方程" use Solve[eqs, vars] (or Solve[eqs, vars, Reals] when the user asks for real solutions / 实数解).
   - Systems use a list: Solve[{eq1, eq2}, {x, y}]. Inequalities use Reduce[ineq, x, Reals] unless the user says solve.
   - Numeric requests ("approximately", "数值解", "近似") use NSolve or N[...].
   - Symbols already defined in the notebook: {{defined_symbols}}. Reuse their names.
   - Never output anything except the JSON object.
   Examples:
   User: solve x squared plus 2x equals 3 → {"wolfram":"Solve[x^2 + 2*x == 3, x]","explanation":"..."}
   User: 求方程 x^3 - 2x + 1 = 0 的实数解 → {"wolfram":"Solve[x^3 - 2*x + 1 == 0, x, Reals]", ...}
   User: 解方程组 x+y=10, x-y=2 → {"wolfram":"Solve[{x + y == 10, x - y == 2}, {x, y}]", ...}
   User: sin x = 1/2 在 0 到 2π 之间的解 → {"wolfram":"Solve[Sin[x] == 1/2 && 0 <= x <= 2*Pi, x]", ...}
   User: 分解因式 x^4-1 → {"wolfram":"Factor[x^4 - 1]", ...}
   User: x^2 < 4 的解集 → {"wolfram":"Reduce[x^2 < 4, x, Reals]", ...}
   User: find numeric roots of x^5 - x + 1 → {"wolfram":"NSolve[x^5 - x + 1 == 0, x]", ...}
   User: 圆 x²+y²=25 和直线 y=x+1 的交点 → {"wolfram":"Solve[{x^2 + y^2 == 25, y == x + 1}, {x, y}]", ...}
   ```
   流程：`supports_json_mode` 时开启 JSON 模式；取回复中第一个 `{` 到最后一个 `}` 解析；`wolfram` 字段用 `om_parse::parse_expr(…, Wolfram)` 解析；失败时把诊断发回模型重试（“Your expression failed to parse: {{diagnostics}}. Return corrected JSON.”），**最多 2 次**；成功后生成 `Suggestion { wolfram: input_form(expr), modern: modern_form(expr), latex: latex(expr), explanation }`（modern/latex **由我们自己的格式化器从解析结果生成**，不信任模型给的文本）。前端展示建议卡片，**需要用户点击才插入或运行**。
2. **explain（步骤讲解）** `prompts/explain.md`：系统提示要求“只能使用提供的步骤与结果，不得引入新的数学结论；用 {{lang}}；引用步骤时写 [S{n}]；公式用 $…$”；用户消息是 `{"input": InputForm, "result": InputForm, "steps": StepsView JSON}`；流式输出，前端用 Markdown + KaTeX 渲染。`step_id` 非空时只讲解该步骤。
3. **complete（幽灵补全）**：见 11.2 FIM；前缀 = 当前 cell 光标前文本（send_context 时前面加上前 3 个 cell 的源码，用方言对应的注释包裹）；后处理：去掉首尾空白中的换行、截断到第一个换行；如果 `prefix + suggestion` 的解析诊断里含 `E0xx` 词法错误（非法字符）就丢弃；与本地补全第一项相同就丢弃。
4. **chat（助手面板，带工具）**：工具：
   - `evaluate {"code": string}`：用 Wolfram 方言解析 → 在 `fork_readonly()` 的求值器里执行（禁止 Set/SetDelayed，否则返回错误文本）→ 返回 `{"input_form","latex","messages"}`；每次调用超时 5 s。
   - `solve {"equations": [string], "variables": [string], "domain": "Complexes|Reals|Integers"}`：同上，但组装成 `Solve`，并返回 `{"solutions": InputForm, "steps_summary": [step titles]}`。
   - `propose_cell {"code": string, "dialect": "modern|wolfram"}`：不执行，只作为建议卡片发送给前端（`LlmSuggestion`），由用户确认插入。
   最多 6 轮工具调用；系统提示 `prompts/chat.md` 要求：“凡是数学计算必须调用工具，不得心算；最终答案必须引用工具结果”。
5. **fix（修复错误）** `prompts/fix.md`：输入源码、方言、诊断与消息，输出 JSON `{"wolfram": "...", "explanation": "..."}`，校验流程与 translate 相同。
6. **test profile**：发送 “Reply with the single word: pong”，max_tokens 8，显示延迟与首字节时间。

### 11.4 Job 状态机（kernel 用它把 LLM 功能接进协议）
```rust
pub struct Job { /* feature, profile, messages, pending tool calls, accumulated text, retries … */ }
pub enum JobStep {
    Http(HttpRequest),                      // 需要发起（下一轮）HTTP 请求
    RunTools(Vec<ToolCall>),                // 需要 kernel 执行工具，然后调用 job.tool_results()
    Done(JobResult),                        // JobResult::Text(String) | Suggestion(Suggestion) | Completion(String)
    Failed(LlmError),
}
impl Job {
    pub fn new(feature: Feature, profile: Profile, input: JobInput, target: Target) -> (Job, JobStep);
    pub fn on_bytes(&mut self, chunk: &str) -> Vec<StreamEvent>;       // 流式事件（文本增量转发为 Event::LlmDelta）
    pub fn on_http_end(&mut self, status: u16, error: Option<String>) -> JobStep;
    pub fn tool_results(&mut self, results: Vec<(String /*call id*/, String /*content*/)>) -> JobStep;
}
```
- HTTP 状态码非 2xx：读取 body 中的 `error.message`（OpenAI/Anthropic 格式）→ `Failed`，前端显示 “{profile}：{status} {message}”。401/403 时提示检查 API Key。
- 取消：`LlmCancel` → kernel 删除 job；native 驱动通过 `tokio_util::sync::CancellationToken` 中止请求；浏览器端通过 `AbortController`。

### 11.5 测试
- `crates/om-llm/tests/fixtures/`：`openai_text.sse`、`openai_tools.sse`（两个工具调用，参数分多块到达）、`anthropic_text.sse`、`anthropic_tools.sse`、`deepseek_fim.json`、`ollama_fim.json`、`mistral_fim.json`、`openai_error_401.json`。
- 解码测试：fixture 按**随机切块**（1–7 字节，用 SplitMix64 固定种子）喂给 SseDecoder，结果必须与整块喂入一致。
- 请求构造：insta JSON 快照（Authorization 头里的 key 用 `sk-test`）。
- Job 状态机：用 fixture 驱动完整的“两轮工具调用 → 最终文本”流程。
- native 驱动：`wiremock` 起本地服务器返回 fixture，验证端到端（仅 `--features http`）。
- 可选真实测试：`OM_LIVE_LLM=1 DEEPSEEK_API_KEY=… cargo test -p om-llm --features http live_ -- --ignored`，默认不跑。

---

## 12. UI/UX 规格

### 12.1 设计原则（“比 Mathematica 更现代”的具体含义）
1. **所见即所得的输入：** 输入框下方实时显示公式排版（KaTeX）；错误就地波浪线标出，并提供一键修复（Fix）。
2. **结果是可交互的对象，而不是一段文本：** 解卡片、数轴、图像、步骤都能点、能复制、能再利用。
3. **响应式笔记本：** 修改一个定义，所有依赖它的 cell 自动重算（marimo 式），不会出现“隐藏状态”。
4. **自然语言与形式语言无缝切换：** 可以随时用中文或英文描述，AI 给出候选表达式，**由用户确认**后才执行。
5. **键盘优先：** 所有操作都能通过 ⌘K 命令面板完成。
6. **可解释：** 每个解都能展开推导步骤，也能让 AI 用自然语言讲解（讲解只能引用真实步骤）。

### 12.2 布局
```
┌───────────────────────────────────────────────────────────────────────────┐
│ ◧ OpenMath · 未命名笔记本 ▾      [方言: 自动 ▾] [▶ 全部运行] [⌘K] [AI ◐] [⚙] │ ← TopBar
├──────────────────────────────────────────────┬────────────────────────────┤
│ [1] ● 现代   solve(x^2 + 2x = 3, x)          │ 步骤 | 助手 | 变量 | 文档   │ ← 右侧面板(可折叠, 宽 380px)
│              ─────────────────────────────── │                            │
│              solve(x² + 2x = 3, x)  ← KaTeX预览│  S1 因式分解               │
│   ┌ 2 个解 · 已验证 ✓ ─────────────────────┐  │     x²+2x−3 = (x−1)(x+3)   │
│   │  x = −3      x = 1        [步骤][图像] │  │  S2 零因子法则 …           │
│   └────────────────────────────────────────┘  │  [✨ AI 讲解]              │
│   ┌ 图像 ────────────────────────────────────┐ │                            │
│   │  y = x²+2x   y = 3   ● (−3,3)  ● (1,3)   │ │                            │
│   └──────────────────────────────────────────┘ │                            │
│ [2] ✎ 问   求圆 x²+y²=25 与直线 y=x+1 的交点   │                            │
│   ┌ ✨ 建议 ────────────────────────────────┐  │                            │
│   │ Solve[{x^2+y^2==25, y==x+1}, {x,y}]      │  │                            │
│   │ [插入为代码] [运行] [编辑]               │  │                            │
│ [+ 数学] [+ 文本] [+ 问 AI]                    │                            │
└──────────────────────────────────────────────┴────────────────────────────┘
```
- 宽度小于 900px 时，右侧面板变为底部抽屉；小于 480px（手机）时单列显示，TopBar 按钮收进菜单。
- 主题：浅色与深色（跟随系统，可手动切换），全部颜色用 CSS 变量定义（`app/src/styles/theme.css`）。

### 12.3 Cell
- **类型：** `Math`（代码）、`Text`（Markdown，支持 `$…$` 与 `$$…$$`，用 KaTeX 渲染）、`Ask`（自然语言提问，产生建议卡片）。
- **左侧栏：** 执行序号 `[n]`、状态点（灰 = 未运行，蓝 = 运行中（脉动动画），绿 = 完成，红 = 错误，黄 = 过期 Stale）、方言标签（点击可切换并固定）。
- **编辑器（CodeMirror 6）：**
  - 语法高亮：由 kernel 的 `Preview.tokens` 驱动（`TokenClass` → CSS 类），避免在 TS 里重写一遍词法分析；本地先用一个极简的正则高亮兜底，防止闪烁。
  - 诊断：`@codemirror/lint`，数据来自 `Preview.diagnostics`；有 `fix` 时在 lint 提示里显示“快速修复”按钮。
  - 自动补全：`@codemirror/autocomplete` 的 `CompletionSource` 调用 kernel 的 `Complete`；显示签名与中文摘要。
  - 幽灵文本：见 12.7。
  - 括号自动配对、`^` 后输入数字时预览区即时显示上标。
  - 快捷输入：`\alpha` + Tab → `α`，`\pi` → `π`，`<=` 显示为 `≤` 的连字（只是显示，源码不变）。
- **实时预览：** 编辑器下方一行 KaTeX（`Preview.latex`，80ms 去抖），解析失败时显示灰色的“…”。
- **动作条（`Preview.actions`）：** 例如输入的是裸方程 `x^2 = 4`，显示按钮 [求解 x] [绘图] [赋值 let x = …]，点击后把源码替换为对应的动作源码并运行。
- **选区浮动工具条：** 在编辑器中选中一段子表达式时，浮出 [因式分解] [展开] [化简] [求解] [绘图]，点击后在下方新建 cell，内容为 `factor(<选中文本>)` 并运行。
- **快捷键：** `Shift+Enter` 运行并跳到下一个 cell（没有下一个就新建一个）；`⌘/Ctrl+Enter` 运行但留在当前 cell；`Alt+Enter` 运行并在下方插入新 cell；`⌘.` 中断；`⌘K` 命令面板；`⌘/` 在当前 Math cell 中切换“问 AI”模式；`Tab` 接受幽灵文本；`Esc` 关闭幽灵文本或补全；`⌘S` 保存；`⌘⇧↑/↓` 移动 cell；`⌘⇧D` 删除 cell（带撤销提示条）。

### 12.4 输出渲染
- **Expr：** KaTeX display 模式。悬停时右上角出现工具条：[复制 LaTeX] [复制 Wolfram] [复制现代语法] [数值 ≈]（切换显示 `N[…, 20]`）[插入到新 cell]。无障碍：`aria-label` 为 InputForm。
- **Solutions（解卡片 `SolutionCards.tsx`）：**
  - 标题：“{n} 个解 · 已验证 ✓”（全部为 Exact 或 ByConstruction）/“数值验证”/“无解”/“对所有值成立”。
  - 每个解是一个“chip”：`x = −3`（KaTeX）；悬停显示 20 位数值；右下角的小徽章表示验证方式（✓ 精确、≈ 数值）。
  - 重根显示为 `x = 1 (二重)`，**不**重复显示多个 chip（Wolfram 格式的复制仍然会重复规则）。
  - 带 `C[1]` 的解族显示为 `x = π/6 + 2πk, k ∈ ℤ`（把 `C[n]` 渲染为 k、m、n…）。
  - 含 Root 对象时显示 `Root 1 ≈ −1.1673`，并提供 [根式形式] 切换（如果 kernel 给出了 ToRadicals 结果）。
  - 按钮：[步骤]（打开右侧面板的步骤标签页并定位到该 cell）、[图像]（切换内联图像）、[代入…]（生成 `expr /. sol` 模板 cell）、[复制全部]。
- **Region（不等式）：** 条件的 KaTeX + **数轴组件**（`NumberLine.tsx`，SVG）：实心或空心端点、着色区间、±∞ 箭头，刻度自动选取。
- **消息：** 在输出下方折叠显示，警告为黄色、错误为红色，格式为 `Solve::svars — 中文说明`（消息中文翻译表放在 `app/src/i18n/messages.ts`，没有翻译时显示英文原文）。
- **计时：** 输出右下角显示 `12 ms`，超过 1 s 时高亮显示。

### 12.5 步骤面板（`StepsPanel.tsx`）
- 垂直时间线；每个节点显示编号 `S1`、标题（i18n 模板 + 参数，参数用 KaTeX 渲染）、前后表达式（`before → after`）、可展开的子步骤；Minor 步骤默认折叠（可以勾选“显示全部细节”）。
- 每个步骤有一个“为什么？”按钮 → `LlmExplain{step_id}`，流式讲解显示在该步骤下方。
- 顶部有“AI 讲解全部”按钮 → `LlmExplain{step_id: None}`，讲解文本中的 `[S3]` 渲染成可点击的锚点，点击后滚动到对应步骤并闪烁高亮。
- LLM 未配置时，讲解按钮显示为禁用状态，并提示“在设置中配置 AI”。

### 12.6 绘图（`PlotView.tsx`，自写 SVG，不引入绘图库）
- 组成：坐标轴、网格（刻度用 “nice numbers” 算法：1/2/5 × 10^k）、曲线（`<path>`，每段一个 `M…L…`）、解点（圆点 + 坐标标签）、着色区间（半透明矩形）、图例、十字准线（显示鼠标处坐标）。
- 交互：拖拽平移、滚轮缩放（以鼠标位置为中心）、双击复位；视窗变化后以 150ms 去抖发送 `SamplePlot` 重新采样。
- 参数滑块：`PlotRequest.params` 非空时，在图像下方为每个参数显示一个滑块（范围取 `param_ranges`，步长为范围的 1/200）；拖动时节流到约 30fps 发送 `SamplePlot`；kernel 返回新的曲线和解点。
- 颜色：曲线使用 CSS 变量 `--plot-1` … `--plot-6`（浅色与深色两套），解点使用 `--accent`。
- 尺寸：宽度 100%，高度 320px，保持响应式。
- **测试：** `scale.ts` 的 nice-ticks 单元测试；PlotView 组件在给定 PlotData 时渲染出正确数量的 path 与点（vitest + testing-library）。

### 12.7 幽灵文本补全（`editor/ghostText.ts`）
```ts
// 状态：StateField<{ pos: number; text: string } | null>
// 1) 触发：文档改变后 350ms 空闲，且光标位于行尾，且当前行去掉空白后长度 ≥ 3，且没有打开补全弹窗，且设置 llm.complete 非空
// 2) 请求：kernel.request({type:"llm_complete", request_id, prefix, suffix, dialect})；每次新触发都要 LlmCancel 上一次的请求
// 3) 显示：Decoration.widget({ widget: new GhostWidget(text), side: 1 }) 放在 pos 处，CSS 类 .cm-ghost（opacity 0.45，斜体）
// 4) 接受：keymap Tab（优先级 Prec.highest，仅在有建议时生效，否则交还给默认处理）→ 在 pos 插入 text，然后清空
//    部分接受：⌘→ 只接受到下一个单词边界
// 5) 清除：Esc；光标离开 pos；文档改变且新输入的字符不是建议文本的前缀（如果是前缀，就把建议文本裁掉已输入的部分，继续显示）
```
**本地补全优先：** kernel 的 `Complete` 给出的确定性补全（函数名、括号闭合）总是立即显示；LLM 幽灵文本只作为补充。
**测试（vitest，mock kernel）：** 触发条件、前缀裁剪、Tab 接受、Esc 清除、快速连续输入时只发出最后一个请求。

### 12.8 AI 相关界面
- **Ask cell / “问 AI”模式：** 输入自然语言后回车 → `LlmTranslate` → 显示建议卡片：LaTeX 预览 + 源码（使用当前的首选方言，由 kernel 格式化器生成）+ 一句话说明 + [插入为代码]（替换当前 cell 或在下方新建 Math cell）[运行] [编辑]。请求进行中显示骨架屏动画，可以取消。
- **助手面板（`AssistantPanel.tsx`）：** 对话界面，Markdown + KaTeX 渲染；工具调用显示为可折叠的卡片（“🔧 solve(x^2+2x==3, x) → {{x→−3},{x→1}}”）；`propose_cell` 的建议显示为带 [插入] 按钮的卡片。输入框支持 `@cell3` 引用某个 cell 的内容（发送时替换为该 cell 的源码和输出的 InputForm）。
- **错误修复：** 输出为错误时，显示 [✨ 让 AI 修复] 按钮 → `LlmFixError` → 显示修改前后的 diff 与 [应用] 按钮。
- **AI 状态指示（TopBar 的 AI ◐）：** 灰色 = 未配置；绿色 = 就绪；旋转 = 请求中；红色 = 最近一次请求出错（悬停显示错误信息）。点击打开 AI 设置。
- **隐私提示：** 第一次使用任何 AI 功能时弹窗说明：“将发送以下内容到 {base_url}：当前输入{、笔记本上下文}”，并提供“不再提示”选项。

### 12.9 设置对话框（`settings/SettingsDialog.tsx`）
- **常规：** 语言、主题、默认方言、常量模式（math/strict）、响应式执行开关、自动重算依赖、默认显示步骤、自动绘图、求值超时。
- **AI 模型：** profile 列表（增、删、改、复制）；表单字段：名称、类型（下拉：OpenAI 兼容对话 / Anthropic / OpenAI 兼容 FIM / Ollama FIM / Mistral FIM）、Base URL、模型名、API Key（密码框；桌面版存入系统钥匙串，Web 版可以选择“在此浏览器中记住”（存 localStorage，附带风险提示），否则只保存在当前会话）、temperature、max_tokens、是否支持工具、是否支持 JSON 模式、额外请求头（键值对编辑器）；[测试连接] 按钮（显示延迟或错误信息）；预设按钮（见 11.2）一键填入 base_url 与示例模型名。
- **功能映射：** 翻译 / 讲解 / 对话 / 补全 / 修复 分别选择使用哪个 profile（下拉框，含“关闭”选项）。
- **Web 版的 CORS 提示：** 选择 Ollama 时显示“需要设置环境变量 `OLLAMA_ORIGINS=*` 后再启动 Ollama”；选择 Anthropic 时说明浏览器直连会自动添加专用请求头；其他提供商若返回 CORS 错误，提示“该提供商不支持浏览器直连，请使用桌面版或 CLI”。

### 12.10 前端工程结构
```
app/src/
├── main.tsx  App.tsx
├── kernel/
│   ├── generated/            # ts-rs 自动生成（不要手改）
│   ├── client.ts             # interface KernelClient { request(req): Promise<Response>; onEvent(cb): Unsubscribe; interrupt(): Promise<void>; kind: "wasm"|"tauri" }
│   ├── wasmClient.ts         # Web Worker 实现；请求用自增 id 配对
│   ├── worker.ts             # 加载 om-wasm，把 postMessage 转成 kernel.request(json)
│   ├── tauriClient.ts        # invoke("kernel_request") + Channel 接收事件
│   ├── llmDriver.ts          # 浏览器端 fetch 驱动（见下文）
│   └── index.ts              # createKernelClient()：检测 window.__TAURI_INTERNALS__ 决定使用哪个实现
├── state/  notebookStore.ts  settingsStore.ts  uiStore.ts   # zustand
├── components/
│   ├── TopBar.tsx  Notebook.tsx  Cell.tsx  AskCell.tsx  TextCell.tsx  CommandPalette.tsx  VariablesPanel.tsx  DocsPanel.tsx
│   ├── editor/  MathEditor.tsx  highlight.ts  completion.ts  ghostText.ts  lint.ts  selectionToolbar.tsx  keymap.ts
│   ├── output/  OutputView.tsx  ExprView.tsx  SolutionCards.tsx  NumberLine.tsx  Messages.tsx  Katex.tsx
│   ├── plot/    PlotView.tsx  scale.ts  sliders.tsx
│   ├── steps/   StepsPanel.tsx  StepNode.tsx
│   ├── assistant/ AssistantPanel.tsx  ToolCallCard.tsx  SuggestionCard.tsx
│   └── settings/ SettingsDialog.tsx  ProfileForm.tsx  presets.ts
├── i18n/  index.ts  zh-CN.ts  en.ts  steps.zh-CN.ts  steps.en.ts  messages.ts
├── styles/ theme.css  katex-overrides.css
└── test/  setup.ts  mockKernel.ts
```
- **WASM 中断：** 浏览器里同步运行的 wasm 无法被打断。`WasmClient.interrupt()` 的做法是 `worker.terminate()` → 新建 worker → `LoadNotebook`（用前端 store 中的源码恢复）→ 按拓扑顺序只重新执行**定义了符号**的 cell（`defines` 非空）→ 其他 cell 标记为 Stale。UI 显示提示条“计算已中断，内核已重启”。另外，每次 Evaluate 都附带 `eval_timeout_ms`（kernel 内的 Interrupt 通过注入的 `Clock` 检查截止时间），所以大多数失控计算会自行结束。
- **`llmDriver.ts`（浏览器）：**
  ```ts
  async function drive(kernel, requestId, http: HttpRequest, signal: AbortSignal) {
    let next: HttpRequest | null = http;
    while (next) {
      const res = await fetch(next.url, { method: next.method, headers: Object.fromEntries(next.headers), body: next.body, signal });
      const reader = res.body!.pipeThrough(new TextDecoderStream()).getReader();
      for (;;) { const { value, done } = await reader.read(); if (done) break;
                 await kernel.request({ type: "llm_http_chunk", request_id: requestId, chunk: value }); }
      const r = await kernel.request({ type: "llm_http_end", request_id: requestId, status: res.status, error: null });
      next = r.type === "llm_started" ? r.http ?? null : null;   // kernel 执行完工具后可能要求下一轮请求
    }
  }
  // 网络异常（包括 CORS）→ llm_http_end { status: 0, error: String(e) }
  ```
  流式文本通过 kernel 发出的 `Event::LlmDelta` 送到 UI（worker 在 `request` 的返回值中附带事件列表，由 WasmClient 分发）。

### 12.11 桌面端（app/src-tauri，crate 名 `om-desktop`）
- `tauri.conf.json`：`productName: "OpenMath"`，`identifier: "org.openmath.OpenMath"`；`build.frontendDist: "../dist"`，`devUrl: "http://localhost:5173"`，`beforeDevCommand: "npm run dev"`，`beforeBuildCommand: "npm run build"`；窗口 1280×820，最小 720×480。
- CSP：`default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self' 'unsafe-inline'; font-src 'self' data:; img-src 'self' data: blob:; connect-src 'self' ipc: http://ipc.localhost`（桌面版的 LLM 请求由 Rust 发出，前端不需要访问外网）。
- Capabilities（`capabilities/default.json`）：`core:default`、`dialog:allow-open`、`dialog:allow-save`、`fs:allow-read-text-file`、`fs:allow-write-text-file`（配合 dialog 选择的路径使用）、`opener:allow-open-url`（仅用于打开文档链接）。
- Rust 侧：
  ```rust
  struct KernelHost { tx: std::sync::mpsc::Sender<(String /*envelope json*/, oneshot)>, interrupt: Arc<AtomicBool>, events: Mutex<Option<Channel<String>>> }
  #[tauri::command] async fn kernel_request(host: State<'_, KernelHost>, envelope: String) -> Result<String, String>;
  #[tauri::command] fn kernel_subscribe(host: State<'_, KernelHost>, channel: Channel<String>);
  #[tauri::command] fn kernel_interrupt(host: State<'_, KernelHost>);   // 直接设置 AtomicBool，不经过队列
  #[tauri::command] fn secret_set(profile: String, key: String) -> Result<(), String>;   // keyring
  #[tauri::command] fn secret_delete(profile: String) -> Result<(), String>;
  ```
  Kernel 运行在一个专用线程上（`Session` 只属于该线程），请求通过 mpsc 队列进入。LLM 任务：当 kernel 返回 `LlmStarted{http: Some(..)}` 时，host 在 `tauri::async_runtime` 上调用 `om_llm::drive_native`，每收到一块数据就经队列把 `LlmHttpChunk` 送回 kernel 线程，并把 kernel 产生的事件推送到 Channel。**这样 native 与浏览器使用完全相同的 Job 状态机。**
- 菜单：文件（新建/打开/保存/另存为/导出 Markdown/导出 LaTeX）、编辑、视图（主题、面板开关）、帮助（文档、关于）。
- 打包：`npx tauri build` 在 macOS 上生成 `.app` 与 `.dmg`（不做签名，签名留到发布阶段）。

### 12.12 CLI（om-cli，二进制名 `om`）
- `om`：进入 REPL。`om -e "<code>"`：求值后打印结果并退出（退出码：0 = 成功，1 = 求值出错，2 = 解析出错）。`om run file.omnb|file.om`：按顺序执行所有 cell，并打印输出。`om config path|show|edit`。`om llm test <profile>`。`om --dialect wolfram`、`om --json`（输出 CellOutput 的 JSON，便于脚本调用）。
- REPL 提示符：`In[n]:= `（Wolfram 方言）或 `❯ `（现代方言）；输出：`Out[n]= …`，使用 `unicode_form`；解集输出为 `x = −3  │  x = 1   (2 个解，已验证)`。
- reedline 配置：`Completer`（调用 kernel 的 `Complete`）、`Highlighter`（token 分类 → nu-ansi-term 颜色）、`Validator`（括号未闭合时进入多行输入）、`Hinter`（默认为历史提示；`[cli] ai_hints = true` 时由后台线程请求 FIM，只返回当前缓冲区已有缓存的结果，**绝不阻塞按键**）。历史保存在 `ProjectDirs.data_dir()/history.txt`。
- 元命令：`:help`、`:steps`（以树形打印上一次的步骤）、`:explain`（流式输出 AI 讲解）、`? <自然语言>` 或 `:ask <自然语言>`（翻译后显示建议，提示 `[Y]运行 / [n]取消 / [e]编辑`，选 e 时把建议填入输入缓冲区）、`:latex`（打印上一个输出的 LaTeX）、`:dialect modern|wolfram|auto`、`:plot`（打印 ASCII 草图，可选）、`:config`、`:clear`（清除所有定义）、`:quit`。
- Ctrl-C：求值期间按下时（通过 `ctrlc` crate 3.5 的信号处理器）设置中断标志，输出 `$Aborted`；在输入阶段按下时清空当前行（reedline 默认行为）。

---

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

---

## 14. Solve 验收语料（权威列表，`tests/corpus/solve.toml`）

> 标记 `=` 表示按第 8.6.1 节顺序做精确 InputForm 字符串比较（不确定顺序的项按多重集比较，标注在“备注”列）；标记 `~` 表示做数值验证（代入原方程，200 位精度下残差 < 1e−100；同时核对解的个数、重数与每个数值）。**标 [不确定] 的输出形式以数值验证为准**，不要求与 Wolfram 字符串完全一致。

| # | 标记 | 输入（Wolfram 语法） | 期望结果 | 备注 |
|---|---|---|---|---|
| 1 | = | `Solve[2x+3==7,x]` | `{{x -> 2}}` | |
| 2 | = | `Solve[a x+b==0,x]` | `{{x -> -(b/a)}}` | 含 GenericAssumption `a≠0` |
| 3 | = | `Solve[x^2-5x+6==0,x]` | `{{x -> 2}, {x -> 3}}` | |
| 4 | = | `Solve[x^2==2,x]` | `{{x -> -Sqrt[2]}, {x -> Sqrt[2]}}` | |
| 5 | = | `Solve[x^2-2x-1==0,x]` | `{{x -> 1-Sqrt[2]}, {x -> 1+Sqrt[2]}}` | |
| 6 | = | `Solve[x^2+1==0,x]` | `{{x -> -I}, {x -> I}}` | |
| 7 | = | `Solve[x^2+2x+5==0,x]` | `{{x -> -1-2I}, {x -> -1+2I}}` | |
| 8 | ~ | `Solve[x^2+x+1==0,x]` | 两个复根 `(-1∓I Sqrt[3])/2` | [不确定] 输出形式 |
| 9 | = | `Solve[(x-1)^2==0,x]` | `{{x -> 1}, {x -> 1}}` | 重根 |
| 10 | = | `Solve[x^3-6x^2+11x-6==0,x]` | `{{x -> 1}, {x -> 2}, {x -> 3}}` | |
| 11 | ~ | `Solve[x^3-2x-4==0,x]` | `{-1-I, -1+I, 2}` | 顺序 [不确定] |
| 12 | ~ | `Solve[x^3-3x+1==0,x,Cubics->True]` | 三实根 ≈ −1.879385, 0.347296, 1.532089 | 默认（Cubics 关闭）返回 Root |
| 13 | ~ | `Solve[x^3+x+1==0,x,Cubics->True]` | 实根 ≈ −0.6823278，复根对 ≈ 0.3411639±1.1615414 I | |
| 14 | ~ | `Solve[x^3==2,x]` | `2^(1/3)`, `-(-1)^(1/3) 2^(1/3)`, `(-1)^(2/3) 2^(1/3)` | 二项式路径，= 比较 |
| 15 | = | `Solve[x^4-5x^2+4==0,x]` | `{{x->-2},{x->-1},{x->1},{x->2}}` | 双二次 |
| 16 | ~ | `Solve[x^4==2,x]` | `±2^(1/4)`, `±I 2^(1/4)` | |
| 17 | ~ | `Solve[x^5-x+1==0,x]` | `Root[1-#1+#1^5&,k]`, k=1..5；Root 1 ≈ −1.167304 | |
| 18 | ~ | `Solve[x^5==2,x]` | 5 个根 `(-1)^(2k/5) 2^(1/5)` | |
| 19 | = | `Solve[(x^2-1)/(x-1)==0,x]` | `{{x -> -1}}` | x=1 被排除 |
| 20 | = | `Solve[x/(x-1)==1/(x-1),x]` | `{}` | |
| 21 | ~ | `Solve[1/x+1/(x+1)==1,x]` | `x = (1±Sqrt[5])/2` | |
| 22 | = | `Solve[Sqrt[x+2]==x,x]` | `{{x -> 2}}` | x=−1 被舍弃 |
| 23 | = | `Solve[Sqrt[x]+Sqrt[x-5]==5,x]` | `{{x -> 9}}` | |
| 24 | = | `Solve[Sqrt[x]==-1,x]` | `{}` | |
| 25 | = | `Solve[Abs[x]==2,x,Reals]` | `{{x -> -2}, {x -> 2}}` | |
| 26 | ~ | `Solve[Exp[x]==2,x]` | `x = Log[2] + 2πI·k` | ConditionalExpression，C[1]∈ℤ |
| 27 | = | `Solve[Exp[x]==2,x,Reals]` | `{{x -> Log[2]}}` | |
| 28 | = | `Solve[Log[x]==2,x]` | `{{x -> E^2}}` | |
| 29 | = | `Solve[E^(2x)-3E^x+2==0,x,Reals]` | `{{x -> 0}, {x -> Log[2]}}` | 核统一 |
| 30 | ~ | `Solve[2^x==8,x,Reals]` | `{{x -> 3}}` | |
| 31 | = | `Solve[Sin[x]==1/2,x]` | 两族：`Pi/6+2πC[1]`、`5Pi/6+2πC[1]` | |
| 32 | = | `Solve[Sin[x]==2,x,Reals]` | `{}` | |
| 33 | = | `Solve[Tan[x]==1,x]` | 一族：`Pi/4+πC[1]` | |
| 34 | ~ | `Solve[x E^x==1,x]` | `x = ProductLog[1]` | |
| 35 | = | `Solve[a x^2+b x+c==0,x]` | 求根公式两根 | GenericAssumption `a≠0` |
| 36 | = | `Solve[{x+y+z==6,2x-y+z==3,x+2y-z==2},{x,y,z}]` | `{{x->1,y->2,z->3}}` | |
| 37 | ~ | `Solve[{x+y+z==1,x-y==0},{x,y,z}]` | `x=y=(1-z)/2`，z 自由 | Solve::svars |
| 38a | = | `Solve[{x+y==1,x+y==2},{x,y}]` | `{}` | 矛盾 |
| 38b | = | `Solve[x==x+1,x]` | `{}` | |
| 38c | = | `Solve[x^2+1==0,x,Reals]` | `{}` | |
| 38d | = | `Solve[x^2==2,x,Rationals]` | `{}` | |
| 39 | = | `Solve[{x^2+y^2==1,y==x},{x,y}]` | `{{x->-1/Sqrt[2],y->-1/Sqrt[2]}, {x->1/Sqrt[2],y->1/Sqrt[2]}}` | |
| 40 | ~ | `Solve[{x^2+y^2==5,x y==2},{x,y}]` | `{(-2,-1),(-1,-2),(1,2),(2,1)}` | 多重集 |
| 41 | ~ | `Solve[{x+y+z==0,x y+y z+z x==0,x y z==1},{x,y,z}]` | 6 个解，是 `{1,(-1)^(2/3),-(-1)^(1/3)}` 的排列 | cyclic-3 |
| 42 | = | `Reduce[x^2<4,x]` | `-2 < x < 2` | |
| 43 | = | `Reduce[(x-1)/(x+2)>=0,x]` | `x<-2 \|\| x>=1` | |
| 44 | = | `Solve[x^2==4,x,Integers]` | `{{x->-2},{x->2}}` | |
| 45 | ~ | `Solve[2x+3y==1,{x,y},Integers]` | 一族 ConditionalExpression 解，代入验证 2x+3y=1 恒成立 | 参数化 [不确定] |
| 46 | = | `Solve[x==x,x]` | `{{}}` | |
| 47 | = | `Solve[x^2==1&&x!=1,x]` | `{{x -> -1}}` | |
| 48 | = | `Eliminate[{x==y+1,y==2z},y]` | `x==1+2z` | |
| 49 | = | `Solve[{x^2+y^2==25,y==x+1},{x,y}]` | 圆与直线交点，两组实数解（数值 ≈(3,4),(-4,-3)） | 用于 UI 绘图演示 |
| 50 | = | `Solve[Cos[x]==-1,x]` | 一族：`Pi+2πC[1]` | |

前 40 条覆盖第 6 节 P0/P1 全部场景，41–50 补充 cyclic 系统、Eliminate、绘图演示、Cos 反函数。**M9.16 要求 1–35 全绿才能进入 M10；36–50 在 M9 结束前全绿。**

---

## 15. 端到端验证

完成 M13 后，按以下步骤验证整条链路（不得依赖“看起来能跑”，必须逐项确认）：

1. `cargo test --workspace` 全绿；`cargo clippy --workspace --all-targets -- -D warnings` 无警告；`cargo deny check` 无违规。
2. `cargo build -p om-kernel --no-default-features --target wasm32-unknown-unknown` 成功（验证 wasm 兼容性，3 节约束）。
3. `cd app && npm run typecheck && npm run lint && npm run test && npm run build` 全部成功；`git diff --exit-code app/src/kernel/generated` 无差异（ts-rs 生成物已提交）。
4. `om -e 'solve(x^2 - 5x + 6 = 0, x)'` 输出两个根；`om -e 'Solve[x^2-5x+6==0,x]'`（Wolfram 语法）输出相同结果。
5. Run skill 启动桌面应用（`npm run tauri dev` 或按 `run` 技能约定），手动执行：新建 Math cell 输入 `solve(x^2+2x=3, x)`，确认（a）实时 LaTeX 预览随输入更新，（b）运行后出现两个解卡片且标记“已验证”，（c）出现内联图像并高亮两个解点，（d）点击“步骤”能看到因式分解→零因子法则的推导。
6. 修改一个被其他 cell 引用的变量定义，确认依赖 cell 自动重算（10.3 节场景）。
7. 在设置中配置一个可用的 LLM profile（优先用本地 Ollama 避免消耗真实 API 额度：`ollama pull qwen2.5:7b` 之类），测试连接成功；在 Ask cell 输入自然语言方程描述，确认生成建议卡片且可插入执行；测试幽灵文本补全出现并可用 Tab 接受；测试步骤讲解能流式输出且不产生新的数学结论之外的内容。
8. 浏览器打开 `app/dist`（`npm run preview`）用同一组操作走一遍第 5–7 步，确认 WASM 路径与 Tauri 路径行为一致（LLM 走 `llmDriver.ts` 的浏览器 fetch 路径）。
9. `om run tests/corpus/solve.toml`（如果 CLI 支持批量跑语料）或者写一个 `tests/corpus_test.rs` 集成测试，逐条跑第 14 节语料并断言，产出通过率报告。
10. 用 superpowers:requesting-code-review 走一次代码评审，重点检查 8 节算法实现与规格是否一致、是否有遗留的 `unwrap`/`unsafe`、Steps 是否覆盖全部路径。

---

## 16. 风险与降级策略

| 风险 | 影响 | 降级策略 |
|---|---|---|
| Zassenhaus 重组在特定构造多项式上指数级慢（8.2d） | M6 延期 | 已有硬上限 + `PossiblyReducible` 标记；v1 接受该局限，写入 `docs/solve.md` |
| `dashu-float 0.6.1` 的初等函数覆盖不全 | M1.3/M1.4 工作量增加 | 计划已预留：缺失的函数在 om-num 自实现（8 节前言已给出方法） |
| Cardano/Ferrari 与 Mathematica 的默认输出形式不完全一致（[不确定] 标记的多处） | 验收语料按 `~` 数值验证，不卡字符串匹配 | 已在第 14 节标注，差异写入 `docs/solve.md`“已知差异”一节 |
| Gröbner 基在退化/高次系统上性能不足 | M7/M9.10 变慢 | v1 只保证第 14 节语料规模（≤3 变量、次数 ≤3）；更大系统提供 `Reduce::nsmet` 式降级而非卡死（受 Interrupt 预算保护） |
| Tauri 2 /浏览器 CORS 限制导致部分 LLM 提供商在 Web 版不可用 | Web 版 LLM 功能受限 | 12.9 节已设计 CORS 提示；桌面版和 CLI 不受此限制，作为主推荐路径 |
| WASM 下无法真正中断计算（10.7 节最后一段） | 用户体验（卡顿） | 已设计 worker 重启方案 + eval_timeout_ms 兜底 |
| 任务量大（约 62 人日），单人执行周期长 | 进度 | 使用 subagent-driven-development 并行非依赖任务（例如 M3 与 M4.1-4.4 可与 M5/M6 并行）；M9/M6 是关键路径，优先保证正确性 |

---

## 17. 范围之外（未来工作，不在本计划内实现）

- MCP 服务器（把 solve/simplify/evaluate 暴露为工具，供 Claude Code 等外部代理调用）——架构已预留（`om-kernel` 的 JSON 协议可直接包一层 `rmcp`）。
- Jupyter 内核（`jupyter-protocol`/`runtimelib`）——JSON 协议设计已考虑到这个方向。
- 多元不等式与更完整的 `Reduce`（v1 只做一元有理不等式，8.9 节末已声明）。
- van Hoeij/LLL 因式分解算法（替代 v1 的 Zassenhaus 上限截断）。
- 模 GCD（Brown/Zippel）替代 GCDHEU，提升大规模多项式性能。
- `Optional`/`Alternatives`/`PatternTest`/Orderless 模式匹配（9.2 节已声明不支持）。
- `egg`/`egglog` e-graph 化简引擎替代当前的启发式 simplify（8.5 节）。
- 命令面板、变量面板、文档面板的具体交互细节（12.2 节已列入右侧标签页，但细节设计留到 M13 执行时按需扩展，不阻塞主线）。
- 移动端 / Tauri mobile 打包。
- 协作编辑（多人实时笔记本）。
