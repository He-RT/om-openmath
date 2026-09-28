<!-- Extracted from docs/plan/PLAN.md sections [4, 5]. PLAN.md is authoritative; keep in sync. -->

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
      排序、Interrupt、消息)                       无平方分解、Z[x] 因式分解、结式、实根隔离、Aberth、
          ▲         ▲                              Gröbner/FGLM、alg: 实/复代数数与 RootReduce)
   om-parse   om-format                                   ▼
          ▼                                             om-num (Integer/Rational/Real/Complex、Ball 球算术、
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
