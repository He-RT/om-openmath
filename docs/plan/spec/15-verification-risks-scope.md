<!-- Extracted from docs/plan/PLAN.md sections [15, 16, 17]. PLAN.md is authoritative; keep in sync. -->

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
