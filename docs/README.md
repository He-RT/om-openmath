# 文档导航

项目默认使用中文；代码标识符、协议字段和许可证保留原文。

| 需要做什么 | 文档 |
|---|---|
| 下载、安装、卸载或校验 | [安装指南](install.md) |
| 写公式、使用笔记本或终端 | [输入语言](language.md) |
| 查询全部功能族、当前支持和未来接口 | [全景功能目录](reference/README.md) |
| 在计算环境查询函数、选项和能力 | [帮助与能力查询](design/function-queries.md) |
| 理解现代语法与统一接口设计 | [现代语言设计](design/modern-language.md) |
| 查看下一版科研功能与Agent预留 | [下一版计划](plan/NEXT_RELEASE.md) |
| 确定 `.4` 的版本、完整范围、同候选验收和发布条件 | [版本与门禁规格](plan/PRE_ALPHA_4.md)、[验收入口](acceptance/pre-alpha.4/README.md)、[32项机器账本](acceptance/pre-alpha.4/gates.toml) |
| 设计 Mac 右侧助手与可操作笔记本的 Pi Agent | [Notebook Agent 草案](design/notebook-agent.md) |
| 将 Mac UI 完全原生化并与 Agent 整合 | [原生 Mac 客户端方案](design/macos-native-ui.md) |
| 实施原生 Mac 宿主并统一 Swift Rust Pi 状态 | [宿主与状态契约](design/macos-host-state.md)、[状态 Schema](design/macos-host-state.schema.json) |
| 保存和恢复 Mac 笔记本、Agent 会话、上下文与附件 | [存储、事务与恢复](design/macos-storage-recovery.md)、[存储 Schema](design/macos-storage.schema.json) |
| 查看 Mac UX、动效与苹果原生组件率 | [UX 与动效](design/macos-ux.md)、[组件清单](design/macos-ui-inventory.json) |
| 细化 Mac 原生源码编辑、公式/Markdown/结果与二维/Metal 展示 | [编辑与渲染契约](design/macos-editor-rendering.md)、[机器 Schema](design/macos-editor-rendering.schema.json)、[交互草案](design/prototypes/mac-editor-rendering.html) |
| 设计原生 Mac 自包含安装、首次启动、签名公证和更新回退 | [安装与发行契约](design/macos-installation.md)、[分发 Schema](design/macos-distribution.schema.json)、[启动/更新草案](design/prototypes/mac-installation.html) |
| 查看助手输入框、供应商模型选择与媒体附件设计 | [输入框与媒体规格](design/agent-composer.md)、[交互草案](design/prototypes/mac-agent-composer.html) |
| 设计供应商连接、多模型、能力、探测和媒体请求路由 | [模型与媒体服务](design/model-media.md)、[机器 Schema](design/model-media.schema.json) |
| 查看供应商、模型、功能映射与媒体处理的原生界面 | [模型媒体 UI/UX](design/macos-model-media-ux.md)、[设置交互草案](design/prototypes/mac-model-settings.html) |
| 管理 Agent 提示词层、实际上下文、任务记忆与会话压缩 | [上下文和提示词管理设计](design/agent-context.md) |
| 教 Agent 使用 OpenMath 并验收实际操作能力 | [操作提示词草案](design/prompts/notebook-operation.md) |
| 查看 Agent 工具的参数、返回、错误和副作用 | [工具契约](design/agent-tools.md)、[完整 JSON Schema](design/agent-tools.schema.json) |
| 查看特殊函数、概率与随机流的实际边界 | [特殊函数与概率实现](design/special-probability.md) |
| 解析 CSV/JSON 纯数据 | [数据格式契约](design/data-formats.md) |
| 导出真实二维 SVG/PNG、数据 CSV/JSON 或用 CLI 保存 | [导出契约与用法](design/artifact-export.md) |
| 三维曲面/参数/隐式采样、相机和 OBJ | [三维基础契约](design/scene3d.md) |
| 有限图元、样式、变换与二维/三维组合 | [场景图契约](design/scene-graph.md) |
| 运行地月 L2 与完整西瓜 | [源码与笔记本](examples/README.md)、[真实验收](acceptance/r36b/README.md) |
| 使用单位并检查量纲 | [单位计算](design/units.md) |
| 查看微积分与数值分析的实际实现 | [微积分实现](design/calculus.md) |
| 了解求解支持范围和验证含义 | [求解指南](solve.md) |
| 接入 DeepSeek 或其他模型 | [AI 配置](llm.md) |
| 编译、测试和贡献代码 | [开发指南](development.md) |
| 开发内核客户端 | [JSON 协议](protocol.md) |
| 构建与发布 GitHub Release | [发布流程](releasing.md) |
| 了解此次版本和验收证据 | [发行说明](release-0.1.0-pre-alpha.3.md)、[验收记录](pre-alpha.md) |
| iPhone / iPad 使用与构建 | [原生移动端指南](ios.md)、[移动验收](ios-acceptance.md) |
| 接续开发 | [计划](plan/PLAN.md)、[进度](plan/PROGRESS.md)、[裁决](plan/DEVIATIONS.md)、[问题](plan/QUESTIONS.md) |

[英文入口](../README.en.md)保留英文说明；历史设计条目保留，供核对实施过程。

- [ODE、连续插值与采样（.3）](design/ode-interpolation.md)

- [局部优化与全局保证边界（.3）](design/optimization.md)

- [真实拟合、可调用模型与数值诊断（.3）](design/fitting.md)

- [结构化结果、科学诊断与只读分页](design/result-pages.md) · [R3.5a真实验收截图](acceptance/r35a/README.md)
