<!-- Extracted from docs/plan/PLAN.md sections [0]. PLAN.md is authoritative; keep in sync. -->

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
