<!-- Extracted from docs/plan/PLAN.md sections [1, 2]. PLAN.md is authoritative; keep in sync. -->

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
