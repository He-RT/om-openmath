<!-- 由 scripts/function_docs.py 生成；编辑 functions.toml 后重新生成。 -->

# 全景功能目录

当前发行：`0.1.0-pre-alpha.2`；下一版目标：`0.1.0-pre-alpha.3`。

此目录是设计与真实实现的对照。规范名称不等于当前已接受的调用名；每项分别列当前签名和目标签名。规划条目不会自动进入运行时或可执行补全。别名不作为新增数学能力计数。

稳定 ID 使用独立的 `fn_000001` 等身份，不随名称/别名变化；[身份账本](function-identities.json)记录初始归属。副作用标签是设计预留，不是运行时授权；`unclassified` 不授予执行能力。runtime 区域提供当前回调的类型、保持角色、字面/上下文默认值；解析、补全、Hover 与 GetFunctionCatalog/GetCapabilities 共用生成元数据。目录的目标参数仍是设计文字，Notebook事务与Agent授权尚未实现。[当前可执行接口](executable.md)按实际回调列明。

| 状态 | 条目数 | 含义 |
|---|---:|---|
| 已实现 | 193 | 在所列支持范围内已有实现 |
| 部分支持 | 11 | 已有真实入口，但数学范围或目标接口未完整交付 |
| 下一版规划 | 44 | 锁定下一版，当前不可用 |
| 后续规划 | 581 | 进入全景目录，下一版不承诺实现 |

当前登记 `209` 个真实注册名；条目按语义归并，与注册名数量不同。

| 分类 | 条目数 | 已实现 / 部分 / 下一版 / 后续 |
|---|---:|---|
| [基础数值与初等函数](basics.md) | 133 | 80 / 2 / 2 / 49 |
| [表达式与多项式](algebra.md) | 45 | 20 / 0 / 0 / 25 |
| [求解与条件](solving.md) | 19 | 2 / 5 / 0 / 12 |
| [微积分与变换](calculus.md) | 33 | 1 / 0 / 10 / 22 |
| [向量、矩阵与张量](linear.md) | 37 | 23 / 0 / 0 / 14 |
| [ODE、优化与拟合](analysis.md) | 19 | 0 / 0 / 5 / 14 |
| [统计、概率与随机](statistics.md) | 49 | 17 / 0 / 0 / 32 |
| [列表、记录与表格](data.md) | 73 | 27 / 3 / 1 / 42 |
| [语言、求值与模式](language.md) | 60 | 15 / 0 / 0 / 45 |
| [函数与数据绘图](plots.md) | 30 | 0 / 1 / 6 / 23 |
| [图元、场景与交互](scene.md) | 87 | 0 / 0 / 20 / 67 |
| [单位、时间与知识库](units.md) | 50 | 4 / 0 / 0 / 46 |
| [文件、导出与系统接口](io.md) | 55 | 0 / 0 / 0 / 55 |
| [图与网络](graphs.md) | 34 | 0 / 0 / 0 / 34 |
| [图像、音频与信号](media.md) | 39 | 0 / 0 / 0 / 39 |
| [机器学习与大模型](learning.md) | 17 | 0 / 0 / 0 / 17 |
| [性能、并行与运行环境](runtime.md) | 43 | 4 / 0 / 0 / 39 |
| [Notebook Agent 与事务预留](agent.md) | 6 | 0 / 0 / 0 / 6 |

## 产品与平台功能

| 功能族 | 当前状态 | 当前/目标范围 | 证据 |
|---|---|---|---|
| 现有现代/Wolfram双语法 | 已实现 | 当前 let/等式/1起始列表/关键字/where 保留；管道、fn、记录、区间和@尚未实现。 | [docs/language.md](../../docs/language.md)、[crates/om-parse/tests/modern.rs](../../crates/om-parse/tests/modern.rs) |
| 现代组合与统一接口 | 下一版规划 | .3 管道/匿名函数/记录/区间/字段/矩阵@；mode/output等只在目标实现后可用。 | [docs/design/modern-language.md](../../docs/design/modern-language.md) |
| 笔记本与文件生命周期 | 已实现 | 已有源码.omnb v1、响应式重算、恢复/中断、Markdown/LaTeX导出；新格式或自动执行不由本轮引入。 | [docs/ios-acceptance.md](../../docs/ios-acceptance.md)、[crates/om-kernel/tests/reactive.rs](../../crates/om-kernel/tests/reactive.rs)、[crates/om-kernel/tests/recovery.rs](../../crates/om-kernel/tests/recovery.rs) |
| 编辑器与本地帮助 | 已实现 | 已有真实Preview/Complete/Hover/诊断/Greek/IME与ghost；本轮目录尚未接入运行时补全。 | [crates/om-kernel/tests/editor.rs](../../crates/om-kernel/tests/editor.rs)、[ios/OpenMath/MathEditor.swift](../../ios/OpenMath/MathEditor.swift) |
| AI 与凭据 | 已实现 | 已有 Ask/讲解/对话/只读工具/修复/模型配置/探测及各宿主凭据隔离；规划语法不写入当前提示词。 | [docs/llm.md](../../docs/llm.md)、[crates/om-kernel/tests/llm.rs](../../crates/om-kernel/tests/llm.rs)、[ios/OpenMathTests/TransportTests.swift](../../ios/OpenMathTests/TransportTests.swift) |
| 当前二维绘图 | 已实现 | 实函数/零等值轮廓、求解点/区间、参数滑块、平移缩放复位；数据来自内核。 | [crates/om-kernel/tests/plot_sampling.rs](../../crates/om-kernel/tests/plot_sampling.rs)、[ios/OpenMath/PlotView.swift](../../ios/OpenMath/PlotView.swift) |
| 三维显示与场景 | 下一版规划 | .3 桌面/Web WebGL2；移动端明确未适配；CLI OBJ导出。无WebGL2明确提示而非假图。 | [docs/plan/NEXT_RELEASE.md](../../docs/plan/NEXT_RELEASE.md) |
| 数据/图形导出 | 下一版规划 | .3 表格CSV/JSON、二维SVG/PNG、三维OBJ；暂不含完整CAD/glTF/视频。 | [docs/plan/NEXT_RELEASE.md](../../docs/plan/NEXT_RELEASE.md) |
| 分发与平台门禁 | 已实现 | .2九类附件已发布；.3实现完成才升级版本/发新Release；不启动本机模拟器。 | [docs/ios-acceptance.md](../../docs/ios-acceptance.md)、[docs/releasing.md](../../docs/releasing.md) |
| 无障碍人工组合验收 | 部分支持 | 已有源码语义、静态路径与大字体证据；用户暂缓的旁白/浮动键盘/真机窄窗口仍未验证。 | [docs/ios-acceptance.md](../../docs/ios-acceptance.md) |
| 专业计算与系统集成 | 后续规划 | PDE/完整符号ODE/Risch/CAD/稀疏张量/媒体/ML/网络/云/外部语言分阶段，不属于.3交付承诺。 | [docs/plan/NEXT_RELEASE.md](../../docs/plan/NEXT_RELEASE.md) |
| Notebook Agent 与框架适配契约 | 后续规划 | 稳定ID、可验证参数/副作用/能力版本、原子文档修改、幂等、撤销、取消及会话分离；只写预留，不安装Pi/Rig、不开放工具。 | [docs/plan/NEXT_RELEASE.md](../../docs/plan/NEXT_RELEASE.md)、[docs/design/modern-language.md](../../docs/design/modern-language.md) |

## 常量、定义域与选项符号

符号识别不等于函数或求解域已实现；下列当前含义独立列明，不计入真实回调数量。

| 名称 | 兼容名 | 含义与边界 |
|---|---|---|
| `pi` | Pi, π | 精确圆周率，现代已识别。 |
| `e` | E | 默认数学常量；strict模式不把小写e映射成E。 |
| `i` | I | 默认虚数单位；strict模式不把小写i映射成I。 |
| `inf` | Infinity, infinity, ∞ | 无穷符号；不是普通有限数。 |
| `true` | True | 布尔真。 |
| `false` | False | 布尔假。 |
| `null` | Null | 空/被抑制输出相关符号，非零。 |
| `reals` | Reals | 现有求解实数域，具体函数边界见目录。 |
| `complexes` | Complexes | 现有Solve默认域，不保证所有复数方程都可解。 |
| `integers` | Integers | 现有受限整数求解域。 |
| `rationals` | Rationals | 现有有理数求解域。 |
| `Algebraics` | Algebraics | 名称被识别，不代表现有现代domain适配器或所有求解支持此域。 |
| `Primes` | Primes | 定义域名字被识别，不是已实现的一般素数约束求解。 |
| `Booleans` | Booleans | 名字被识别，不是完整SAT工具声明。 |
| `automatic` | Automatic | 个别接口中的默认策略，须看该函数选项。 |
| `all` | All | 个别接口的全量选择，不是所有函数都支持。 |
| `none` | None | 个别接口的无/关闭选项，不是统一空值类型。 |

## 维护与验证

```sh
python3 scripts/function_docs.py --check
CARGO_PROFILE_TEST_OPT_LEVEL=2 cargo test -p om-eval --test function_catalog --locked
```

目录校验检查状态、稳定身份、副作用预留、必填字段、证据和生成文件一致性；Rust 契约比较真实注册表与当前示例。测试引用表示已有覆盖入口，不代替本轮执行日志，也不意味着规划接口已实现。后续类型/必填/枚举/范围及实际默认值接入时另升级描述版本，不能将文字说明直接传给模型。
