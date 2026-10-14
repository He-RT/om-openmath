# pre-alpha.4 实施进度

[完整实施计划](PRE_ALPHA_4.md) · [机器门禁](../acceptance/pre-alpha.4/gates.toml) · [裁决](DEVIATIONS.md) · [问题](QUESTIONS.md)

2026-10-15。目标`0.1.0-pre-alpha.4`，当前运行/公开版本`.3`；开发`dev`，不合main、不改旧标签、本机不启动iOS模拟器。本文只记录结果，规格及完成标准唯一见主计划；计划编写本身不计实施完成。

**当前状态：** 34任务完成（全部R4.0与R4.1、R4.2.01–02、07及09–10、R4.3.01–03及05–06）；其余64任务尚未完成；32最终运行/发行gate全部not_run；candidate未分配，ready_to_publish=false，released_verified=false。设计/HTML/Schema通过不填运行时pass。

**下一任务：** 用户选择保留系统输入法设置、先做其他任务；R4.2.03真实候选窗口仍未验收。R4.3.05三个只读工具已完成实际输出引用与持久事务历史，R4.3.06原子修改已完成实际四轮/耐久effect回执/冲突与未知状态；继续R4.3.07执行、结果检查和独立scratch，复用真实Kernel/Result owner及当前Agent作用域；R4.3.04物理锁屏验收仍未完成，原依赖满足后再接非求值编辑分析/长笔记本/Markdown。存储维护、备份、新代次、配额和GC已通过实际验证；日末备份触发/数据管理UI仍在R4.2.18接线，完整Pi/provider/上下文/Agent界面仍继续R4.3–4.4，不能将存储Done当产品完成。保持同candidate完整门禁，R4.5.07在R4.5.05冻结前完成。

## 已知外部前置

- 原生宿主、事件恢复和首代/source物理存储已运行；DocCommitPort已通过实际跨语言验证；完整工作台、checkpoint及Agent尚未接通，运行源码版本仍`.3`。
- 上轮只读预检没有Developer ID Application材料；执行阶段复核并按R4.0.11准备接口，真实公开签名门禁不能降级。
- 用户已于2026-10-13提供4Router端点及两个模型的测试授权；仓库外私有文件保存凭据，目录/短文本均200。仅开发smoke，不代表新Registry/Pi/工具/媒体或完整live gate；不自动读取旧Key/环境/profile。当前live gate未运行。

## 任务账本

同次实现提交更新任务状态、主计划复选框与下面记录；SHA在后续记账避免自引用。`[x]`只表示该任务Done/Tests满足，不自动改变最终candidate门禁。blocked应写具体前置并继续独立任务。

| 任务 | 目标 | 状态 | 验证/证据 | 提交 |
|---|---|---|---|---|
| [x] [R4.0.01](PRE_ALPHA_4.md#task-r4.0.01) | 建立基线、任务账本和恢复入口 | completed | [基线](../acceptance/pre-alpha.4/baseline/index.json) | e876c16 |
| [x] [R4.0.02](PRE_ALPHA_4.md#task-r4.0.02) | 依赖与许可闭包核对 | completed | dependencies.json / clean npm ci + audit | 75b4e5c |
| [x] [R4.0.03](PRE_ALPHA_4.md#task-r4.0.03) | 原生Mac工程和数学链接骨架 | completed | Release build / native AX /真实C ABI读回 | 75b4e5c |
| [x] [R4.0.04](PRE_ALPHA_4.md#task-r4.0.04) | 冻结框架无关DTO与新MacABI契约 | completed | 91 DTO / Rust+Swift / CI37880165376 | 21c9254 |
| [x] [R4.0.05](PRE_ALPHA_4.md#task-r4.0.05) | 安全host-service所有者与调度骨架 | completed | 10调度+9契约 / 真实CAS / 独立取消 / Clippy | daecc3f |
| [x] [R4.0.06](PRE_ALPHA_4.md#task-r4.0.06) | 新CABI句柄、缓冲与直接取消 | completed | 9 Rust ABI / 真实 Swift 调用 / Release 链接 | c9bb366 |
| [x] [R4.0.07](PRE_ALPHA_4.md#task-r4.0.07) | Swift客户端、事件泵和权威投影 | completed | 纯reducer / 真实ABI丢帧恢复 / 原生AX运行与停止 | ece80a1 |
| [x] [R4.0.08](PRE_ALPHA_4.md#task-r4.0.08) | 共享Modern自动续行与根因诊断 | completed | 91 DTO / Rust+Swift / CI37880165376 | 21c9254 |
| [x] [R4.0.09](PRE_ALPHA_4.md#task-r4.0.09) | 原语言和数学基线持续兼容 | completed | 91 DTO / Rust+Swift / CI37880165376 | 21c9254 |
| [x] [R4.0.10](PRE_ALPHA_4.md#task-r4.0.10) | fixture与真实故障注入基础设施 | completed | 自有媒体封存/真实解码、回环HTTP、SIGKILL/ENOSPC、实际budget/stream codec | 627c978 |
| [x] [R4.0.11](PRE_ALPHA_4.md#task-r4.0.11) | 发行前置、签名和现场环境预检 | completed | [环境](../acceptance/pre-alpha.4/baseline/environment.json) / verify-env.sh | e876c16 |
| [x] [R4.1.01](PRE_ALPHA_4.md#task-r4.1.01) | SQLite单写者与新通道初始化 | completed | SQLite3.54/WAL/FULL / 实际F_FULLFSYNC / 根锁 / SIGKILL/ENOSPC | ddbeb88 |
| [x] [R4.1.02](PRE_ALPHA_4.md#task-r4.1.02) | 权威源码、事务和幂等表 | completed | Rust实际计划→Swift同库事务→Rust实际回执 / 幂等/回滚/失联/UTF8 | c310565 |
| [x] [R4.1.03](PRE_ALPHA_4.md#task-r4.1.03) | 不可变Blob和资源引用发布 | completed | 分块实际字节/同库refs/临时pin/恢复读取/六SIGKILL点 | 26078d0 |
| [x] [R4.1.04](PRE_ALPHA_4.md#task-r4.1.04) | 文档操作与非执行失效计划 | completed | 真实parser/owners清理/无cascade / 批量操作 / title/Text epoch / 设置同库 | c72ce17 |
| [x] [R4.1.05](PRE_ALPHA_4.md#task-r4.1.05) | 冻结Preview与新格身份分配 | completed | 10preview+3ref / 原计划→SQLite→原authority实际回执 / 期限/scope完整性 | 22fc708 |
| [x] [R4.1.06](PRE_ALPHA_4.md#task-r4.1.06) | DocCommitPort、编辑屏障与未知提交 | completed | [实际提交](../acceptance/pre-alpha.4/development/r41/commit.json) / NSTextView+SQLite / 63Rust / Release | 21b01e7 |
| [x] [R4.1.07](PRE_ALPHA_4.md#task-r4.1.07) | 统一源码撤销和后续编辑冲突 | completed | [历史/文本/回执](../acceptance/pre-alpha.4/development/r41/history-and-groups.json) / 200+pin / actual UTF/IME / 80Rust / Release | 3853967 |
| [x] [R4.1.08](PRE_ALPHA_4.md#task-r4.1.08) | 完整可写计算状态与无损codec | completed | [整个Session/原证据](../acceptance/pre-alpha.4/development/r41/session-checkpoint.json) / 723相关Rust＋原53＋data-only/角色/预算 | 55d11b8；de0ec9e及前置 |
| [x] [R4.1.09](PRE_ALPHA_4.md#task-r4.1.09) | 主KernelWorker、候选接纳和直接取消 | completed | [实际主运行](../acceptance/pre-alpha.4/development/r41/kernel-runtime.json) / CABI+Swift+SQLite / 310相关Rust+实际IO/长CAS/close | 5469091；3ce0320、b309baf |
| [x] [R4.1.10](PRE_ALPHA_4.md#task-r4.1.10) | 不可变结果库和只读检查通道 | completed | [实际原生结果](../acceptance/pre-alpha.4/development/r41/result-runtime.json) / CABI＋worker＋Swift / 324Rust＋actual receipts/late reply | 6d53f17；f04474f；后续补验 |
| [x] [R4.1.11](PRE_ALPHA_4.md#task-r4.1.11) | NSDocument打开保存与冻结快照 | completed | [实际文件/同库回执/本轮Release面板闭环](../acceptance/pre-alpha.4/development/r41/file-saves-completion.json) / 6 SIGKILL+原件冲突 / N+1/IME/身份/取消/重开 / Mac CI38122270416 success | 25df8d8；f04474f、c4eab5a |
| [x] [R4.1.12](PRE_ALPHA_4.md#task-r4.1.12) | 会话outbox、草稿与启动恢复 | completed | [indexed长会话/原生恢复入口](../acceptance/pre-alpha.4/development/r41/indexed-session-recovery.json) / 4374事件>16MiB/37+37分页/13+5+3+6SIGKILL/真实UI | a23ef8a、5b96deb、642cfc9；本批待补SHA |
| [x] [R4.1.13](PRE_ALPHA_4.md#task-r4.1.13) | 引用租约、备份、quota与GC | completed | [实际备份/升级/配额/GC](../acceptance/pre-alpha.4/development/r41/storage-maintenance.json) / 17SIGKILL/原schema/113Rust/15原生回归/Release | 27ede3f |
| [x] [R4.2.01](PRE_ALPHA_4.md#task-r4.2.01) | 原生工作台窗口、分栏和投影 | completed | [原生导航/严格索引](../acceptance/pre-alpha.4/development/r42/native-source-and-navigation.json) / 实窗/42scalar/滚动/真实5和6/原生回归/Release | 6a59193 |
| [x] [R4.2.02](PRE_ALPHA_4.md#task-r4.2.02) | NSTextView和严格SourceIndexMap | completed | [原生导航/严格索引](../acceptance/pre-alpha.4/development/r42/native-source-and-navigation.json) / 实窗/42scalar/滚动/真实5和6/原生回归/Release | 6a59193 |
| [ ] [R4.2.03](PRE_ALPHA_4.md#task-r4.2.03) | 原生IME、UndoManager与草稿屏障 | in_progress | [真实输入组/marked保护](../acceptance/pre-alpha.4/development/r42/native-composition.json) / 真实候选窗口及离屏pin仍待验 | 本批待补SHA |
| [ ] [R4.2.04](PRE_ALPHA_4.md#task-r4.2.04) | 非求值Preview/Complete/Hover和高亮 | planned | — | — |
| [ ] [R4.2.05](PRE_ALPHA_4.md#task-r4.2.05) | 本地snippets、希腊快捷、Fix和缩进 | planned | — | — |
| [ ] [R4.2.06](PRE_ALPHA_4.md#task-r4.2.06) | 长笔记本布局、复用与scroll anchors | planned | — | — |
| [x] [R4.2.07](PRE_ALPHA_4.md#task-r4.2.07) | SwiftMath原生公式和原式fallback | completed | [实际57回执/字体/原生附件](../acceptance/pre-alpha.4/development/r42/native-math.json) | 21355b3 |
| [ ] [R4.2.08](PRE_ALPHA_4.md#task-r4.2.08) | Markdown AST、数学扫描与连续原生流 | planned | — | — |
| [x] [R4.2.09](PRE_ALPHA_4.md#task-r4.2.09) | 结果状态、解条件Root与数值投影 | completed | [实际12型/引用/数值](../acceptance/pre-alpha.4/development/r42/native-results.json) | 40cc1cb |
| [x] [R4.2.10](PRE_ALPHA_4.md#task-r4.2.10) | 矩阵、表格、记录与科学诊断 | completed | [真实21输入/分页/原式/科学来源](../acceptance/pre-alpha.4/development/r42/native-values.json) | 本批 |
| [ ] [R4.2.11](PRE_ALPHA_4.md#task-r4.2.11) | 真实步骤、变量与独立讲解入口 | planned | — | — |
| [ ] [R4.2.12](PRE_ALPHA_4.md#task-r4.2.12) | CoreGraphics二维和实际采样控制 | planned | — | — |
| [ ] [R4.2.13](PRE_ALPHA_4.md#task-r4.2.13) | Metal三维原生基础管线 | planned | — | — |
| [ ] [R4.2.14](PRE_ALPHA_4.md#task-r4.2.14) | 参数探索、视窗保留和真实图形预览 | planned | — | — |
| [ ] [R4.2.15](PRE_ALPHA_4.md#task-r4.2.15) | 原生菜单、命令、文件导出和复制 | planned | — | — |
| [ ] [R4.2.16](PRE_ALPHA_4.md#task-r4.2.16) | 原生主题、可访问语义与初步组件审计 | planned | — | — |
| [ ] [R4.2.17](PRE_ALPHA_4.md#task-r4.2.17) | 本地计算首启、欢迎恢复与助手空态 | planned | — | — |
| [ ] [R4.2.18](PRE_ALPHA_4.md#task-r4.2.18) | 关于、手工更新、诊断与新数据管理 | planned | — | — |
| [x] [R4.3.01](PRE_ALPHA_4.md#task-r4.3.01) | 规范Agent消息、任务与IPC身份 | completed | [真实source/CABI/私有进程与耐久ACK](../acceptance/pre-alpha.4/development/r43/agent-protocol-native.json)；357DTO、20Rust/3Node/实际SQLite恢复，合成helper无模型/工具执行 | 本批D045 |
| [x] [R4.3.02](PRE_ALPHA_4.md#task-r4.3.02) | 固定Pi Core与随包生产helper | completed | [实际随包Node/Pi/原生任务](../acceptance/pre-alpha.4/development/r43/pi-production-runtime.json)；86依赖/4169资源、13帧/3事件、SIGKILL后原文档4 | 本批D047 |
| [x] [R4.3.03](PRE_ALPHA_4.md#task-r4.3.03) | sans-IO单轮模型codec基础 | completed | [实际Rust codec/Pi/原生帧及两模型SSE](../acceptance/pre-alpha.4/development/r43/single-turn-codec.json)；10单轮/182回归、4Pi、真实SQLite ACK；生产transport仍后续 | 本批D048 |
| [ ] [R4.3.04](PRE_ALPHA_4.md#task-r4.3.04) | Swift URLSession和SecretStore传输 | in_progress | [真实请求账本/随包Pi/强杀恢复](../acceptance/pre-alpha.4/development/r43/provider-durable-request.json)、[过期计时回调修复](../acceptance/pre-alpha.4/development/r43/provider-timeout-race.json)；实际计时竞态RED→GREEN；物理锁屏仍未完成 | D052/本批D054 |
| [x] [R4.3.05](PRE_ALPHA_4.md#task-r4.3.05) | 读取笔记本与真实函数资料工具 | completed | [只读基础](../acceptance/pre-alpha.4/development/r43/readonly-services-foundation.json)、[耐久工具/Pi](../acceptance/pre-alpha.4/development/r43/readonly-native-tool.json)、[实际输出/持久历史](../acceptance/pre-alpha.4/development/r43/readonly-outputs-history.json)；1297Rust/实际签名App两轮/重启history/独立worker | D053/D055/本批D056 |
| [x] [R4.3.06](PRE_ALPHA_4.md#task-r4.3.06) | 预览与原子修改工具接入 | completed | [预览/Pi三轮](../acceptance/pre-alpha.4/development/r43/patch-preview-foundation.json)、[实际原子提交/Pi四轮](../acceptance/pre-alpha.4/development/r43/patch-atomic-apply.json)；1301Rust/38帧31事件/5真实边界/原回执恢复 | D057/本批D058 |
| [ ] [R4.3.07](PRE_ALPHA_4.md#task-r4.3.07) | 执行、结果检查和隔离试算工具 | in_progress | [依赖规划](../acceptance/pre-alpha.4/development/r43/math-run-planning.json)、[实际结果检查/Pi](../acceptance/pre-alpha.4/development/r43/math-inspect-native.json)、[独立scratch/Pi](../acceptance/pre-alpha.4/development/r43/math-scratch-native.json)、[主run执行器](../acceptance/pre-alpha.4/development/r43/math-run-executor.json)；Rust逐格执行/原回执链已接通，Swift耐久组与Pi执行仍未完成 | D059/D060/D061/本批D062 |
| [ ] [R4.3.08](PRE_ALPHA_4.md#task-r4.3.08) | 操作核对、撤销和Host停止控制 | planned | — | — |
| [ ] [R4.3.09](PRE_ALPHA_4.md#task-r4.3.09) | 附件工具与宿主处理注册接口 | planned | — | — |
| [ ] [R4.3.10](PRE_ALPHA_4.md#task-r4.3.10) | 全工具原生声明与provider严格schema转换 | planned | — | — |
| [ ] [R4.3.11](PRE_ALPHA_4.md#task-r4.3.11) | 实际Pi合成provider编辑运行修正闭环 | planned | — | — |
| [ ] [R4.3.12](PRE_ALPHA_4.md#task-r4.3.12) | Agent任务状态、模式和预算执行 | planned | — | — |
| [ ] [R4.4.01](PRE_ALPHA_4.md#task-r4.4.01) | 新ProviderRegistry和凭据候选事务 | planned | — | — |
| [ ] [R4.4.02](PRE_ALPHA_4.md#task-r4.4.02) | 模型身份、预设和逐项继承 | planned | — | — |
| [ ] [R4.4.03](PRE_ALPHA_4.md#task-r4.4.03) | 能力证据、参数与思考强度映射 | planned | — | — |
| [ ] [R4.4.04](PRE_ALPHA_4.md#task-r4.4.04) | 目录发现、分项Probe与回执 | planned | — | — |
| [ ] [R4.4.05](PRE_ALPHA_4.md#task-r4.4.05) | 原生供应商和模型管理界面 | planned | — | — |
| [ ] [R4.4.06](PRE_ALPHA_4.md#task-r4.4.06) | 用途映射、全局关闭和本次模型选择 | planned | — | — |
| [ ] [R4.4.07](PRE_ALPHA_4.md#task-r4.4.07) | 原生粘贴、拖入、文件承诺与附件原件 | planned | — | — |
| [ ] [R4.4.08](PRE_ALPHA_4.md#task-r4.4.08) | ImageIO/Vision与PDF真实准备 | planned | — | — |
| [ ] [R4.4.09](PRE_ALPHA_4.md#task-r4.4.09) | 音频本地/云转写与播放 | planned | — | — |
| [ ] [R4.4.10](PRE_ALPHA_4.md#task-r4.4.10) | 视频选段、帧/音轨与远程文件生命周期 | planned | — | — |
| [ ] [R4.4.11](PRE_ALPHA_4.md#task-r4.4.11) | 全部目标protocol和rich request编译 | planned | — | — |
| [ ] [R4.4.12](PRE_ALPHA_4.md#task-r4.4.12) | 媒体处理设置、选择范围与工具闭环 | planned | — | — |
| [ ] [R4.4.13](PRE_ALPHA_4.md#task-r4.4.13) | PromptRegistry分层/版本与模板 | planned | — | — |
| [ ] [R4.4.14](PRE_ALPHA_4.md#task-r4.4.14) | ContextPlanner和真实发送前快照 | planned | — | — |
| [ ] [R4.4.15](PRE_ALPHA_4.md#task-r4.4.15) | 任务记忆、规范完整会话与admission ACK | planned | — | — |
| [ ] [R4.4.16](PRE_ALPHA_4.md#task-r4.4.16) | 有界输出引用、裁减和历史压缩 | planned | — | — |
| [ ] [R4.4.17](PRE_ALPHA_4.md#task-r4.4.17) | 真实FIM ghost与原生接受生命周期 | planned | — | — |
| [ ] [R4.4.18](PRE_ALPHA_4.md#task-r4.4.18) | 原生助手对话、固定输入与工具轨迹 | planned | — | — |
| [ ] [R4.4.19](PRE_ALPHA_4.md#task-r4.4.19) | 本轮输入、提示配置和记忆原生管理 | planned | — | — |
| [ ] [R4.4.20](PRE_ALPHA_4.md#task-r4.4.20) | 全模型媒体上下文集成与失败回归 | planned | — | — |
| [ ] [R4.5.01](PRE_ALPHA_4.md#task-r4.5.01) | 功能对齐、能力目录和完整范围审计 | planned | — | — |
| [ ] [R4.5.02](PRE_ALPHA_4.md#task-r4.5.02) | 生产依赖、字体、工程与许可最终固定 | planned | — | — |
| [ ] [R4.5.03](PRE_ALPHA_4.md#task-r4.5.03) | 一次统一运行版本为pre-alpha.4 | planned | — | — |
| [ ] [R4.5.04](PRE_ALPHA_4.md#task-r4.5.04) | Release自包含构建与RuntimeManifest | planned | — | — |
| [ ] [R4.5.05](PRE_ALPHA_4.md#task-r4.5.05) | 候选冻结和同SHA输入证据 | planned | — | — |
| [ ] [R4.5.06](PRE_ALPHA_4.md#task-r4.5.06) | 开发Preview自包含安装与隔离核对 | planned | — | — |
| [ ] [R4.5.07](PRE_ALPHA_4.md#task-r4.5.07) | 实现gate报告、原件索引和聚合器 | planned | — | — |
| [ ] [R4.6.01](PRE_ALPHA_4.md#task-r4.6.01) | 同candidate全部contracts/Rust/WASM回归 | planned | — | — |
| [ ] [R4.6.02](PRE_ALPHA_4.md#task-r4.6.02) | Release原生/存储/竞态/codec全验 | planned | — | — |
| [ ] [R4.6.03](PRE_ALPHA_4.md#task-r4.6.03) | 真正原生UI、IME、Markdown和Metal验收 | planned | — | — |
| [ ] [R4.6.04](PRE_ALPHA_4.md#task-r4.6.04) | 原性能定义和Mac交互压力测量 | planned | — | — |
| [ ] [R4.6.05](PRE_ALPHA_4.md#task-r4.6.05) | 真实Pi/model/media协议fixtures全回归 | planned | — | — |
| [ ] [R4.6.06](PRE_ALPHA_4.md#task-r4.6.06) | Windows/Web/CLI/iOS同SHA实际门禁 | planned | — | — |
| [ ] [R4.6.07](PRE_ALPHA_4.md#task-r4.6.07) | 到位发行材料和签名工具链验证 | planned | — | — |
| [ ] [R4.6.08](PRE_ALPHA_4.md#task-r4.6.08) | 最终App、DMG和ZIP签名公证封存 | planned | — | — |
| [ ] [R4.6.09](PRE_ALPHA_4.md#task-r4.6.09) | 最终公证包的隔离下载和安装实跑 | planned | — | — |
| [ ] [R4.6.10](PRE_ALPHA_4.md#task-r4.6.10) | 最终Mac包真实模型和视觉现场闭环 | planned | — | — |
| [ ] [R4.6.11](PRE_ALPHA_4.md#task-r4.6.11) | 文档、组件与九最终资产综合核对 | planned | — | — |
| [ ] [R4.6.12](PRE_ALPHA_4.md#task-r4.6.12) | 公开前逐gate汇总与ready判定 | planned | — | — |
| [ ] [R4.7.01](PRE_ALPHA_4.md#task-r4.7.01) | 创建完整GitHubdraft并发布pre-release | planned | — | — |
| [ ] [R4.7.02](PRE_ALPHA_4.md#task-r4.7.02) | 公开tag九资产字节与结构回读 | planned | — | — |
| [ ] [R4.7.03](PRE_ALPHA_4.md#task-r4.7.03) | 公开Mac包与CLI用户流程最终实跑 | planned | — | — |
| [ ] [R4.7.04](PRE_ALPHA_4.md#task-r4.7.04) | 长期证据归档与最终发布回执 | planned | — | — |
| [ ] [R4.7.05](PRE_ALPHA_4.md#task-r4.7.05) | 收尾任务账本与交付说明 | planned | — | — |

## 实施记录模板

```text
Task / phase:
前置与实现文件:
接口/裁决变化:
命令/实际环境/配置:
结果/失败/取消/attempt:
Done依据与覆盖case/proof:
原件URI与SHA256:
后续补记提交SHA:
剩余事项/下一可执行任务:
```

任务有部分完成时保留unchecked，记录已经可验证部分；未知提交/现场阻塞不写Done。只有同candidate全gate真实通过且公开下载回读结束才能填写发行完成。

## 本次计划整合记录

已按旧PLAN的规格＋逐任务格式合并13篇专题、共享续行、版本/门禁及完整发行步骤；没有执行R4任务。结构检查见[planning-review.json](../acceptance/pre-alpha.4/planning-review.json)。

## R4开发记录 D001 — 基线与发行前置

- R4.0.01：公开`.3`与本地tag同为0889d34e4fce9926054025ca71ea328f6cc65b39；原53语料逐字节与公开tag一致，保存逐条散列、三个受保护tag、九公开附件/digest与已有功能源索引。历史发行证据是发布后文档，不误称包含在原tag。当前安装仍为Tauri`.3`。
- R4.0.11：实际macOS27.0/26A428、Xcode27.0/27A266a、ARM64、Rust1.94及四targets就绪；host Node25.9与拟随包Node26.11.1区分。`bash macos/Scripts/verify-env.sh`通过；`--require-distribution-signing`按预期拒绝，仅developer_id_application_missing。Developer ID身份0、Apple Development1，没有导出个人身份/凭据。
- 签名CI秘密接口/临时Keychain/finally清理与新Registry现场测试范围见macos/native-package/signing.example.json；它不是已签名配置或真实live通过。最终R4G21/live仍not_run，缺材料不阻挡独立开发。
- 证据：[baseline/index.json](../acceptance/pre-alpha.4/baseline/index.json)、[环境摘要](../acceptance/pre-alpha.4/baseline/environment.json)；原命令输出在target/acceptance/pre-alpha.4/development/r40/。结构、版本、tag和任务状态核对，不改`.3`源码版本或资产。

## R4开发记录 D002 — 共享续行（代码完成，跨端验证进行中）

- R4.0.08：只在需要右操作数时跳过现代方言换行；分号/EOF/下一条let留在恢复边界，缺失右值只指向对应操作符。非法声明的参数/函数环境按语句恢复，保留独立未知函数提示和之前合法绑定。Wolfram/InputForm/已完整语句不改变；不在UI删除换行。
- 新增6个解析器回归（等价AST/边界/Unicode/恢复）及3个真实内核回归（Preview无执行/实际结果/3→6/721网格草方块）。原53语料整体/逐条散列不变。完整Rust工作区1094通过、2个原ignored保留；Clippy通过。生产WASM重建后78前端单元通过，新增真实WASM源码保存/执行用例；TypeScript严格检查修正新用例的tuple/空格索引边界。17 Python通过，函数目录829项/22页无漂移。
- 移动新增testModernContinuationUsesIdenticalSourceForPreviewAndExecution，明确检查真实Preview类型和字段；仅交GitHub CI执行，不在本机开模拟器。未取得同提交CI前R4.0.08/09保留in_progress，最终32candidate gate仍not_run。
- 原始日志在target/acceptance/pre-alpha.4/development/r40/，保留初始失败和修复后的attempt；开发证据不是`.4`公开候选或已发布声明。下一步取得手机/平板CI，继续依赖闭包和新Mac工程。

## R4开发记录 D003 — 依赖闭包完成

- R4.0.02：Pi Core/AI1.0.4精确SRI/版本、86项生产依赖/实际API/安装钩子与可选Kerberos区分，纯Node原生addon数0。忽略安装钩子后真实Agent构造且sequential/空工具列表符合实际API，无模型请求。干净npm ci后依赖及许可散列完全相同。
- Node26.11.1官方darwin-arm64归档与固定SHA一致、原二进制strict代码签名成功；记录OpenPGP未测和helper/app重签名未做。SwiftMath1.7.3/Markdown及cmark0.9.0实际checkout提交核对，28项字体bundle资源及原MIT/OFL/GUST许可登记；SQLite系统版本3.54.0/unix VFS选择及未测耐久明确。
- [依赖清单](../../macos/native-package/dependencies.json)是真实开发审计，不是生产RuntimeManifest；声明/上游正文来源、npm发布gitHead与参考Git SHA差异见R4-D001和第三方许可。已补全缺正文的实际库许可，未给GPL/未知许可做全局豁免。
- 共享续行的所有本地Rust/WASM/前端检查已通过；原CLI Release53/200ms正在独立执行，新增Swift用例尚待CI。接着R4.0.03可复现Mac工程/真实Rust链接，之后冻结DTO/宿主服务。

## R4开发记录 D004 — 原生链接与性能原件

- 原生Xcode工程/scheme已生成，generate-project.py --check无漂移。ARM64/macOS27 Release通过，独立Preview Bundle ID，ad-hoc strict签名校验与实际原生窗口启动通过；C ABI metadata读回26、ABI1。UIView无WebView/Tauri，不将链接窗口标完整宿主或可发布UI。当前版本由真实Rust静态字符串/Bundle目标取得，避免冻结时漏改硬编码。
- 首轮CLI Release53正确性均通过，case12/17/41整次kernel timing分别292.349/236.792/245.627ms超过原200ms；保存原失败，不删断言。三个有界公开.3/开发构建A/B回合数学InputForm一致、12约80..92ms/17约61..63ms/41约51..55ms，未见稳定新代码回归，首轮波动根因尚未完全确定。
- 编译全部结束后的独立第二次完整53原200ms门禁通过，未改语料/阈值。保留首次失败和A/B原件；此开发复核不是最终同candidate的R4G22/全平台通过。
- 原件native-app-build.log、native-authority-performance.log、performance-ab.json、native-authority-independent-2.log在target/acceptance/pre-alpha.4/development/r40/。采用telegram-ui-reference的实际空间/状态反馈基础路径，缺业务能力明确显示；没有本地iOS模拟器。

## R4开发记录 D005 — 原生工程任务完成

- R4.0.03完成：工程生成/校验、Rust ARM64静态库及Swift Release链接成功；新C ABI真实查询metadata26/ABI1/Rust实际版本字符串，Xcode原生包Mach-O ARM64、macOS27、Preview身份经回读。旧进程正常退出后用CUA启动最终构建，真实AX窗口/截图可见相同元数据；nm确认三条桥接符号位于实际exe。
- om-apple-ffi定向测试与Clippy通过；核心仍forbid unsafe，独立ffi仅no_mangle边界和静态版本指针，没有修改iOS ABI。未实现create/submit等不注册，也不把此窗口称完整CAS/Agent客户端。
- 构建和当前开发边界见[macos/README.md](../../macos/README.md)，后续R4.0.04冻结生成DTO/错误接口，R4.0.05/06/07补安全服务、生命周期和实际求值。

## R4开发记录 D006 — DTO/ABI与跨端续行阶段验收

- R4.0.04完成：native_contracts.py从3份机器Schema生成91个Rust/Swift DTO，单值kind枚举确保union正确分派，必填nullable包装拒绝遗漏，结构/原计数/字节预算/状态条件和owner代次拒绝已有9项Rust实际验收。Swift4负例、常量kind路由及实际Rust→Swift→Rust最高精确计数字节往返通过。生成漂移检查、完整Mac Release重建和host-service Clippy通过；旧iOS ABI未改。
- C头与create_result(handle,error)所有权由native-host-abi.json统一生成。当前只是冻结声明；R4.0.06仍需真正实现、销毁/取消/缓冲竞争测试。声明与JSON形状不授予权限，不开放未注册handler，不把回执载荷当持久化成功证据。
- R4.0.08/09完成：525ce5629908a010ece9ec42283d57f5c437664d的CI37880165376全部job成功；新增移动续行用例iPhone/iPad实跑通过，各30单元+5UI。原53整次最大phone306.521ms/pad337.052ms，unsupported_latex=[]；纯WASM/生产53独立数学回读/前端/依赖全部成功。原CLI本地完整53/200ms第二attempt通过，首次波动失败和A/B保持。没有改原数学期望/时限或启动本机模拟器。
- [开发摘要](../acceptance/pre-alpha.4/development/r40/summary.json)保存实际source/CI/附件散列与范围。新DTO提交尚不等于该旧SHA的全CI证据；本版完整native/live/签名/最终candidate仍未完成，32最终门禁不填旧成功。
- 下一步直接安全DocumentCoordinator控制通道与独立Kernel/Editor/Aux worker、每操作取消及实际服务事件；存储/事务/checkpoint/完整UI/Agent仍按完整计划继续，不收窄发布范围。

## R4开发记录 D007 — 安全调度与独立操作取消

- R4.0.05完成：安全host-service实际创建DocumentCoordinator短控制通道与Kernel/Editor/Aux独立有界worker；Preview与隔离scratch都由真实Session处理。状态读取不排在长CAS后，直接取消为每操作独立token；外部token不会被Session内部重置，旧默认Session语义保持。取消排队任务不会执行，关闭先停admission并signal，后台finish才join，后续工作不清零旧取消。
- 10调度+9契约测试通过，实际let a=5返回6且下一scratch不继承a；长map运行期间状态读取/独立Preview可用、过载拒绝、排队取消/终止状态/背压resync/保留结果/旧runtime/source拒绝均验证。catalog真实全量逐页回读与已注册callback一致；状态只返回活动与最近32终止摘要，按完成顺序回收大详情，旧操作ID仍可核对。数学错误另有run_outcome=failed，不把传输完成当数学成功。
- new直接构造也重新校验init边界；catalog在独立worker分页，不把大序列化塞控制步骤。当前没有文档持久化/写入/definition snapshot接纳，相关入口显式不开放；native_renderers_ready=false与内核Desktop数学能力区分。框架禁unsafe与iOS原ABI保持。
- host-service Clippy通过；kernel reactive/editor/artifacts/explore/operation_cancel定向兼容与纯WASM构建已通过。初始字段名/类型编译错误原件保留，最终以scheduler-tests-final3.log、scheduler-clippy-final3.log为准。提交21c9254的CI37883006504实际全部success；新scheduler尚待自己SHA的CI。
- 原始命令/日志保存在target/acceptance/pre-alpha.4/development/r40/，[scheduler.json](../acceptance/pre-alpha.4/development/r40/scheduler.json)登记散列与范围。这不是最终candidate gate，完整native UI/存储/Agent/签名仍待后续。下一步R4.0.06真实CABI句柄、owned bytes与关闭竞争。

## R4开发记录 D008 — 真正C ABI生命周期与Swift后台客户端

- R4.0.06完成：create/submit/next_events/cancel/close_begin/close_finish/buffer_free与三项probe共10符号均位于实际Release静态库。只在独立ffi输入边界使用from_raw_parts且立即复制；安全registry不解引用C句柄。guard计数与registry admission同一短锁顺序，finish先撤销新调用、等待活动guard与owner停止再释放完整服务。稳定token小tombstone保持进程期且有界，避免旧句柄ABA复用；buffer只按注册分配及长度释放，不能从外部pointer重构Box。
- 最新9项Rust ABI验收通过：null/空/非法UTF-8/未知字段/超限、真实4与数学失败、借用输入在返回后被销毁、长度错误/重复/伪buffer、旧句柄/重复取消、实际活动guard阻挡关闭及并发poll/submit/cancel/close均覆盖；每测试核对live host和owned buffer数/字节恢复。panic/整数路径在边界捕获；iOS旧ABI没有改动。
- Swift直接调用Release库通过：精确4、真实parse错误run_outcome、Desktop能力、直接取消、decode/free与消费旧句柄。DTO回读/四负例/constant kind也通过；NativeHostClient独立control/events/cancel/shutdown后台队列通过Swift6严格检查与实际Release链接。客户端现场fixture核对状态读取/直接取消、MainActor事件轮询期间26次调度和后台finish。首轮等待300ms后任务已终止，取消正确返回false但fixture原precondition失败；保留原件，并把运行中取消提前、主线程轮询验证独立进行，不改产品取消含义或运行预算。
- 工作区完整Rust1122通过/2原ignored、全Clippy、纯WASM、17Python、生成协议/工程/函数文档与结构检查通过；完整回归之后的新ABI负例单独再验收为9通过。Mac ARM64/SDK27 Release build及strict ad-hoc签名通过。新的macos-native.yml在Xcode27镜像持续构建/执行Swift实际ABI与客户端fixture；它不代替最终签名公证/真实UI/candidate gate。
- R4.0.07继续in_progress：后台transport与实际计算已实现；App窗口仍是链接预览，NotebookViewModel、producer独立计数、重复/缺口/旧scope reducer与resync尚待接通，不能标完整工作台或此任务完成。
- [ffi.json](../acceptance/pre-alpha.4/development/r40/ffi.json)记录原始日志/散列；原件在target/acceptance/pre-alpha.4/development/r40/。前一scheduler提交daecc3f的CI37895404872查询时Rust/frontend/dependencies成功、iOS仍进行，不能写同SHA全绿或用它证明本次ABI。下一步直接R4.0.07，随后fixture基础/SQLite持久化与完整原生/Agent。

## R4开发记录 D009 — 独立读回、序列恢复和原生试算

- R4.0.07完成：96个共享DTO新增HostSnapshotQuery/HostSnapshot/HostOperationStatus，新的om_host_read_snapshot通过稳定guard读取实际owner状态与序列，最多4项详情、有界全部摘要，不接纳新操作、不消耗事件队列。终止接纳/事件生成与读取采用同一短投影顺序，JSON编码/解码/释放均后台进行。当前document_binding=null/storage_ready=false，未制造第二份笔记本或保存成功。
- AppEventRouter按runtime/document/liveness/sequence拒绝旧回执，Rust/source/UI计数独立；未知事件需要owner基线。NativeHostSession建立单事件消费者、最多32 pending/8 UI订阅、admission-before-delivery关联与原ID恢复。UI流是有界最新完整投影，终止awaiters独立；停止只能由实际终止/读回确认，重复停止不再发请求，并发close等待同一真正关闭。
- 纯reducer验证重复/缺口/旧runtime与文档/不同producer计数/背压/空批次序列位置/关闭后拒绝。真实ABI fixture将event字节预算压到128（所有终止帧无法容纳），12次精确4通过独立读回获得；慢订阅取得最新完整结果，真实解析错误、停止与晚到停止、两次并发关闭通过。
- 首次强制丢帧fixture停住，sample显示CAS/控制worker已空闲、消费者等待事件；查明在途旧snapshot覆盖较新resync信号。增加batch.last_rust_event_sequence与恢复revision，旧快照不能清除新缺口，读回跨越最新位置才恢复；首轮原件保留。第二attempt又发现await响应先于UI完整投影发布，改为先发布再resume，第三及最终全部通过；不通过无限重试、伪成功或放宽数学预算绕过。
- NativeHostClient所有ABI/编码/解码队列严格检查非主线程，NotebookViewModel只持确认事实和隔离draft。SDK27 ARM64 Release通过；CUA真实AX观察2+2→4、源码编辑保留旧结果并标已修改、错误显示“表达式尚未完成，需要右侧表达式”、长solve/map仍可停止且实际已停止、停止后新计算4、⌘↩多行f(4)→5。仅手工隔离试算，完整工作台/文件/渲染/Agent仍按后续88任务实施。
- 本批最新host-service 11调度+9契约、apple-ffi 9 ABI、Clippy/deny、17Python及生成协议/工程检查通过。Swift脚本执行实际ABI/DTO负例/actor/reducer/session五类fixture。原kernel/iOS ABI和53数学期望未改，没有本地模拟器。
- c9bb366的CI37897937189整体失败：Rust/frontend成功；dependencies准确指出om-apple-ffi内部path缺version（已改workspace固定版本、本地deny全通过）；Mac实际ABI与DTO通过但300ms≥10心跳测试失败（改非主线程硬检查+真实请求/心跳取得进展，并记录时序，不伪称门禁旧绿）。iPhone原18整次1086.392ms/内核88.524ms，transport queued0.074/encoded0.056/FFI88.661/decode3.286/resume994.294ms，仍超原1s。原门槛保持；后续新SHA需实跑，不能将内核88ms当移动端通过。
- [router.json](../acceptance/pre-alpha.4/development/r40/router.json)登记测试、UI实际观察、原失败与CI附件散列；源码版本仍.3，最终32candidate gates尚未执行。下一步fixture基建/SQLite单写者与真实存储，跟进新CI/移动端恢复阶段延迟。

## R4开发记录 D010 — 自有输入与真实故障基础

- R4.0.10完成：Fixtures/generate.py自绘sRGB图案及2+2=4字形、PCM WAV、带真实文本的PDF、JSON/Unicode SSE/截断UTF-8；SDK27的GenerateVideo使用新PixelBufferReceiver/独占CVMutablePixelBuffer，生成10帧移动标记H264。封存8项输入/SHA256，不读取用户私件，不复制系统字体。视频可能非逐字节重编码，验收总是使用固定封存字节。音频是正弦而非语音，不以它宣称ASR准确。
- 实际ImageIO/PDFKit/AVFoundation读取尺寸/颜色空间、PDF文本、音频帧/采样率/功率及全部10个视频帧的实际时间与移动白标像素；不是读取manifest假装媒体解码。Node26.11.1严格TextDecoder各1/2/3/5/13/64字节split精确还原中文emoji；Rust真正SseDecoder/decode_openai_chunk也跨全部split产生同一文字/Finish，ControlledClock/原token触发实际Interrupt Timeout/Interrupted。
- 真实回环HTTP服务只绑定127.0.0.1，测试显式禁代理，固定分段UTF8、401/429/500、302拒绝和50ms超时可复现，没有付费模型请求。I/O子进程在opened/written/file_synced/renamed/directory_synced真实SIGSTOP后由父SIGKILL，保留各阶段文件；ENOSPC明确注入且失败，无durable receipt。进程中断不代表断电耐久，SQLite原COMMIT/fsync/selector故障仍由R4.1实测，不拿基础fixture替代业务。
- 命令test-fixtures.sh（固定Node26运行）和2项Rust真实codec/budget测试通过，Clippy/deny/17Python通过。初始生成器旧API警告、SDK27sending像素所有权错误与修复、新Rust用错Vec返回签名及文档lint原件保留，最终采用新SDK唯一像素所有权API；没有删除检查或改数学期望。
- manifest、生成器、实际解码测试、agent分片工具和Rust support已入库；原件/残留/attempt保存在target/native-fixture-attempts/及target/acceptance/pre-alpha.4/development/r40/，[fixtures.json](../acceptance/pre-alpha.4/development/r40/fixtures.json)记录散列。macos-native CI新增封存媒体/网络/中断检查。
- ece80a1的CI37916235119读取时native_macos/rust/frontend/dependencies全部success、iOS仍in_progress；此前c9的原18恢复994ms失败继续保留。新fixture提交需用自己的SHA检查，最终candidate gates仍not_run。下一步R4.1.01实际SQLite单写者、根锁、selector和同步证据。

## R4开发记录 D011 — SQLite单写者、库代次与真正同步

- R4.1.01完成：Swift StorageService使用系统目录API，production NativeMac与Preview NativeMacPreview分区；OS flock持有store.lock，不通过删文件抢锁，每库唯一StoreWriter串行utility队列。SQLite指针/stmt不跨API，编码与SQLite/系统sync均后台；关闭队列结束前保留RootLease，forgotten-close final owner也保留lease。Native Preview实际启动新分区并显示“本地存储已就绪”，同窗口2+2返回4；文档事务/文件保存仍未开放，Rust document_storage_ready继续false。
- store.json/active.json严格字段集合及格式/身份检查；SQLite user_version/header/store/minimum_reader/codec与selector header_hash均校验。先readonly检查再配置writer；未来格式拒写，不把已有但未选择的库当空库，不扫描最大代次猜活动库。身份散列排除可变last_clean_shutdown。自有根/Generations目录700、JSON/DB/锁600，相邻临时全同步+rename+目录sync；symlink祖先/根拒绝，仅允许实际本地卷。旧TOML/keyring没有入口，测试不可读旧配置sentinel保持字节。
- 实际链接/usr/lib/libsqlite3.dylib，运行版本3.54.0/unix VFS3，查询回读WAL、synchronous2/FULL、foreign_keys1、fullfsync1、checkpoint_fullfsync1。测试探针最初只监控VFS槽，没有捕获同步；后续隔离fixture dylib转发SDK public fcntl实际调用，区分出系统SQLite用F_BARRIERFSYNC，不能把最初合计15写成15次F_FULLFSYNC。生产SQLiteDatabase已在COMMIT之后对实际WAL执行F_FULLFSYNC，checkpoint对DB执行全刷新；最终记录9次F_FULLFSYNC/9成功及15 barrier（其他header/selector原JSON同步不计为SQLite）。探针只在测试进程，不进入App，不伪造成功。
- COMMIT后同步失败返回unknownCommit，不能宣传回滚或耐久成功。真实SQLite write/pwrite注入ENOSPC明确失败、不发布selector；postcommit F_FULLFSYNC EIO也保持无selector及显式恢复。duplicate root writer/同文档writer、目录只读/符号链接、未知root/user_version、selector缺失、独立重新打开相同identity均通过；主App不接任意模型SQL/路径。
- 实际bootstrap在root_published/before_commit/committed/before_selector/selector_renamed/published六边界SIGSTOP/SIGKILL后重新打开：前四中的已有未选择DB保持recovery_required，新库尚不存在的root-only阶段可正常创建；有效selector后保持同storeID/代次。跨进程第二实例拒绝，原进程死亡后OS释放锁，锁文件未删除。仅证明进程中断及实际同步调用，不冒称实验测过断电。
- test-storage.sh最新完整通过，Swift6严格构建、原生ARM64/SDK27 Release与ad-hoc strict签名、17Python/生成协议/工程/结构检查通过。初始Swiftgetter错误、C探针OFD/系统fcntl参数覆盖不足、漏捕获同步与修复全部原件保留；不删除断言或把PRAGMA成功当耐久结果。SQLite官方WAL/fullfsync文档和本机SDK header核对，SDK内部95只由test转发SQLite已发的调用，生产代码不增加私有API。
- [storage.json](../acceptance/pre-alpha.4/development/r41/storage.json)记录命令、runtime/sourceID、原日志/断点残留与散列。新CI加入实际SQLite/故障测试。627c978的CI37918801398查时Rust/dependencies/native_macos成功、frontend生产34项中33通过，extended plot在log_log最后一次未出现.plot-view，原件保存待定向复现；iOS仍进行。不能把部分成功写最终全平台绿色。
- 下一任务R4.1.02（权威source/revisions/操作唯一键/幂等/回执），其余86任务与所有最终candidate gates保持未完成，运行版本仍.3。

## R4开发记录 D012 — 权威source与同库幂等事务

- R4.1.02完成：安全Rust document模块持已确认NativeSourceSnapshot，单独prepare冻结before/after/完整逆向快照与变更ID，不构造Session或执行定义。正确trusted durable receipt才能推进；旧/unknown回执、不同generation和重复接受不能改authority。源码修订、cell revision与execution epoch独立，暂对whole-source提交保守增长epoch；更细的数学/纯文本失效策略留R4.1.04。语法错误仍可编辑保存。
- native-source-store机器契约与现有DocumentCommit/OperationReceipt记录生成111个跨端DTO；字段/未知字段/nullable/counter保持一致。source/request采用同一长度前缀UTF8+大端整数hash，含原baseline/冻结源码/actor，不含重连传输、generation、回执新ID及提交时钟。源码/顺序/标题不归一化，Swift不使用规范等价String相等来认定source未改；实际提交时钟由可信writer产生。
- DocumentStore通过每库原StoreWriter消费真正Rust冻结计划，在一个SQLite事务写document_revisions/head、transactions（forward与完整inverse）、operations唯一键、operation_transitions和outbox；外键一致，读回校验整个immutable graph与每项原散列。相同operation/内容读原回执，不要求旧base仍是head、不重放后来的head；不同内容IDEMPOTENCY_CONFLICT，新ID/旧base拒绝。没有三个文件假原子、前端第二份authority或原DeleteCell循环。
- 实际Rust计划→Swift物理store测试source0→1→2，原title/Unicode emoji/分解组合字符/NUL源码和cell顺序保留；多次原ID只有一个效果，后续修订后仍返回原回执。故意新增重复cell/错误hash整笔拒绝、插入revision中途ENOSPC整笔回滚、COMMIT后ACK失联unknown与原ID查询、重启head/receipt保持、五类表真实计数3/2/2/2/2均通过。实际Swift回执再次由原Rustauthority核验接纳，不用synthetic receipt宣称成功；example第二计划只从候选数据prepare，明确preparation_only。
- 源码COMMIT后真实F_FULLFSYNC注入EIO：write返回unknown；失败仍生效时receipt query也不能报durable，恢复真实同步后原ID读回才确定完成。receipt query在实际readonly连接校验后再确认已提交字节，既不重放source，也不把旧已完成记录作为未提交。丢失关联记录/断开join是损坏，不当操作不存在。探针仅测试helper。
- 发现旧.omnb v1允许非空Unicode cell ID，原通用Identity ASCII约束会破坏兼容。新增CellIdentity仅用于cell引用，路径/operation/runtime身份仍受控；按原UTF8区分é与e+组合字符，Swift用Data键和字节排序，Rust稳定ID不重分配。source、状态、editor/wire/storage机器定义同步，5项Rust source测试覆盖Unicode ID/源码、独立epoch、generation/overflow等。
- 最新host/ABI 36 Rust通过（9 ABI/9协议/5source/2fixture/11scheduler）、Clippy、Swift真正port/ABI/actor/reducer/session全部通过，bootstrap同步/六SIGKILL点回归、ARM64 SDK27原生Release、17Python、纯WASM/deny/版本/生成检查通过。中间getter/capture/Vec签名和一次修改编译中Swift源造成编译拒绝均保留，后续固定源码的完整attempt通过，不复用旧失败或删断言。
- [document.json](../acceptance/pre-alpha.4/development/r41/document.json)保存日志/真实计划与receipt/散列。UI/Agent patch、editor fence、durable admission、完整undo和checkpoint不在本任务冒称实现，继续R4.1.04–09；当前新写入body仍不注册可调用能力。inline source受2MiB预算，超限明确SOURCE_BLOB_REQUIRED，Blob扩展为下一任务。
- ddbeb88的CI37923343056已结束：native_macos/rust/frontend/dependencies全部成功（原frontend log_log场景在此SHA通过），iOS失败为原17/18整次1330.407/1026.974ms超1s，原门槛及数学期望不改，本次保留job原件但未取得新的分段附件前不推测原因。不是完整同SHA平台绿色，也不填写最终candidate门禁。下一步不可变Blob与引用，完整85项继续。

## R4开发记录 D013 — 不可变Blob与同库资源引用

- R4.1.03完成：独立BlobStore utility有界队列（最多32任务、128 publication pins、64 reader handles），只用SHA256推导Blobs/sha256/<prefix>/<hash>目标。64KiB流式复制/散列、真实F_FULLFSYNC、同本地卷hard link不覆盖发布、父目录sync、实际整文件hash/length复核完成后才发ready/pin。单件128MiB与读取页1MiB预算明确；输入源fd前后stamp核对，非regular输入用O_NONBLOCK打开后拒绝，避免FIFO挂死。临时路径/媒体类型不由模型决定；新可调用工具仍未注册。
- BlobReferences在所属Library/文档writer的同一库事务写blob_objects+blob_refs，Owner/codec/mime/hash/length全部核对，同owner/hash不同metadata冲突不覆盖。StorageService先验证incoming pin并建立独立transfer pin，覆盖数据库等待期间；失败最多留下孤儿，不能让未发布字节进入DB。提交后回执失联为unknown，原owner/hash独立读取可核对；broken join是损坏，不作不存在。后续source/checkpoint/会话的同业务事务引用接对应写入任务，跨库整体原子/GC不在此冒称完成。
- 分块读取持实际已验证fd+stamp，reader本身pin资源；原publication释放后reader仍可读取。重启临时pins/reader IDs失效，但通过持久owner ref重新核验真实字节并建立新reader，无须再次附加原文件。检查返回missing/corrupt/pinned/referenced/orphan_candidate及allStoresChecked；未打开的文档存在则明确检查范围不完整，不删除候选孤儿。完整租约/保留/GC仍属R4.1.13。
- 实测原PDF逐字节、3MiB+17自有数据按1MiB页整合、空文件、同hash不同长度及同长度错内容、不覆盖损坏目标、错hash/伪pin/缺文件、源输入变化、FIFO与128MiB+1 sparse拒绝均通过。数据库插入后ENOSPC回滚留下unreferenced orphan；提交后ACK失联查询成功；取消在复制后的真实per-job信号阻止发布。close_begin先撤销Blob admission/signal独立token，不等SQLite结束才取消；background close才settle/readers清理，root lease在所有owner停前保留。
- 自有helper实际在stage_opened/copied/file_synced/before_publish/linked/directory_synced六位置SIGSTOP后SIGKILL：前四无ready目标，后两只能有完整未引用字节，重启零reference/pin。原残留在target/macos-storage-tests/，不把测试进程中断等同断电实测。临时正常失败清理仅app-owned staging，不删来源或损坏目标。
- test-blobs最新完整通过；bootstrap六崩溃/未知format/根锁/ENOSPC、实际source事务/真实F_FULLFSYNC错误回读、Native ABI/actor/reducer/session回归也通过；SDK27 ARM64 Release/17Python/生成工程/契约检查通过。初次Swift semaphore async属性与语法错误日志保留，修正成fixture后台同步等待，无产品假成功。没有改Rust CAS/原53语料/旧tags/运行版本，未运行本机模拟器。
- [blobs.json](../acceptance/pre-alpha.4/development/r41/blobs.json)记录日志和实际发布中断attempt/hash；macos-native CI新增该脚本。c310565/CI37927161789结果native_macos/rust/dependencies/iOS成功，frontend失败是dev西瓜.scene-view 30s未出现；实际error-context仍为Running且UI未有结果，原件target/ci-evidence/c310-frontend，不根据间接上下文宣布根因或放宽门槛。此CI不是本批SHA或全平台green。
- 下一任务文档操作/失效与预览/编辑屏障、完整checkpoint及UI/Agent；Blob字节机制已完成，类型化checkpoint/媒体准备、scoped工具权限、完整GC仍按后续原计划接入，84任务/全部candidate门禁保持未完成。

## R4开发记录 D014 — 源码批量操作、非执行失效与启动快照保护

- R4.1.04完成：SourceCoordinator在临时file一次验证最多64个typed insert/update/delete/move/rename，迟到非法操作整笔拒绝、未改authority。共享om-kernel::source用真实parser/lexical dependency分析，发现跨格function声明、符号 uses/defines、重复定义、真正SCC循环与blocked节点；无Session/Evaluator构造或statement执行。图以symbol owner索引生成，显式200000 edge上限，SCC采用迭代O(V+E)算法，不用递归爆栈或把Kahn全部余项假称cycle。
- 实际Session新增kernel-owner API apply_source_file_without_evaluation，一次换source/顺序，清除受影响旧owner的真正eval.defs，标stale并保留历史；不调用旧DeleteCell/Upsert cascade。实际kernel测试先得到a=2/b=3，再整批删a，只保留b源码：原a和b旧值已清除，读取b/a都是symbol、没有中间自动计算。真实f调用的value旧owner也纳入依赖闭包；非法ID批次不会改变原定义。旧request/iOS接口与默认reactive行为未改。
- Math相对序列/内容/kind/dialect及实际计算设置保守增长execution_epoch；title/Text/Ask变更、纯prose插入/移位不误使Math索引变化变成计算变化。源码revision仍增长、cell内容revision只对实际原UTF8变化增长。Locale与execution不同；设置由NativeCalculationChange冻结实际before/after、参与request hash并与source同库保存，writer核对已有settings基线，独立回读重启可恢复实际after。纯标题保持epoch2、strict constants设置使epoch3，数据库五类计数现为5/4/4/4/4；语言-only在kernel/source测试不清数学值。
- 机器契约126 DTO新增5种SourceOperation及actual calculation change；真实slot字段均typed，None calculation变更的原语义hash保持，settings新值进入canonical长度前缀hash。physical设置信息取同库forward plan，反向数据保留原source与settings before；scope/fence/grants/预览以及完整undo仍由后续任务接入，不提前开放模型handler。
- 同时复现共享前端启动竞态：initial get_notebook_state在新编辑期间返回，会把新多行f源码恢复成旧1+1。新增受控顺序的state单元回归先实际失败，再以启动代次/liveness和dirty source guard修复；source未触动才whole replace，否则只merge匹配metadata。只证明这项明确的源码覆盖bug，不把c310西瓜Running故障的根因借此断言已查明。
- 完整Rust工作区1141 passed/2原ignored、全Clippy、纯WASM、前端79单元/lint/TypeScript严格生产build、生产Web定向6项（原extended/log_log、L2真实SVG、切开西瓜真实GPU旋转/OBJ）全部通过。Mac ARM64 SDK27 Release、actual source/settings/ACK失联/fullsync回读、native ABI/actor/reducer/session、bootstrap/Blob故障、17Python通过；原53期望和时限未动，本机没有iOS simulator。最初Rust借用闭包生命周期/unused parens、前端错误调用不存在的check脚本原件保留，最终以工作区/production build/正确脚本为准。
- [source-operations.json](../acceptance/pre-alpha.4/development/r41/source-operations.json)记录日志与失败/修复attempt/hash、范围；UI/Agent写入仍等待R4.1.05/06真实冻结计划与编辑屏障，后续main CAS checkpoint仍需R4.1.08/09，不将本批当完整native数学工作台。下一步预览/身份分配/期限/原始plan，再DocCommitPort，剩余83任务/最终candidate gates继续。

- D014补记：26078d0/CI37931481338已实际全job success（包括iOS和前端），此前失败原件仍保留；本批新source代码还需自己的SHA。旧SHA success不填最终candidate门禁。

## R4开发记录 D015 — 不求值冻结预览与原引用

- R4.1.05完成：PreviewService从实际SourceDocument与完整read exposure构造不可变snapshot，保留原source/title/order及真实coordinator解析绑定。模型参数只含snapshot_ref/input；source仅检查语法/方言/参数且不产可提交ref，patch全部在临时文档校验，再冻结NativeSourceCommit、原normalized operations、实际失效/诊断、新key→cell ID、完整scope与plan_hash。不构造CAS Session，不推进source authority，也不将合法parse当计算成功/保存完成。
- native-preview.schema.json与agent-tools参数约束逐项对应：source/patch闭联合，新增仅Math/Text、保留既有Ask，最多64操作；new client_key必须唯一，anchor只能引用已存在/先前声明key。update需要原read完全source可用；replace按原快照SHA256核对，片段按临时source顺序匹配原UTF8字符边界，并计入重叠匹配（aaa中的aa不是唯一）。删除/修改只能已有cell，重复整格write或混合整格+片段、错误anchor/未知source hash/extra权限字段全部拒绝，后面的非法操作不发布前面的改动。共享coordinator.apply_operations作为唯一source算法，不另写一套delete/move规则。
- 不可变FrozenPreviewPlan以Arc原值返回，重复解析同ref得到相同原operation/transaction/newcell ID，绝不以最新draft重拼旧内容。plan hash含冻结原内容、scope与实际影响；实际writer提交UTC仍由writer生成。原源码解析错误、相关cycle/重复定义不产committable ref，diagnostic截断不能把失败藏成valid；结果有64KiB上限/影响ID1000预算。真实sin用户绑定及跨格f空格调用通过实际parser事实核对。
- ReferenceRegistry为新runtime host提供的32字节安全entropy密钥签发HMAC-SHA256 ID（test key仅自有fixture），counter目的隔离与验签，不以字符串格式当授权。严格绑定runtime/document/generation/revision/epoch/sourcehash/task lifetime/grant/config/definition/metadata/editor state与讨论/执行能力。snapshot TTL≤10min、preview≤5min，deadline相等即expired；不同issuer、非ASCII伪tag、wrong kind、scope/source/permission变化、revoked与重开runtime都拒绝。缓存≤128refs/32MiB、counter不回绕，过期释放资源，临时refs不作durable ledger。真实注册时密钥/绑定只由trusted host提供，不读模型或Transcript；本任务不提前注册CABI/Agent tool。
- 10个实际preview与3个ref测试通过，涵盖full-source/fragment、wrong kind/重复client key/晚操作/alias、语法cycle/conflict、64/65预算、原hash及permissions/mode、scope变化/expired/revoke/重开。跨语言example创建真preview→Swift物理source事务→真实回执再由原Rustauthorityaccept：原newcell IDs、源let a=5/a+1、duplicate一笔效果均通过，没有synthetic receipt冒充生产成功；fixture导出prepared_only明确。
- 最新host/ABI共54 Rust通过、Clippy、Swift actual frozen port、source/settings/fullsync失联查询、bootstrap六SIGKILL、Blob六发布点、原native ABI/actor/reducer/session、SDK27 ARM64 Release、17Python、纯WASM/deny/生成检查通过。中途raw type/primitive DTO/Arc serde与large enum错误原件保留并修正，不关闭lint。核心crates仍forbid unsafe，不增加第三方数学或3D框架；hmac0.13/sha2_ref0.11为既有lock包直接host依赖，不传入WASM数学核心，MIT/Apache许可检查通过。
- [preview.json](../acceptance/pre-alpha.4/development/r41/preview.json)保存日志/实际frozen JSON/回执和hash。原c72ce17的CI37936489563全部jobs实际success，旧失败原件不删除；不能填本批新SHA或最终candidate门禁。
- 下一任务R4.1.06真正DraftStore/IME/fence/commit admission/IOAck/取消和unknown线性化，之后undo/checkpoint/原生工作台/Agent。没有Native UI或Pi提前获得writer入口、Preview成功不表示持久化/计算/文件保存，剩余82任务全部继续原计划。

## R4开发记录 D016 — 实际原生屏障、持久化提交与原ID恢复

- R4.1.06完成：新Mac C ABI内部source union、实际SourceEndpoint/DocumentCommitController、Swift DocCommitPort和MainActor DraftStore接通真实SQLite。open恢复真实source/calculation/config_revision，预览不使用默认常量覆盖恢复配置。192 DTO及12符号同源，CABI/旧iOS协议边界明确，没有Pi写入工具或完整工作台提前注册。
- 原ID admission在实际文档库耐久记录；source/inverse/receipt/outbox/completed admission原子COMMIT。新输入/IME/过期/stop在短屏障前拒绝，屏障用try_lock不等待source解析，SQL/F_FULLFSYNC不占MainActor。真实数据库14次admission仅9次source提交，transactions/operations/outbox/completed admissions均9，取消/失败没有源码效果。
- 实际NSTextView TextKit2 marked-text回调保护中文合成，手工未完成语法可保存为source。own ACK丢失按原ID读回后仅确认原序列，较新draft保留；外部source提交后的晚到marked overlay保留且明确conflict。重复进行中的apply不排在SQL后面，终止原ID不复活；完整重复回执被改hash会拒绝。已终止记录不再持有完整source计划，临时scope资源释放但签发计数不回绕。
- 验证了admission ACK和COMMIT ACK分别丢失、原ID无重放、屏障前输入/TTL/停止/未到fence时停止、COMMIT后stop保持completed及有序close。首轮新增测试实际复现Writing→AwaitingFence和终止admission误Unknown，修复后通过；早期测试误用不存在字段及编辑编译输入导致编译失败的日志保留，没有删除断言/改数学期望/放宽原时限。
- 63 Rust host/ABI测试、相关全target Clippy/fmt、Swift实际ABI/DTO/reducer/session和提交fixture、SQLite六崩溃/ENOSPC/fullsync失联、Blob六发布中断、原source与preview回归、17Python、纯WASM/deny通过；SDK27 ARM64 Release链接/strict ad-hoc签名和全部12符号实际回读通过。本机未启动iOS模拟器，也未接真实模型/签名公证或宣称最终gate已过。
- [commit.json](../acceptance/pre-alpha.4/development/r41/commit.json)索引实际fixture DB/回执计数、源码及原日志hash。上一文档提交9cae3bc的CI37964563531全部五jobs成功，仅作该SHA证据；本批新提交的CI与最终candidate仍须独立验证。原有数学和公开`.1/.2/.3`均保留，运行版本仍`.3`，32最终门禁not_run。
- 接下来R4.1.07统一逆事务/UndoManager/近期200历史与后续编辑冲突；再R4.1.08无损完整计算checkpoint。剩余81项继续完整计划，未缩减原生UI、Agent、模型媒体、现场和发行范围。

## R4开发记录 D017 — 逆向source合并基础（R4.1.07尚未完成）

- 已实现纯Rust `undo::merge_inverse`，不恢复整本旧快照。校验原transaction/current snapshot hashes与文档/revision；原操作真正修改的内容和原cell revision须仍吻合，后来的相关修改及改后又改回相同字节拒绝。标题只在原操作改过且当前仍一致时恢复；未变邻格/后来的新增和删除保持当前内容。
- 插入/删除/移动逆向通过O(n log n)稳定顺序锚点区分每个原变化间隙。各变化间隙必须匹配原after次序；间隙内后来插入/移动及身份重用为conflict，未改间隙的后来新格和编辑保留，未被原source写入的内容仍用当前值。原最小连续区域法被有效反例证明会误拒绝两个独立变更之间的新格，已按分段合并修复。不部分应用，不反转revision或执行CAS，不产生durable receipt。
- 初始整本before恢复在6个有效用例均失败（确实丢掉后来源码/标题/新格或错误接受冲突），原件保留；12项真实source-level案例和相关全target Clippy/fmt通过，其中4个不同ID的65种顺序/成员集合两两组合4225项逆向均与原before全部字段一致，包含Unicode、ABA、插入/删除/移位、非法记录和错误文档。证据[undo-source.json](../acceptance/pre-alpha.4/development/r41/undo-source.json)。
- R4.1.07仍unchecked/in_progress：真实SQLite原transaction读取与逆向提交、原ID核对、UndoManager分组/echo、1..32 group整体操作、200完整事务保留/pin/tombstone及重启/原生/权限验收尚未接入。此纯函数不注册为Pi工具或执行writer，不把部分通过称统一撤销完成。
- R4.1.06的21b01e7已推送；同SHA CI37968923767的native_macos与dependencies实际success，其他jobs当时仍运行，不填全部CI或最终gate。继续完整81项剩余范围，无本机iOS模拟器，运行版本仍`.3`。

## R4开发记录 D018 — 实际逆事务与原生UndoManager（R4.1.07继续）

- 实际SQLite返回原plan/receipt，Rust整体校验/按新到旧逆向1..32个原事务，DocCommitPort一次source COMMIT产生undo_of和新revision。可选undo_group绑定原组/transaction IDs到同源hash，老无字段计划hash不改，不进.omnb。组内刚恢复的comparison revision只用于临时校验，owner计数不倒退；顺序/重复/后期相关修改/超限整笔拒绝。16项逆向测试含4225顺序组合、两笔同cell/插入后编辑及32上限均通过。
- 实际C ABI/SQLite验证两笔→一逆事务、重复原ID无新效果、错组同ID拒绝、redo以实际逆事务运行、later manual conflict不覆盖、lost ACK按原ID核对，以及重开host/storage后原ID读取而不重放。测试DB最终14修订/transactions/operations/outbox/completed admissions均14；原fixture_root和各输入/原件hash见[undo-native.json](../acceptance/pre-alpha.4/development/r41/undo-native.json)。
- 原生NSUndoManager等真实事务成功后才消费命令和登记redo。实际同组回执合并、相同echo去重/不同hash拒绝、pending禁用、undo/redo以及conflict后保留原命令通过。unknown禁用两方向，界面核对仍用original operation，取得实际回执后才更新native栈。首轮真实closed-group setActionName触发Cocoa exception，已修为只在初次group内设置名称，后续仅扩展事务列表；失败原件不删除。
- 79host/ABI Rust、相关Clippy/fmt、198 DTO/工程漂移、真实source/preview/commit/Swift ABI/actor/reducer/session、17Python/纯WASM/deny、SDK27 ARM64 Release与12符号/strict ad-hoc签名通过；新增CI实际undo脚本。本机没有运行iOS模拟器、旧公开资产和运行`.3`不改；本机Release仍是Preview完整工作台尚未开放。
- R4.1.07保持unchecked：完整编辑器实际文本/IME group自动映射、跨重启200完整事务/pin与裁减tombstone、早准备/close和保留故障验收尚待完成。不能把基础NSUndoManager fixture/普通回读当这些条目通过，没有Pi undo handler，也不填32最终candidate门禁。
- 上一纯合并3f8f28f的CI37971347764全部五jobs实际success；21b01e7的CI除iOS取消外各job成功，取消由后续push中断，不能称全过。当前新工作区/候选仍需同SHA验证。继续完整81项剩余，不缩减原UI、Agent、媒体与发行范围。

## R4开发记录 D019 — 历史保留、文本group与R4.1.07完成

- 实际206个Rust计划经SQLite源码提交；最近200+user/task pin保留，原操作/回执/outbox206项均不删。先整体核验再一次裁减旧full plan/inverse/admission重复source与无active/retained引用的旧snapshot，存不可执行的版本化Tombstone及byte SHA。回滚、lost ACK、重启、取消pin、原ID重复无新效果、损坏拒绝与legacy/full-only/future只读拒写全部实跑。最后counts206/206/206/201/5，完整代码和原件见[history-and-groups.json](../acceptance/pre-alpha.4/development/r41/history-and-groups.json)。
- 新文档库格式2/min-reader2；Root/selector/Library仍1，老Native文档1完整模式，无原地升级/裁减。input_group_id可选仅manual actor可用，绑定request hash和持久化元数据；原无字段散列不变。200 DTO、Swift optional init默认nil、原.omnb/数学/iOS ABI保持；格式裁决及安装来源hash已同步。
- 实际NSTextView.insertText emoji/local undo/redo、确认后durable undo/redo、中文IME更新同组/未完成不提交、group随真实source计划存储及history回读通过。原生文本引擎负责范围/选区，服务echo不入undo；EditorCommitBinding消费实际回执，没有以textarea或composing bool模拟原生输入。准备期间stop/close均按真实已知ID处理，close等待read/preparation和SQL，再消费host/storage。
- 原生undo库18笔source/receipt/outbox计数一致；再真实205笔滚动写入并裁减至200，原undo full source已移除，但重复original undo仍读原metadata/receipt，未产生新source效果。完整工作台、所有R4.2编辑交互、Agent handler、quota/backup/GC调度仍属后续任务，不以此提前勾选。
- 同批80 Rust host/ABI/16逆向+4225组合、Clippy/fmt/200生成/工程、Swift actual session/ABI/reducer/source/preview/commit/Blob/SQLite六崩溃、17Python/纯WASM/deny和SDK27 ARM64 Release/strict ad-hoc签名/12符号通过；本机没有iOS模拟器。32最终candidate门禁仍not_run，运行`.3`、公开旧标签/资产均保留。
- 前置bbdc2b4 CI37979609572实际native session exact-0 unknownOutcome，其余四jobs成功。原附件已下载，根因及确定性先失败后修复见D015：接纳前snapshot在ACK后返回，按dispatch时已接纳集合判定并重读原ID，stale group不丢。真实CABI barriers修复后通过，原12丢所有terminal的结果仍正确，未放宽数学/性能标准或隐藏原失败。
- R4.1.07的Done已满足，当前18/98；下一任务R4.1.08无损完整可写checkpoint，包括definitions/rules/attrs/Out/history/random/owners/precision与格式/资源验证，再R4.1.09候选接纳。完整80剩余任务及发行范围继续，不把开发任务完成当`.4`可发布。

## R4开发记录 D020 — 可写Evaluator候选基础（R4.1.08未完成）

- 审计真实Evaluator：defs own/down/attrs/changed、history/Out、settings和SplitMix64是持续数学状态；depth/evaluating/scopes为瞬时解释器状态，当前statement messages/solver/science evidence由kernel消费。新WorkingEvaluator在空闲可写边界克隆真实持续状态、拥有独立可写Evaluator，readonly/inflight lexical捕获拒绝；丢弃候选释放全部未接纳定义/history/random，consume本身不授予文档权限或称耐久。
- 5 state测试实际验证用户downvalues/Listable、Out、候选a=5与owner a=2隔离、随机流独立且消费后继续、精确大有理数无机器投影、设置分离与不可提升readonly；非空闲depth/lexical边界拒绝。先缺API失败原件保留。om-eval完整191项（含原求解语料）、相关全target Clippy/fmt和纯WASM通过。
- R4.1.08仍unchecked/in_progress：Expr/Number无损有界codec、整个Session的source/owners/settings/输出与checkpoint registry、Root/model/interpolation/高精度往返、未知版本/节点/深度/容量/取消验证尚未实现。未序列化指针/闭包，不使用readonly restore假冒主文档可写状态，不靠重跑let恢复；本候选未接入主kernel接纳/持久化。
- [working-stage.json](../acceptance/pre-alpha.4/development/r41/working-stage.json)记录具体源码与日志hash，下一步按R4.1.08全范围实施。18/98完成、80待完、32最终gate仍not_run，运行`.3`；无本机iOS模拟器或旧公开资产变化。

## R4开发记录 D021 — 有界无损数值原子和表达式图（R4.1.08继续）

- OMNU1二进制存canonical整数/约分有理数、机器u64位型、BigFloat原significand/exponent/bit precision及有限scalar complex。机器signed zero/subnormal/极值和高精度signed zero保留；未知版本/长度/trailing/precision/平台exponent及cancel拒绝。新增实际反例暴露raw zero的特殊指数可变成非有限值，已在构造/归一化前拒绝sentinel与偶数mantissa，原失败保留。4096固定seed机器样本逐位往返。
- OMEX1保存真实immutable共享图，进程内allocation key只在encoder去重，不序列化pointer，不用数学相等合并+0/-0；任意head/顺序/原UTF8字符串/symbol preserved。整个图的bounds/版本/tags/known builtin/backward references/depth/edges/roots/可达性先校验，随后才intern和构造，不执行CAS。100层共享重复保持线性，无2^100展开；孤立/循环/forward/wrong builtin/坏UTF8和512固定seed恶意短图拒绝，32bit引用计数溢出先检查。
- 数值5/图6 codec用例，num/core完整回归、相关Clippy/fmt、原working-stage及纯WASM通过；限制和具体日志/源码hash见[codec-atoms-graph.json](../acceptance/pre-alpha.4/development/r41/codec-atoms-graph.json)、[编码契约](../design/checkpoint-codec.md)。旧math53/时限不改，核心继续forbid unsafe，没有新第三方依赖或主文档恢复入口。
- R4.1.08仍unchecked，完整Evaluator/Session/checkpoint codec、稳定callback/build/catalog绑定、owners/输出与原Root/model/interpolation/高精度的主状态往返及耐久候选接纳待完。不把原子/图codec或owned clone当整个会话可恢复，剩余80任务继续原范围；本机不运行iOS模拟器，版本仍`.3`。

- 本批观察前置5bf2a20的CI37986742459已结束：Rust/native_macos/dependencies成功，frontend与iOS失败。前端是1280设置CRUD保存status5s未找到，下载error-context时真实status已显示AI settings saved，根因尚待定位；iOS是explore旋转用例expression.16未出现（slider存在），原1s/数学标准未改。失败日志及前端原artifact保存target/acceptance/pre-alpha.4/development/r41/codec-prior-ci-failed.log与target/ci-evidence/codec-5bf2a20-frontend，不能将这些job或当前新SHA标全绿。继续codec并独立定位展示/时序问题，不开本机模拟器、不删除断言。

## R4开发记录 D022 — 实际Evaluator数据恢复与owned Session工作副本（R4.1.08继续）

- OMES1持续math codec真实编码own/down/ordered delayed rules/attrs/changed/history/Out/settings/SplitMix64，shared raw OMEX图保留exact/high precision/机器位型，build/crate/metadata_version/actual callback registry匹配后才构造新可写Evaluator。unknown字段/重复身份/root引用/顺序/packet length、readonly/非空闲与cancel拒绝；延迟RHS counter不执行，history不是重跑文本，recapture byte-stable。
- 实际Root、80位decimal、interpolation、fit和ODE存储值及后续调用对照原Evaluator通过。内部真实attrs表先复现OneIdentity bit8被旧mask255拒绝，再修到511；bit9仍拒绝，机器负零与非默认设置逐项保留。早期SetAttributes held调用不能证明user attrs，已移除该无效验证并补真实HOLD_FIRST行为克隆/候选改动隔离；属性不是新增数学API别名。
- owned WorkingSession克隆真Notebook/Cell/StatementRecord/steps/owners与readonly Explore snapshot，主definition a=2依赖3，candidate a=5依赖6且主投影不变；candidate消费后真实定义保持。新token必须不同于parent Arc，parent已cancel仍不影响新candidate，运行后的token不会内部reset；profiles/API key/routes/LLM jobs/config_store不复制。原solver输出为Solutions/取消为实际Error+interrupted message，初始新测试误认Expr/空items，已按原协议校准并保留失败，无原数学期望或阈值变化。
- om-eval完整201项、portable kernel204项（含原scene/solve/探索语料）、7codec/3working Session、实际attribute/machine内部用例、Clippy/fmt/纯WASM及native feature build通过；源码/原件hash见[evaluator-state-codec.json](../acceptance/pre-alpha.4/development/r41/evaluator-state-codec.json)。核心仍forbid unsafe，无新依赖，无本机模拟器/真实模型或新公开包。
- 6315272的CI38006779929五jobs全部success，仅属于前置原子/图SHA；之前5bf2a20 frontend/iOS失败原件仍保留，不将这些数据替代当前新SHA或final candidate。
- R4.1.08保持unchecked/in_progress：完整Session checkpoint bytes、source/config/owner/result/record provenance/build binding、结构化solver/steps/explore持久化及host checkpoint identity/Blob与接纳验证继续全原范围。当前18/98、80待完、32final gate not_run；不把Evaluator恢复或memory clone称整个会话可恢复。

## R4开发记录 D023 — 整个Session、真实步骤与有界临时kernel状态（R4.1.08完成）

- OMKS1保存实际source/status/defines/uses/exec_count/output、owner/general/serial与全部StatementRecord；OMES完整数学与OMRS只读Explore另分段。构造owned writable候选不执行源式，原producer descriptor与当前已核验source/general分别比对；新操作token/clock由host提供，不持有旧AI/credential/config-store IO。
- om-solve保存全部27种StepKind与原SolutionSet的条件/参数域/重数/Verification/numeric位型，steps树引用及rule/ID/层级逐项验证。跨graph exact comparison保留数值类别/precision/±0并memoize实际节点对；不展开共享DAG。OMRS header+metadata role拒绝仅改magic的提升，原确定性失败log保留。
- KernelStatePool保留真实byte/hash/source/binding，32/64MiB、4096 lifetime；撤销lookup后被caller pin的字节继续计费，实际release才reap。不发耐久回执、不创建active指针或Agent权限；真实Blob/accepted记录属于R4.1.09。
- 原53实际Kernel输入→bytes→owned恢复→原输出/步骤/数值残差→byte recapture通过；原期待未改。实际Root/80位decimal/fit/ODE/interpolation后续调用、随机/Out、readonly Explore、延迟RHS不重跑通过。未知版本/角色/嵌套字段/owner/history/provenance/引用、trailing/容量/深度/取消拒绝，不返回partial success。
- 最终相关六crate回归723passed/1原ignored/0failed（整次含4Session用例），随后新增不重跑延迟RHS用例的5Session目标测试通过。原steps4、readonly8、host pool2通过；Clippy/fmt/纯WASM/deny、Python17及计划/200DTO/drift验证通过。原Release200ms/1s和32final gate本批未执行，不把debug 53往返写成性能pass。
- 全部源码/log hash、完整命令与范围见[session-checkpoint.json](../acceptance/pre-alpha.4/development/r41/session-checkpoint.json)。上一de0ec9e的CI38057397556五jobs全success，只证明前置SHA。R4.1.08 Done满足，当前19/98；下一任务R4.1.09真实主worker/非执行对齐/存储接纳，余79任务继续完整范围。版本仍`.3`、final candidate未分配，无本机iOS模拟器/旧公开资产变化。

## R4开发记录 D024 — 主CAS线程、非执行对齐与真实候选（R4.1.09继续）

- 独立KernelWorker每job pin显式parent、以新clock/token恢复owned Session，先非执行同步current source/settings/owner失效再原run_cell单格真实CAS；注册池的编码移到短锁外，worker没有自己的active指针。等待队列2、lifetime admission4096、fresh token/duplicate identity、direct stop、后台join与丢失receiver的candidate回收有明确边界。
- a2→5对齐清理旧a/b，不自动cascade；未运行a前b拒绝DEPENDENCY_NOT_READY，显式新parent后得到b6。标题改动不退还定义，跳过多个math epoch或改后恢复相同source则保守退还所有owned Math定义，history/random不重跑。非reactive重新赋值按原语义，reactive真实依赖/cycle/conflict检查。
- 实际现代多语句cancel前成功history1与赋值7仅在candidate；Wolfram失败CompoundExpression有先前赋值7而无成功history。新的测试最初把held singular inverse误当throw，再误把semicolon子式计为history，保留全部诊断与失败logs，按真实原Session语义纠正新fixture，不改既有数学期望。错scope/epoch/hash、cancel-after-freeze、queue/pool满、token复用、close以及拒绝candidate的Out/random隔离均实测。
- 最终8新worker case＋2池case在相关三crate完整回归中全部通过：301passed/0failed/1原ignored。Clippy/fmt/纯WASM/200DTO与计划检查通过；源码与日志hash/实际命令见[main-worker.json](../acceptance/pre-alpha.4/development/r41/main-worker.json)。55d11b8整Session提交的CI38061268087五jobs全success，只证明前置SHA，非本worker或final候选。
- R4.1.09保持unchecked/in_progress，actual NativeHost/Swift main calculation注册、同一source逻辑门与accept/cancel屏障、真实Blob/accepted SQLite/active head/operation/outbox及原ID耐久回读尚待接通；临时candidate不是已接纳主状态，也不是可用工作台。当前19/98、79任务待完、32final gate not_run，运行`.3`，无本机模拟器/旧公开资产/第三方新依赖/unsafe变化。

## R4开发记录 D025 — 冻结生产事实、同库耐久接纳与实际中断（R4.1.09继续）

- FrozenKernelPlan/AcceptedKernelState定义真实bootstrap/candidate/原receipt边界，原producer source/cell revisions及current acceptance source独立冻结；title能在同epoch接受，stale source/config/lifetime/parent拒绝。新KernelLifecycle使worker cancel和enter同短锁，64race确认先后；合成receipt unit不当物理证明。5契约/恢复case、8原worker和2新lifecycle通过。
- 新document3/minreader3/codec3支持实际accepted checkpoint/result索引/operation/transition/outbox/head及Blob refs在同SQLite transaction提交；WAL F_FULLFSYNC/独立回读真实成功才ACK，sync/lateACK保持unknown按原ID确认。UTF8 framed hash与新5DTO（205总）Rust/Swift一致，unknown/shape/provenance拒绝。source历史裁减保留checkpoint引用，旧native1/2无自动upgrade、kernel接口明确unsupported，future4只读拒绝；旧Tauri/公开资产不动。
- 实际Rust source bootstrap→Swift original bytes/DB receipt→Rust data-only恢复→next真实a2/b3/failed partial7链及exact reopen bytes通过。旧parent、原ID重复/冲突、SQL rollback/owner barrier拒绝、lostACK、actual F_FULLFSYNC失败后stable reconfirm、outbox/Blob损坏拒绝均实测。5actual SIGKILL保留previous parent或exact accepted candidate，Blob refs/head/receipt/outbox不拆开提交。
- 本地host+ABI97Rust/0fail、Python17、Clippy/fmt/205DTO/工程与计划检查、Release actual app链接成功。原physical source+fullsync、206history/200+pin、Blob6SIGKILL、actual native UndoManager及CABI/MainActor/SQLite CommitPort回归通过，完整输入/命令/log/hash见[kernel-acceptance.json](../acceptance/pre-alpha.4/development/r41/kernel-acceptance.json)。无本机iOS模拟器；final候选未分配/32gate未跑。
- 3ce0320的CI38067733784：Rust/dependencies/native Mac success，frontend settings.spec.ts117 save status缺失（afterEach有真实已存配置/状态），iPhone原row17整次1139.849ms超原1000ms导致failure；原日志/xcresult/Web截图已下载target/ci-evidence/r4-worker-38067733784。不改期望/计时、不算全CI通过；先确认实际transport分段/保存响应时序再修，旧失败保留。
- R4.1.09仍unchecked/in_progress：NativeHost/CABI/Swift main运行endpoint、真实同文档shared logical gate/最终scope屏障、UI/Agent accepted结果事件及部分已接纳前缀路由待完；physical fixture不代表完整工作台。当前19/98、79剩余、运行`.3`，继续全部原计划。

## R4开发记录 D026 — 实际主运行/CABI/共享source门（R4.1.09完成）

- 13号Mac ABI及NativeKernelEndpoint进入SourceEndpoint同一确认owner；每job从实际AcceptedKernelState parent选择、source/定义拥有者非执行对齐、真实CAS后台执行。已冻结candidate才保留共享source逻辑门，idle/control/draft/direct stop不等CAS；原physical写/unknown不可被另一source或kernel写绕过。新增actual execution_context读取defs/owners给source parser，额外bytes计入有界pool。
- Swift KernelCommitPort完成实际source同步、admit、chunk byte/hash→Blob→同库COMMIT/actual stable receipt→Rust active revision/单次result event；no-COMMIT须actual writer已settled、healthy original absence及source/parent不变且sync成功。error/stop/stale保留逐格actual accepted prefix，未接纳defs/history/random不进入next job；closing停止新admit，cancel/token与final barrier共短锁，join脱离source/UI锁。
- actual main fixture a2→3/a5→6/a9→10、prerequisite拒绝、production后title与source revisions分别保存、重复原ID不重播event、physical IO时输入draft保留、early stop rollback/late stop committed prefix/lostACK、actual F_FULLFSYNC unknown挡新任务及原ID稳定回读、长CAS期间source read/edit<1s/ephemeral未接纳definition no-leak及运行中close全部通过。Kernel/host/ABI完整相关310passed/0failed/1原ignored，原53 checkpoint/math expectations不改。
- Native229DTO/13symbols、Clippy/fmt/纯WASM/deny/Python17、actual storage5SIGKILL与原SourceCommitPort/MainActor/NSTextView/UndoManager回归及Release完整链接通过。[kernel-runtime.json](../acceptance/pre-alpha.4/development/r41/kernel-runtime.json)保存全部命令/源/log/hash。新build.rs只加同锁sha2构建依赖，实际source/schema/catalog/ABI/target/profile编译指纹；没有新math runtime/core unsafe/旧iOS ABI变化。
- 最初generator annotation x-rust-boxed-variants被原strict schema validator拒绝，state RPC确定性失败后改为既有x-openmath-contract annotation，JSON/Swift形状及unknown拒绝保持。fixture初始签名/async lock、编译中修改输入失败均保留，最终冻结编译后完整runtime通过，不算失败attempt为pass。
- b309baf的CI38085995468 Rust/dependencies/frontend/nativeMac成功，仅iPhone原row34整次1116.914625ms超原1000ms失败，原log保留；上一3ce row17已离线解析kernel205ms/decode2ms/resume933ms，仍需mainActor占用根因，未放宽阈值/本机不跑iOS模拟器。不以这些前置CI替新SHA或final同candidate门禁。
- R4.1.09 Done/Tests满足，当前20/98、78待完；下一R4.1.10 immutable result store/readonly inspections，再NSDocument/完整启动恢复/工作台/Agent/模型媒体/现场和发行全部范围。当前仍`.3`、32final gate not_run/candidate未分配，不把main backend完成当`.4`可发布。

## R4开发记录 D027 — 原值/步骤/几何与只读历史基础（R4.1.10继续）

- 原Session实际retained cell/statement/value/steps/science/solver/Explore/已采样output和readonly context独立冻结，原input/out/view/suppression/type/nature/producer保持，不重播source。值/步骤/几何按原route分页，numeric直接原Expr；完整source显式读取，formatter先检查200000expanded节点。原2D/3D/Explore buffer分页逐值相同，无重采样/前端triangulation。
- Host ResultStore只从AcceptedKernelState/匹配receipt读取原OMKS data；HMAC result refs可跨producer source/definition修订以history readonly访问，但runtime/document/generation/task/grants/metadata/read permission/10min严格保持。freshness要求actual current文档/occurrence/status/epoch/config，不能把同ordinal或别的文档当current；retired pins继续占32/64MiB cache reservation到actual释放，不冒称RSS限额。
- Set/SeedRandom/hidden delayed function write被真实readonly拒绝，held warning/error不是成功；random draw在fresh本地fork保持可复现、不推进原random/Out/history。scratch从原accepted bytes恢复owned未接纳临时副本，reactive=false且不超5000ms，无主发布路径。原owned单格definition重执行补actual affected依赖stale/owned退还，旧b3能history查看，但新main b不沿用；原public Evaluate语义不变。
- Kernel5/Host4case、完整相关三crate319passed/0failed/1原ignored，最后文档身份修订目标4case与Clippy再通过；纯WASM/Clippy/fmt/Python17/计划检查、actual kernel CABI/main/IO/关闭回归通过。Rust ResultStore额外从actual Swift SQLite receipt+original Blob恢复a2/b3/failed partial7，actual同库/5SIGKILL回归保持，不拿unit synthetic receipt充physical证明。原输入/数学期望/阈值未动。全部source/log/命令hash见[result-store-foundation.json](../acceptance/pre-alpha.4/development/r41/result-store-foundation.json)，设计见[result-store.md](../design/result-store.md)。
- 5469091的CI38089837352五jobs全部success，先前两个手机timing/Web失败原件保留；只证明上一main runtime SHA，非本批或final candidate。
- R4.1.10仍unchecked/in_progress：NativeHost结果后台/只读注册通道、普通事件summary/ref、显式完整source分块与late reply隔离、Swift ResultClient和跨语言inspection待完。当前20/98、78任务剩余、运行`.3`，32final gate未跑，不启动本机iOS模拟器，无新依赖/unsafe。

## R4开发记录 D028 — 实际结果worker/CABI/Swift交付（R4.1.10完成）

- ResultRuntime独立后台接actual accepted snapshot，SourceEndpoint只admit/check actual scope；14号Mac CABI/263DTO及Swift ResultTransport/ResultClient接通summary/ref、exact/value page、presentation/source/numeric/steps/geometry/readonly/scratch。presentation复制原SolutionSetView条件/verified/Root/intervals，error-only保留actual messages/spans，不推测证明或重新solve。large data不进ordinary main events，raw source按UTF8边界/同total+SHA完整交付，partial/stale不返回完整copy。
- HMAC ref/原request状态继续严格read grant/runtime/doc/gen/task；actual current source/kernel修订与selection generation双检查拒绝迟到reply。4waiting/32pending/4096IDs/32terminal详情、5s/4Mtick/普通256KiB、source8MiB/单片64KiB（Swift16KiB）有明确上限，reply过大/expired/unsupported/timeout不是空成功。record.detached不pin main旧envelope，cache只驱逐older unpinned data、durable Blob/receipts保留，旧refs明确expired，不重播source。
- 请求身份跨source/main/scratch/read检查，不能shadow direct cancel。真实worker pause验证status/stop不等inspection，scope变更不能用已知request ID读别的doc/grant。close先停admission/cancel，thread由后台join。trusted fixture delivery probe只观察actual decoded reply，无model/serialized入口。
- actual C ABI＋main/SQLite receipt＋result worker＋Swift验收exact/80digit、长中文/emoji/NFD source逐片byte/hash核对、原solution verified/steps/2D3D samples、readonly Set/random/Out/scratch隔离、source变化后old a2历史、parse error零history、selection与main修订改变后迟到reply丢弃、events<32KiB不含long source与revocation全部通过。kernel/main/storage/5SIGKILL回归和Release完整链接通过，相关三crate324passed/0failed/1原ignored；Clippy/fmt/纯WASM/deny/Python17/263DTO/工程与计划检查通过。
- 第一轮fixture的throwing precondition compile失败、Message大variant/len_zero Clippy失败保留，修实际fixture/Box布局/断言后再验；无数学期望或30s/1s门槛调整。[result-runtime.json](../acceptance/pre-alpha.4/development/r41/result-runtime.json)保存输入/命令/原输出/source/log hash，设计[result-store.md](../design/result-store.md)同步真实边界。
- 6d53f17的CI38092397694 Rust/dependencies/nativeMac success；frontend旧完整西瓜.scene-view30s未见、iPhone原row17整次1056.760708ms超原1000ms失败，原log保留，不把前置CI当全success/新SHA/final candidate；上一5469091全CI成功原件仍在。后续门禁继续根因定位，不放宽、不开本机iOS模拟器。
- R4.1.10 Done/Tests满足，当前21/98、77待完；下一R4.1.11文件快照/NSDocument再恢复/资源管理/全工作台/Agent/模型媒体/现场及发行范围。运行`.3`、32final gate not_run/candidate未分配，结果API完成不代表完整`.4`可发布。


## R4开发记录 D029 — 原生文件保存基础（R4.1.11实施中）

- NativeDocument/FilePort/FileService/SaveStore接通实际SourceSnapshot、不可变Blob、同库intent/receipt/outbox和用户授权文件。新文档格式4读取已有Native1–3，不自动升级；Root/Library1、公开`.omnb`v1、Rust数学及iOS ABI保持。
- 实际保存/N+1/另存为旧回执/外部修改/只读拒绝/IME/同步未知与原ID重启核对通过；应用工厂共享root lease、打开不执行CAS、实际TextKit输入2秒自动保存和UndoManager、后续输入撤销关闭决定通过。6个真正SIGKILL边界与额外被交换外部原件冲突通过，原件保留且不伪造receipt。
- 已运行test-storage、test-history、test-kernel-store、test-commit-port回归；原SQLite16次真实fullsync、206历史/200+pin、原OMKS/5kernel SIGKILL与跨语言编辑屏障均通过。Release构建通过，CI新增test-saves；本轮未启动本机iOS模拟器。
- 初始授权/写入同队列死锁、权限拒绝误作unknown、Swift签名/异步autoclosure错误及测试抢先于receipt判断成功的失败保留并修复。系统保存面板异常禁用仍待修；原生新建/输入/关闭保护不等于全部文件UI验收，R4.1.11未完成，计数仍21/98。
- 前批7b470cc/CI38094998965五jobs全success；不能代替本批/final候选。运行`.3`、32final gate not_run/candidate未分配，下一步先完成面板闭环，再R4.1.12恢复与R4.1.13资源管理。实现见[native-file-saves.md](../design/native-file-saves.md)。

- D029补验：应用会话editor索引改UTF8 Data，真实NFC/NFD两个cell分别保持独立NSTextView；原save/SIGKILL再次通过。f04474f已推dev，CI38100274832运行中，尚未填success。最终factory配置实际面板仍禁用，问题保持未解决。


## R4开发记录 D030 — 文件身份与面板证据纠正

- 用户确认Save可点击，独立AppKit panel返回OK并写出真实.omnb。实际OpenMath通过系统面板保存与Cmd-S编辑保存，字节/源码仅v1，SHA与原SQLite回执一致。此前瞬时辅助功能禁用误判已纠正，任务还需完整另存为/重新打开，仍21/98，不把人工可用性口述当全部流程pass。
- 同目录rename旧路径被新文件占用时真实bookmark可返回旧URL，新增原device/inode核验和授权目录内有界定位；更新receipt中的真实URL，不自动扩大移动后的授权。真实旧路径保护/外移新绑定/缺失不重建/negative revision损坏及6SIGKILL+交换原件冲突通过。
- 原file bookmark.withSecurityScope在非沙盒fixture失败，改为独立身份bookmark并保留目录grant；记录实际失败与DB原件。原件清理后恢复分支修复为只读literal bound核对，正常write仍拒绝错inode。首次SQL故障fixture误跑MainActor的失败已修，守住后台实际数据库规则。
- 22d64de CI38100465269五jobs已全部success。当前新代码尚待本批CI，运行.3/final gate not_run/no candidate；不启动本机iOS模拟器，不合main，不改旧公开包。

- D030补验：取消关闭恢复正常自动保存，原failure pause不自动复活，两路径经真实SQLite save intent/receipt验证。新完整test-saves与Release构建通过；[文件身份与面板证据](../acceptance/pre-alpha.4/development/r41/file-identity-and-panels.json)记录源码、命令、原件hash与待完成的UI流程。

## R4开发记录 D031 — 文件面板闭环与会话刷新竞争

- R4.1.11 Done满足，22/98已完成。早一构建实际Save As与用户确认单独记录；本轮Release系统Open取消/Open/Save As取消/新文件保存/关闭/Cmd-O重开/窗口关闭退出通过。三份自有合成文件SHA一致，新Save As receipt/outbox/source hash实际相符；重开独立source0/零accepted checkpoint，两文档clean shutdown均1。[完整证据](../acceptance/pre-alpha.4/development/r41/file-saves-completion.json)。
- 系统panel回调先orderOut；整个Open面板保留busy，普通保存/自动保存不同URL拒绝；取消关闭里的保存面板恢复原policy。真实文件fixture保留原数学语法错/N+1/旧binding/IME/身份/未知同步，6SIGKILL+原件冲突通过；test-saves和Release构建exit0。
- c4eab5a/CI38119321361 Rust、Web、依赖、iOS通过，native_macos在SessionFixtures第17行失败。修复显式刷新在已有readback时直接返回的问题；新固定并发fixture、12精确4/丢帧恢复/停止/并发关闭以及test-native-contracts完整exit0。保留原artifact失败，本批CI待推送后回读。
- 下一任务R4.1.12同文档outbox投影Library、未确认/IME草稿、启动按原ID核对与接纳前缀恢复；不自动重放操作或付费请求。R4.2完整工作台、Agent和发行仍未完成；运行.3/dev/no main/no本机iOS模拟器，32final gate not_run/no candidate。

## R4开发记录 D032 — 原文档事实投影与 Library 故障隔离

- R4.1.12实施中，仅Step1完成。Source/kernel/save三份原outbox逐条验证后写Library2不可变事件，按store/event去重；Library实际同步/回读后才标原库projected。保存投影不含路径/bookmark。最大16条/4 MiB原载荷、分批恢复游标不重复卡在第一页；首次和重启不创建 evaluator/HTTP。
- 真实TextKit→源码事务、四次CAS输出2/3/5/6、保存生成7事实（包括初始化检查点）；回滚、丢ACK、实际全同步未知、伪造投影拒绝、跨文档相同event、完整记录损坏和健康文档另存为通过。两库五个真正SIGKILL点，重复恢复不增加原事实，原源码/初始化＋四计算检查点/文件散列不变。新freshLibrary版本2，原Native1不自动升级；Root1/文档4/.omnb v1不变。
- SDK只读VFS在WAL不存在时实际O_RDONLY/no-follow打开失败ENOENT；新增仅在根租约/串行队列与无WAL/journal时短期不可变读取，前后核对文件与侧文件。现存日志仍正常读取，不删除WAL或绕过恢复。独立文件变更守卫和原future版本不写回拒绝通过；初期实际失败/诊断保留。
- test-recovery（5 SIGKILL）、storage（17真实同步＋6初始化中断）、saves（6保存中断）、history（206事务/200+pin）、kernel-store（5接纳中断）、commit-port和Release构建实际exit0。[证据](../acceptance/pre-alpha.4/development/r41/library-projection.json)与[实现](../design/native-recovery.md)。当前22/98，不把投影基础勾成整个恢复完成；接下来草稿/IME持久化与完整启动恢复，再R4.1.13资源/升级。
- 25df8d8/CI38122270416 Rust/Web/依赖/Mac全部success，phone成功；iPad仅探索UI端点失败，实际slider3.96/正确结果15.6816。测试补真实thumb越过轨道的动作并明确要求a4＋原16，原数学/1秒门槛未改；SDK27 Swift6类型检查通过，执行留新同SHA CI，不在本机启动模拟器。初次类型检查漏XCTest Swift overlay/运算符空格错误已修，日志保留。
- 运行.3/dev/no main/旧公开资产保留，32final gate仍not_run、candidate未分配；canonical Agent/admission/network、完整工作台及发行不由本批替代。

## R4开发记录 D033 — 原生草稿耐久备份与输入保护

- R4.1.12仍in_progress、总22/98。新文档5在selector之前建立草稿schema，旧1–4不自动升级；Root1/Library2/.omnb v1/iOS ABI不变。运行实例取得新lease，晚旧owner拒绝；原序列/hash回读解决unknown，不另起ID重写。
- 真实TextKit文本/IME/UTF16选区冻结为UTF8原字节，500ms/最大2s后台编码/散列/SQLite耐久确认；旧ACK仅确认旧persistence revision，新输入保持未备份。备份和文件保存状态分别显示，关闭最终flush之后停止producer；失败不会提前关闭/停止备份。
- 重启的DraftStore/TextKit保留原文与选区，没有setMarkedText或自动源码提交；待确认/冲突/删除格草稿分别保留。实际NFC/NFD、emoji代理对拒绝、新owner、晚输入、全同步unknown原sequence1、超过4MiB的8MiB编码读回和三个真正SIGKILL通过，原source revision0/无计算保持。[机器证据](../acceptance/pre-alpha.4/development/r41/native-drafts.json)。
- 最近两份完整payload＋原receipt/tombstone，历史裁减保护draft引用的source0。原206/200+pin、文件6中断、检查点5中断、编辑屏障、两库5中断以及Release构建均通过。两个旧schema4断言更新为实际fresh5；新增baseline保留证明，原数学/206事务期望不改；初期编译变量重名失败保留。
- 47ccfb0/CI38142168036 Mac/Rust/Web/依赖成功，phone成功，iPad第27条总3556.718ms失败；分段FFI30.942ms、decode2.668ms、queue1.104ms、encoded0.109ms、resume3521.889ms。前批探索端点UI已通过。本批未动1秒门槛/数学，没有本机模拟器；主线程恢复阻塞仍需诊断，新SHA CI独立回读。
- 下一步完整启动扫描、用户恢复选择、source/overlay/已接纳prefix/原save intent与unknown核对，300ms非IME提交及canonical Agent会话/admission/network后续继续；不能把底层草稿往返当完整应用恢复Done。

## R4开发记录 D034 — 本地文档启动恢复与原检查点导入

- R4.1.12 Step2完成，整体仍in_progress、22/98。只读扫描当前根active selector，每页32/目录10000，不扫描用户文件/执行源码/猜未选代次；原生恢复List显示草稿、dirty、损坏和旧格式原因，由用户选择恢复document/store并分配新runtime/lease。旧格式1–4不隐式升级，计划R4.1.13处理。
- 新host内部restore start/write/finish，分块<=65536并连续/重复验证、总metadata16MiB+Blob64MiB，保持2MiB控制上限/14 C ABI符号/旧iOS。后台OMKR1→原plan/receipt/OMKS校验后才接入实际worker，原kernel revision不增，数据库不制造新接纳；配置修订不符不标definitions_current。直接取消和失败释放共享门，Swift核对原终态后交还编辑。
- 五个中断根重复投影后，通过实际会话工厂导入原checkpoint无重跑；随后明确b-only用恢复a5得到6并只增一计算。原Blob损坏拒绝，源码和历史receipt保留；尚无原文档“不兼容head重置”接口，UI提供真实另存为/新文档路径，不声称能直接重跑旧head。
- 三draft SIGKILL改为NativeDocumentSession恢复，原lease1是否提交与新runtime备份分别核对，真实文本/待确认/无系统IME保留；六save SIGKILL也改实际会话工厂，按原intent/授权字节补receipt或确定not_written，不replace。Library ready时有界投影，未完保留cursor/继续按钮；规范Agent admission/发送中断仍待后续账本。
- 实际SDK27 Release界面完成新建/中文emoji/本地备份→保留修改关闭→Quit/重启→恢复原文→使用草稿→系统Save。恢复时source0/checkpoints0，确认保存后source1/仍zero；实际文件SHA9febeb69…与原SQLite receipt/outbox相同，正常退出clean=1。[完整证据](../acceptance/pre-alpha.4/development/r41/startup-recovery.json)区分观察时UI与末次提示清除/Library接入构建，旧AX菜单失效记录保留，当前原生sheet/List动作已验证。
- 111 host/ABI Rust、269 DTO、4导入定向负例、Clippy/fmt、native contracts、恢复/草稿/保存/编辑实际回归及Release最终exit0。新UInt32字节转换/Abort映射、Swiftthrowing表达式/Serial.value、原vs新草稿batch计数错误修正，原数学/1秒期望不改；失败日志不删除。
- 642cfc9 CI38145308816五jobs全部success（含iPad），不抹掉47ccfb0的resume3.52s历史失败，也不替本批或final gate。本机无iOS模拟器。下一规范会话/tool admission/context dispatch的恢复状态与原操作核对，再R4.1.13备份/升级/GC及完整工作台；运行.3/dev/no main/旧公开包保留，32final gate not_run/no candidate。

## R4开发记录 D035 — 规范会话、接纳及发送恢复账本

- R4.1.12仍in_progress、22/98。freshLibrary3建立session/event/source序号和operation claim，旧Library1/2不自动升级。源码、文档5/.omnb v1不变；原schema60个actual event/required-null全部校验。请求/回执Hash、完整记录/序号缺口拒绝、原IDunknown核对、全库跨会话未完成claim保护通过。
- Tool callback必须在admitted+started实际耐久确认以后；已有原effect只读核对，重复旧claim不执行。Recovery不持有callback，原op/store/frozen参数检查之后补actual result。源码真实Rust计划→SQLite原回执，kernel真实a5/b6与失败CompoundExpression仍partial7，typed partial不伪成功/回滚。
- Context request正文/实际响应原Blob与事件同库refs，planned→dispatch→response→finished/failed；真实URLSession回环HTTP200/503字节和13SIGKILL（包括source COMMIT、结果ack、HTTP发送/接收）通过，重复恢复source操作数/服务器POST数不增加。没有使用付费供应商或旧凭据，不能将回环请求数声称实测账单。
- 真实F_FULLFSYNC unknown直到原事件读取确认、新事件不能占未确认序号；伪造结果/部分流工具参数/错误hash拒绝。Library完整坏记录只读拒绝，原件和健康source保留。[机器证据](../acceptance/pre-alpha.4/development/r41/canonical-session-ledger.json)。当前单次语义扫描4096/16MiB，超限incomplete并挡新admission；长会话索引/续页与native session恢复入口仍待完成，不勾整个任务。
- 112 host/ABI Rust、Clippy/fmt、269 DTO、canonical ledger13中断及原19同步/6初始化、投影5中断、draft3、save6+conflict、真实kernel partial、Release实际通过。Swiftshadow/throwing属性、编译中源文件变化与错误statement-count假设修正，日志保留。原schema无改变；现有Rust validator补date-time结构，独立timestamp正负例通过，无第三方Python/runtime依赖。
- 5b96deb/CI38159289992四jobssuccess，phone原53 row17总1085.738ms失败：FFI290.465ms/encoded0.106/queue0.024/decode2.273/resume792.857；iPad未运行。保持原1秒和数学，artifact已下载，不抹掉失败/不把kernel时间替代整次。下批同SHA CI独立验，no本机模拟器/dev/no main/运行.3/final32gate not_run/no candidate。

## R4开发记录 D036 — 长会话持久索引与原生会话查询入口

- R4.1.12 Done/Tests满足，23/98；下一R4.1.13。freshLibrary4增同事务event links、tool/request索引，旧1–3不隐式升级，Root1/doc5/.omnb v1不变。admission查询相关原事件，旧4096全量扫描不再阻新接纳；索引/事件/调用/plan计数、父子/身份/hash重新检查，缺指针不解释为“未执行”。
- 实际4374事件/34,897,624字节原文，37工具＋37模型计划，每页5项全部唯一恢复，新admission在4096后仍成功，原prefix与重启后查询保持。[完整证据](../acceptance/pre-alpha.4/development/r41/indexed-session-recovery.json)。游标绑session/store/generation/创建上界，工具与请求独立推进；实际删除索引拒绝，而非隐藏原call。
- 未打开的原managed文档仅在active selector存在时检查/临时打开，按storeID原op核对且投影先于writer关闭，不创造猜测目标、不执行回调。close失败保留writer注册以便核对。Native原生List/HSplitView显示not-started/unknown/完成/partial及未发送/发送未知/响应中断/完成失败，继续页可操作，不提供执行/重发按钮。
- 实际production组件+隔离fixture根的UI验收完成：2原事件，点击“查看与核对”显示call-edit“尚未接纳，未执行”和context-crash“尚未发送”，原source/HTTP均0，正常Quit。使用同一NativeFileWorkspace与StorageService，产品根不通过args可切换；测试fixture不读取Preview/public历史。
- 长session/index损坏/13规范Tool/request/source/HTTP中断＋原5投影、3草稿、6保存+conflict全部通过，Release最终成功、269DTO/工程/fmt/计划结构通过。最初UI文件放Files被backend通配编译拒绝，已移UI目录/再生工程，失败日志保留。
- a23ef8a CI38163760418五jobs全部success含iPhone/iPad原1s；不抹掉5b96deb row17失败，也不替本批及final candidate。完整Pi/Provider/Prompt/Context/压缩仍R4.3–4.4，重置不兼容数学head仍需工作台动作；当前只有真正另存新文档路径。运行.3/dev/no main/no本机模拟器/无子代理，32final gate not_run/no candidate。

## R4开发记录 D037 — 备份、显式代次、配额和全库资源维护

- R4.1.13 Done/Tests满足，24/98；下一R4.2.01完整原生工作台。Root1/library4/doc5/.omnb v1/运行.3保持；普通打开不隐式升级，不读取旧应用数据、旧凭据或用户下载目录。
- StorageService全部异步接口纳入维护屏障，排空已接纳的写入/上传后才快照、升级或GC，新操作明确transactionUnavailable。关闭同样排空，缓存和数据库所有者释放根租约；实际并发上传保持真实publication pin。
- SQLite Backup API复制完整WAL视图；目标实际F_FULLFSYNC/完整性/外键/行数/引用/DB SHA，实际Blob文件pin、分块复制、SHA/长度及独立自包含副本都确认后才发布。manifest.json符合原冻结Schema，manifest_hash绑定inventory.json实际字节；伪verified被拒绝，原Rust校验器独立通过，发布确认丢失仍保留实际备份。
- 显式升级先升级前全库备份，在新代次重建Library索引及doc历史/kernel/save/draft，原typed表SHA/计数和回执字节不变，关闭同步/发布代次后才切selector。实际Library3的24原events和文档1的原source op/history、迁移后原事件投影去重通过。回退旧备份恢复Blob并标rollback_quarantine，备份后操作回执确实缺失仍不能打开writer、重放或借升级绕过；Library未来格式时仍独立只读隔离源码。
- 全库及保留代次引用、完整事件/资源、原Blob实际字节、健康备份和运行publication/reader pins核对后GC；过24h孤儿隔离同步/重查后删除，旧staging仅处理应用UUID路径，实际GC强杀后可续核。跨库未来格式拒绝，原字节不回收。派生缓存独立LRU512MiB、producer pin优先于配额，清缓存不动原附件；实际空间256MiB/1GiB门禁、5GiB软预算返回状态。保留最近7普通/2升级前及至少1完整健康备份，软预算不牺牲它们；日末触发和native管理UI仍属R4.2.18。
- [完整证据](../acceptance/pre-alpha.4/development/r41/storage-maintenance.json)封存输入、19个命令/完整输出/SHA及17实际SIGKILL原件；113 Rust、Clippy、269 DTO、15既有原生专项、Release最终exit0。旧失败构建/测试条件/隔离校验顺序/SHM缓存识别原件保留；没有修改原数学/计时门槛。
- eac1e35 CI38168802699全部5jobs成功；本批新SHA仍独立CI，不能沿用旧成功。32final gate全部not_run、candidate未分配、ready/released=false，完整Pi/Agent/模型媒体/上下文/工作台及最终发行仍未完成，无用户操作阻塞。

## R4开发记录 D038 — 原生工作台基础分栏与实际运行

- R4.2.01保持in_progress，24/98；已建立原生目录/笔记本/文档-结果检查器及真实宽度1042/861/640基础路径，助手/步骤/变量目的地和完整展示仍继续，不提前勾整个任务。原NSHostingView/TextKit editor/selection保留，隐藏不关闭owner；窄宽度popover可达，实际拖分栏208→238且结果不丢。
- 文档持有NotebookRuntime，原KernelCommitPort及耐久接纳回执/只读ResultClient实际接通。源提交/原UndoManager确认先于数学运行，不自动跑打开文件；不因窗口或面板隐藏销毁运行。真实producer phase/原ID stop/unknown核对，已提交与结果读取失败分开，晚stop不改成功。
- 实际AppKit窗口+原生产组件验证1042/861/640/1100、同一中心view/editor及原UTF16选区/中文emoji字节；SQLite/C ABI得到5和6，长CAS时缩窗/编辑/明确stop，后草稿9保持。持续隔离UI fixture经实际AX/截图验证2和3、目录折叠、结果检查器和分栏；正常关闭笔记本并Quit，不读实际Preview/public根。离屏cacheDisplay不作为实际画面证据。
- [完整记录](../acceptance/pre-alpha.4/development/r42/workbench-foundation.json)及[实现/技能参考](../design/native-workbench.md)。Release、本轮workbench及原保存6/草稿3/恢复5、kernel-runtime、UndoManager回归均exit0。初次@State多变量声明被编译器拒绝，两个fixture注释/UTF16选区错误与手工enum大小写错误已修；失败原件保留，原数学/性能门槛不变。
- 27ede3f CI38251418308 Mac/Rust/Web/deps四job通过，iPad原53 row34总1024.01475ms超过原1000ms，分段FFI948.197458/queued55.057208/decode2.589875/resume18.085417；原件已下载target/ci-evidence/27ede3f并封存corpus，不改用内部时间冒充通过。新SHA/完整最终candidate仍独立，本机无模拟器。

## R4开发记录 D039 — 授权模型测试、导航目的地和精确原生索引

- 用户提供的新测试端点/模型/凭据保存于仓库外受限文件（目录0700/文件0600）。4Router模型目录HTTP200且列出两个授权ID；gpt-6.1-sol/gpt-6-luna短文本均HTTP200且返回4。原始临时probe及显式配置文件的可重复脚本输出封存，[开发smoke](../acceptance/pre-alpha.4/development/authorized-api/smoke.json)没有密钥。生产Registry、Pi、工具/媒体和最终live gate仍未接通/未运行，不把transport smoke当完成。
- R4.2.01 Done：WorkbenchNavigation由原文档持有，文档/结果/助手/步骤/变量五目的地可达；同一原NSHostingView/TextKit/selection和滚动100点跨1042/861/640/1100、切换/隐藏保持。实际SQLite/C ABI得到5和6；长CAS可缩窗、编辑、停止且后稿9保留。步骤基础页取真实不可变记录，空记录明确未录制；变量仅核对真实计算版本/定义是否对应当前源码，完整树/变量值/助手按R4.2.11/R4.4.18继续，未实现能力明确不可用。
- R4.2.02 Done：原生TextKit2编辑器接线、数学smart替换/自动接受关闭，精确UTF8/UTF16 scalar表绑定source SHA/draft_sequence/行首。CRLF一次逻辑换行且原码元保持；原生Enter按原偏好、纯文本粘贴不拆格。42合法端点与全范围往返，中文/emoji/NFC-NFD/ZWJ/旗帜/Tab/混合换行/空文/EOF、非法UTF8/半代理/byte内部/越界溢出/旧sequence/hash拒绝，实际窗口原字节通过。
- [完整输入/日志/失败](../acceptance/pre-alpha.4/development/r42/native-source-and-navigation.json)。本轮workbench/source-editor、原DocCommitPort/Undo、草稿3/保存6/恢复5中断与Release通过，269 DTO/工程/计划结构保持。第一版新fixture将throw放进precondition非throw闭包，被编译器拒绝，修为先读取；原失败保存。实窗2/3与真实步骤未记录/定义随编辑失效/助手未启用已看，测试应用正常菜单Quit后确认退出。
- 80949bf/CI38274406316 Rust/Web/deps/iOS全部成功；Mac因runner未装rg在workbench源码收集阶段退出，未执行该fixture。原artifact封存，测试与UI构建脚本改用现有Python确定性文件发现并核对原文件集合完全相同，新SHA独立查CI，不删测试/放宽断言/新增数学改动。
- 26/98、R4.2.03 in_progress：真实marked-text API、UndoManager、晚输入/fence与中断回归已有，但此次Control-Space和实际字母键只出现拉丁文本，未验证系统中文/日文候选窗口，不声明该任务Done。下一继续候选窗口、完整非求值编辑分析及余下72项。运行.3/dev/no main/旧公开包/no本机iOS模拟器/无子代理，32final gates not_run/no candidate/ready=false/released=false。

## R4开发记录 D040 — 空间回收、真实输入组与命令面板竞态

- 用户要求继续完整计划并随时清理无用项目缓存；确认无cargo/rustc/swiftc/xcodebuild后，仅删除target/debug的deps/build/.fingerprint/incremental可重建缓存，target从52GiB到13GiB，可用磁盘11→50GiB（后续构建约49GiB）。验收原件、CI artifact、原源码/用户凭据、Release及旧公开包均保留。后续定向Rust测试使用CARGO_PROFILE_DEV_DEBUG=0/CARGO_PROFILE_TEST_DEBUG=0，debug缓存约1.2GiB，incremental仍关闭；不降低检查或数学阈值。
- R4.2.03继续in_progress，26/98。原生local UndoManager按实际input_group保留连续输入的一组，600ms空闲/光标跳转/显式边界另起组，实际marked合成超过空闲仍一组；undo/redo及确认回执前关闭自己组，echo不注册第二份文字undo。粘贴调用原生pasteAsPlainText响应链，保留原字节且单次撤销。
- 真NSTextInputClient中文/日文markedRange/更新/提交API、连续ab一次撤销/redo/空闲后c独立、合成超过700ms、合成时run及已冻结Agent preview不能替换、原source revision0/无已接纳kernel、NFD/emoji/CRLF粘贴一次撤销通过。它不是系统候选窗口验收；专用可见UI fixture用生产组件/私有root并实时显示真正hasMarkedText/markedRange/草稿/原source revision，无合成模拟按钮。请求临时显示输入法菜单+添加日文后恢复的授权仍待用户回答，未修改系统设置；不据此勾任务。
- 6a59193 CI38279366974 iOS/Rust/deps成功，Web命令面板复现旧parse错误仍显示时运行收尾busy未结束；palette的Run仍可点，controller静默忽略。已让Run/RunAll同步真实busy/可用cell，并显示disabled样式；原预期4/5s用例未改，固定Node26.11.1实际生产构建下原用例12次及笔记本全7项通过，lint/typecheck通过。
- 同CI Mac workbench出现Trace/BPT trap，旧-O日志没有具体断言，不能声称修好。补明确file/line失败输出、实际split/editor/selection状态与真实CAS观察，并只收本任务fixture的CI crash reports；本机原断言全部通过，下一新SHA独立诊断。
- [完整输入/回归/原失败/缓存证据](../acceptance/pre-alpha.4/development/r42/native-composition.json)：undo、composition、草稿3/保存6、source-editor、workbench及Release通过。待真实候选Enter/Tab/Esc与离屏pin；不阻断独立R4.2.07，已核对SwiftMath原fa8244ed固定源码并成功构建Mac真实模块/资源，生产公式接线尚未提交/未完成。运行.3/dev/no main/旧公开包/无子代理/本机无iOS模拟器，32final gate not_run/candidate未分配/ready=false/released=false。

## R4开发记录 D041 — 实际原生公式、内联视图与安装资源

- R4.2.07完成，27/98。固定SwiftMath1.7.3/fa8244ed接Mac工程、Release资源和各全源fixture；默认字体与metrics SHA验证、32个安装资源/许可文件完整核验。真实已接纳结果分别只读InputForm/Modern/LaTeX，保留ResultBinding、原UTF8和实际高精度源码，不在renderer制造近似或运行数学。
- 仅词法flat ASCII operatorname→mathrm显示适配；转义/star/嵌套/中文不改，原式复制与显示分离。64KiB、逻辑深度128（含无括号sqrt/font命令）、实际20000atoms在原生parser/排版前后核对；未知/缺参数/缺失或损坏字体/超预算/非法字号均可选原式及完整复制。300项长式保留22pt横滚，整式原生焦点/Command-C复制无截断。
- 原53+4科研实际C ABI/SQLite接纳与LaTeX回读，57 done/57非空/57 native ready；分式/根式/pmatrix/cases/区间在12/22/42pt的15组真实metrics。inline由实际TextKit2 fragment provider和frames显示，真实ascent/descent基线，源码替换移除旧view；未知原式真实附件fallback。可见fixture发现原手动mount测试遗漏的系统文件图标，修复公共viewport did-layout回调/provider请求重排和旧icon屏蔽后加强为自动窗口显示及实际frames断言，旧失败保留。深浅色、全部字号/矩阵/条件/区间、内联字形和完整未知命令，以及科研Root原式popover/24pt横滚均已实际屏幕检查。
- f875658/CI38285711835现全部terminal：Rust/deps/iOS success；Web设置保存status超过原5s仍失败，Mac原工作台目的地切换重置真实scroll失败。中心root只在文档会话实例变更时重赋，保留现有观察更新，本地原工作台四宽度/100点scroll/编辑选区/真实5与6/长CAS编辑停止全部通过；新SHA CI待独立验证，不将本地通过称旧CI成功。原SourceEditor/Composition/KernelRuntime、Release、269DTO与工程核验通过。
- [完整源输入/实际回执/原失败/安装清单/日志](../acceptance/pre-alpha.4/development/r42/native-math.json)及[实现说明](../design/native-math.md)。用户明确选择不调整系统输入法、先其他任务，已关闭暂缓IME fixture，R4.2.03仍in_progress，不再请求同一设置授权。磁盘约48GiB可用，target约13GiB，无需再清仍使用的依赖/Release/证据。
- 原运行.3、dev、不合main、不改旧公开包/标签、无子代理、不启动本机iOS模拟器，Root1/lib4/doc5/.omnb1，32final gates not_run/no candidate/ready=false/released=false；剩余71任务与完整发行继续，goal保持active。

## R4开发记录 D042 — 原生结果、条件/重数与只读数值

- R4.2.09完成，28/98。ResultPresenter按真实ref保留文档拥有的只读投影，读取原summary/presentation，NativeSolutionView显示finite/none/all/region、原条件/Root/根式/各验证标签；未求值表达式保留真实消息，不当作无解。首次类型与已接纳输出一起准备，原工作台scroll/选区断言仍通过。
- 核心原presentation page从完整typed解记录（含条件/verification/所有绑定原字节）取全局重数/首次序号，不比较pretty label、不从一页猜全局。派生索引20000条/8MiB/实际Interrupt，原20重根两页16+4均20、同初始序号；原数学求值与公共SolutionSetView不改。Swift只安排当前页，条件不同同值不合并。
- Actual InspectResultNumeric20/50/80、单解完整InputForm的原readonly InspectExpression20位；ref/原序号/绑定位置/原UTF8隔离，原精确式和80位精度标记保持，Out/random/主定义/源不改变。实际decoded回复等待期间真实编辑guard2→9、主提交后，旧回复不写新展示；源rev只由显式编辑0→1。重算保留旧值，接纳新值后32条小历史原回执/Binding按需读2，不复制全部大值，旧数据不可用不自动重算。运行单元格独立于焦点导航。
- 真实多语句失败保留2成功记录/7与Null、partial及实际递归错误；零成功E022明确未产生成功结果，不制造前缀；真正suppressed声明不在普通笔记本画输出。可见Cua检查原Root多项式/根序+实际近似1.167303978…、Exp整数条件+真实Numeric33、跨页重数20、高精度80/深浅色、无解/全体/区域、未求值消息、实际编辑/9/历史原值2以及最终零前缀提示，随后正常关文档/退出。
- [完整实际12输入/回执/数值/原57/回归/失败证据](../acceptance/pre-alpha.4/development/r42/native-results.json)、[实现说明](../design/native-results.md)。6 retained Rust与host results/runtime、Clippy/fmt/pureWASM、269DTO/工程、原ResultClient/KernelRuntime/保存/草稿/恢复/SourceEditor/Composition/Workbench和Release/安装字体许可检查通过。回归编译时输入被编辑的失败明确拒绝并保存，冻结源码后的独立重跑通过；旧单compound测试也保留，不当作真实成功前缀。
- 21355b3/CI38369472679已五jobs全部success，Mac实际workbench scroll和57SwiftMath/fonts资源也真通过，原1s数学门槛不动/iOS只CI。此批新SHA继续独立核对；此前f875设置超时/scroll失败仍保留。当前约48GiB空闲、target约14GiB，不删当前依赖/Release/源码/证据。
- 用户保留OS输入设置、R4.2.03真实候选窗口仍未验；下一R4.2.10或规范Agent/IPC，Markdown/Steps需原依赖先满足。运行.3/dev/no main/旧公开包不动/无子代理/no本机模拟器、Root1/lib4/doc5/.omnb1，70任务与完整发行、32finalgate not_run/no candidate/ready=false/released=false，goal保持active。


## D043 · 2026-10-13 原生结构值与科学诊断

- R4.2.10完成，29/98。真实InspectResultPage经原result_ref/document/scope/view核对；NSTableView/NSOutlineView有界读取，真实条目稳定身份，原字段展开/选择/双击与Return入口。记录选择拥有实际窗口键盘焦点，隐藏/同页更新不重载数据源。
- 默认32×8，协议单轴最大100行/32列、path32；真实105×35矩阵、40×10表、37字段、嵌套与深度边界，完整28页/元数据记录及21输入。100×32大页真实触及原256KiB回复预算，保留原页；首次超限保留重新核对的summary/presentation/完整源式。没有抬高64MiB/32快照、8MiB源式或最终性能限额。
- 精确1/3、大整数90071992547409931234567890、80位原数、NFC/NFD/中文/emoji原字节与性质不丢，不提供Double排序。页面摘录明确命名，完整当前路径的Modern/Wolfram/LaTeX单独分块/SHA读取，等待系统剪贴板接纳与原字节回读；可见1/3独立pbpaste校验。
- 真实根ScientificOrigin才解释科学字段/保证；任意converged记录、缓存、包装和嵌套不冒认证。实际fit模型及样本/预测表、interpolation/series/quantity/ODE原kind与完整源式可读，不在界面猜曲线。实际延迟回复在native guard2→9提交后丢弃，Out/random/定义/source不因分页改变。
- 最后完整原生21用例/真实窗口焦点/NSEvent与Return、10Rust value/retained tests、269DTO/工程/Release安装32资源与签名通过；原57公式/12解、workbench真实width/scroll/CASstop、ResultClient/KernelRuntime回归通过。可见浅/深表格、真实与伪造保证、复制回执、真实嵌套双击/Return检查后正常Quit。原编译/依赖未就绪/大页/原ref过期/AX超时与采样保留，不拿选择性窗口检查宣称千格或AX性能完成。
- 精确40cc1cb/CI38377911174五jobs全部success；Mac57/12/mathresources/workbench和远端iPhone/iPad各35passed/0failed，仅读xcresult不启动本机模拟器。本批提交后的新SHA另验CI；旧失败不抹。
- 用户仍保留系统输入设置，R4.2.03真实候选窗口待验。新授权私有API仍可在全计划使用，不读旧Key/env/profile；运行.3/Root1/lib4/doc5/.omnb1/dev/no main/旧公开包不动/无子代理/32final gates not_run/no candidate/ready=false/released=false，完整69任务及发行仍继续，goal active。下一独立R4.3.01规范Agent消息/任务/IPC。


## D044 · 2026-10-13 Agent规范契约与入口基础（未完成任务）

- R4.3.01进行中，仍29/98。新增native-agent.schema.json规范15类IPC帧/5类canonical parts/12冻结工具名，合并生成Rust/Swift/TypeScript，native DTO336。没有Pi内部state/任意URL/凭据字段；大字节只受控引用。schema声明版本不证明实际Pi包或runtime，R4.3.02仍未做。
- Rust/TS FrameDecoder用真实字节断包，UTF8/重复解码member、1MiB、depth64/node32768/batch32、stdout非JSON/不完整EOF失败并poison连接。Rust AgentIngress固定trusted runtime/session/document/task/connection与generation，先scope再duplicate/gap；canonical assistant/turn/task/part/call和原argsJson核对，拒绝伪造工具结果。trusted ledger confirmation以前不能映射工具intent；新native entropy ID与provider/call ID分开、重试同ID不造新intent。readonly/grants/cancel/close和settled仅awaiting实际结算已约束。
- 这些是pending identity/data，没有executor；NativeHost/CABI真实source owner接线、Swift AgentSessionService真实canonical SQLite readback→confirm→ACK、model/request ID与方向/stream correlation、私有实际process/pipe和跨语言恢复仍未完成，R4.3.01主checkbox保持空。不能拿memory registry合成scope当实际业务接纳，更不能当完整Pi/live gate。
- 7Rust新protocol/identity+10原contracts、3Node真实流测试、严格tsc、336DTO/Swift真实CABI、Clippy所有host targets/fmt/最终Release32资源+codesign通过。Node开发测试用本机25.9，包内Node26仍属下一任务；CI新增Node协议测试。完整输入构造/检查源码、失败/最后日志SHA见agent-protocol-foundation.json/raw。
- 7cdfe03/CI38406847557精确SHA五jobsall success，前批R4.2.10开发验证有效；本批新SHA另验CI。无本机模拟器、无子代理、运行.3/dev/no main/旧公开包不动/Root1/lib4/doc5/.omnb1，32finalgates not_run/no candidate/ready=false/released=false。用户保留输入法设置不重问，新授权API仍仓库外私有路径且可全计划用，无旧key/profile/env读取；69任务及完整发行继续，goal active。

## D045 · 2026-10-14 规范Agent实际宿主接线与耐久ACK

- R4.3.01完成，30/98；下一R4.3.02。357闭合DTO、SourceEndpoint真实owner→CABI→Swift服务，文档生命周期绑定；原消息/请求/工具身份映射与新代次/取消隔离，恢复不复活授权。
- 20Rust（3新真实source端点+7原协议+10contracts）、3Node、strict tsc/Clippy/fmt/生成漂移、Swift实际CABI原回归、Release及32资源/深度严格签名通过。实际私有合成进程7帧、相同ACK重放、8类异常输出、真实写入序号缓冲与6条SQLite全事件/回执保存；日记恢复后旧runtime/提案/grants拒绝。SQLite写暂停仍可立即取消，已持久化未发送输入保留；1,000,914字节合法帧展开超过2MiB日记限制，拒绝且无ACK/事件。
- 原失败（缺测试源依赖、Foundation填满读小帧死锁/实际sample、Swift async semaphore、OS自动字符编码变量断言）及完整输入、帧、事件、命令/日志/SHA见[证据](../acceptance/pre-alpha.4/development/r43/agent-protocol-native.json)和49 raw文件。Node本机25.9仅开发，CI26.11.1；生产Pi/runtime未验，tool/request仅pending意图，没有模型请求/工具/CAS效果。
- 549418b exact CI38419147394五job全success；本批新SHA需另验CI。清理16项闲置WASM/iOS编译deps/build/fingerprint/incremental约3.91GiB，保留profile根产品、旧公开包、当前Mac/Debug、所有源码及输入/失败证据；盘约28GiB。
- OS输入设置保留，R4.2.03真实中文/日文候选窗口待验不重问；私有API仍仓库外600/700全计划可用，无旧Key/env/profile读取。运行/公开.3、Root1/lib4/doc5/.omnb1、dev/no main、无子代理/本机iOS模拟器、32最终gates not_run/no candidate/ready=false/released=false；完整68任务及发行继续，goal active。

## D046 · 2026-10-14 CI资源竞争与Pi循环初验

- eaaa91b/CI38441550266前端36例中34通过/1跳过/1失败：西瓜场景30s仍Running，2 workers。下载原错误页面及完整job日志；本地Node26并行5 workers复现scene5s/西瓜30s两失败，33通过/1跳过。CI改为1 worker后完整35通过/1跳过，96s；保留应用预算、断言与测试时限。资源竞争为证据支持的推断，仍需新SHA CI确认，[官方Playwright建议](https://playwright.dev/docs/ci#workers)同样使用CI串行。[完整证据](../acceptance/pre-alpha.4/development/r43/frontend-ci-serial.json)。
- R4.3.02开始：官方Node26.11.1 arm64 SHA与现有候选缓存一致；真实Pi1.0.4 wrapper的3循环测试通过（ACK后串行工具/取消阻止执行/拒绝shell与错task），尚为合成native callbacks；ESM生产闭包/动态资源/随包绝对路径/真实task fixture仍待实现。严格自身TS通过skipLibCheck；原上游Google GenAI声明缺可选MCP类型失败日志保留，不为此安装MCP或CodingAgent。任务未完成，当前30/98、最终32gate not_run、运行.3，完整68任务与发行继续。

## D047 · 2026-10-14 实际随包Pi与生产闭包

- R4.3.02完成，31/98、67任务未完成；下一R4.3.03单轮sans-IO模型codec。固定Node26.11.1/arm64与Pi1.0.4真实ESM闭包86依赖/4169文件/约172MB，原生App实际签名/资源路径/空cwd/env/绝对入口、完整hash/架构/无外部node/npm/TS/addon插件；官方Node OpenPGP签名与原Developer ID实际验证。
- 8真实Node/Pi协议/循环例，21Rust（新host流offset/terminal/旧task/伪receipt拒绝）、严格自身TS/Clippy/fmt/357DTO/工程漂移、最终App Release32字体/许可与严格深签名通过。实际App内Node/Pi通过CABI/source/SQLite完成6helper帧、13双向全帧、3事件（用户、assistant proposal、工具未执行事实）；原参数空白完整保留，ACK后才提案。合成native model port是本任务fixture，没有模型HTTP或工具效果；错误模型配置无假assistant事件。真实SIGKILL后同文档主内核仍4；取消/迟到delta/stdin、缺资源、错manifest及实际Mach-O即使伪造匹配hash拒绝。
- a70311f/CI38445836520终态4job成功（Rust/frontend/deps/iOS）；Mac保存独立编译漏Agent源，已补保存/草稿/恢复3脚本，原保存6中断、草稿3中断、恢复5中断与原CABI真实回归全通过。新SHA仍需另验CI；不继承旧结果。原全部失败/命令/输入/实际数据与完整manifest/SHA见[54 raw文件证据](../acceptance/pre-alpha.4/development/r43/pi-production-runtime.json)。
- 清理闲置重复候选约165MiB，当前完整App/独立runtime/Node原包/全证据及源码保留，盘约27GiB；不清理活动编译。私有API外部600/700全计划授权，未进源码/IPC/env/log；OS输入设置保持，R4.2.03候选待验不重问。Root1/lib4/doc5/.omnb1、.3、dev/no main/旧包不变/无子代理/本机iOS模拟器、32finalgate not_run/no candidate/ready=false/released=false；完整67任务及发行继续，goal active。

## D048 · 2026-10-14 独立单轮 codec

Chat/Anthropic 请求与实际 SSE/JSON decoder 完成，canonical 来自同一生成契约；prepare 冻结 binding/工具/输出身份，保留原参数/response ID/model/finish/usage 与兼容 signed thinking，incomplete 不产生可执行调用。182 Rust（含10单轮）/4实际Pi/17 Python/357DTO/Clippy/fmt/原生 ABI 与真实随包 helper 通过；原生实际6 helper帧/13双向帧、3 SQLite事件、ACK后工具不可用、SIGKILL后同文档4。授权两模型真实Chat SSE HTTP200/完整4及usage，仍只是开发smoke；完整证据 single-turn-codec.json。

94dbfbe/CI38457404317终态Rust/deps/Mac success，frontend保存超过5秒后成功、iOS原17总1168ms（FFI280/resume884）失败；原门槛未改。本地设置连续3轮12项pass，不能据此声称CI问题已修复，新提交CI独立验。32/98，66任务与最终发行继续；系统输入设置保留、不使用子代理/本机iOS模拟器、版本仍.3。

## D049 · 2026-10-14 新 SecretStore 读取基础（R4.3.04进行中）

新Mac服务精确引用、不枚举或回退旧profile/凭据，WhenUnlockedThisDeviceOnly与禁止同步、LAContext禁止交互；失败状态不返回secret，数据不可Codable/调试脱敏，HTTP CRLF/非ASCII/长度拒绝。实际系统仅查新随机不存在引用；三锁定/认证状态为注入验证，没有锁屏机器或更改系统设置。Swift6 warnings-as-errors/新项目生成/真实Release/strict签名通过。初始弃用API警告已用SDK推荐的LAContext替换并留证。

R4.3.03已推21a08e5，CI38465324522运行中；本基础先本地提交，后续URLSession/Rust通道/生命周期/URLProtocol/实际锁屏验收继续，不勾选R4.3.04，不改32/98及全部最终门禁。

## D050 · 2026-10-14 实际原生单轮传输（R4.3.04进行中）

独立Rust model channel/C ABI与371DTO、SwiftURLSession实际原始字节接线完成；随机codec ref、request/context nonce、8在途/4096历史、8KiB feed、host close单向撤销，与CAS/source队列分离。ProviderRoute拒绝非HTTPS/认证query/任意redirect、只允许显式literal loopback；认证仅最终native头、cookie/cache/shared credential关闭、不自动付费重试。原生Agent端口分割32KiB Unicode，incomplete失败不发可执行canonical，保留原usage/replay/结束理由给native complete callback。

194Rust/17Python/371DTO/Clippy/fmt、实际URLProtocol Chat+Anthropic/body/auth/401/429/echo/三阶段timeout/cancel/hostclose/并行真实CAS4、原ABI/Pi回归与Release/strict签名通过；两授权模型实际SwiftURLSession→Rust C ABI均完整4/实际usage。首响应60s包含DNS/TCP/TLS，独立连接10s尚未实现；durable request journal callback在此证据为合成边界，真实ContextSnapshot/请求账本producer和物理锁屏仍待验，不勾Done，不改32/98。SDK确认D049缺DataProtectionKeychain标记导致默认Mac访问策略不能成立，现已修正、留旧证据。完整输入/实际HTTP/原生事件/源hash/初始失败见provider-transport.json/raw。21a08e5 CI38465324522已五job success；当前新提交另验。

## D051 · 2026-10-14 独立连接预算（R4.3.04继续）

通过公开URLSession didSendBodyData/实际body发送观察连接已建立，或响应头观察作保守完成证据；本接口非空POST，连接10s与首响应60s/idle30s/total180s各自计时。真实自有HTTP回环收完请求后延迟1.3s响应，在1s connection预算仍成功；无连接进度的connection timeout分别验证，未把模型等待裁为连接超时。完整raw HTTP请求、SSE、实测elapsed及全部旧URLProtocol回归保存。真实请求账本/ContextSnapshot producer及物理锁屏仍未完成，R4.3.04保持in_progress、32/98。99889e4已推dev，新CI另核；本连接小批次先本地保存，继续下一接线。

## D052 · 2026-10-14 真实请求账本与随包原生传输

ProviderRequestLedger实际存body/plan→dispatch→原始response→finish及独立原模型facts Blob，遵循现有request/response owner绑定；metadata与header-only观察自描述，unknown dispatch保持interrupted。五真实SIGKILL在plan/dispatch/response/finish/model回执耐久点恢复准确状态且不发HTTP；原13边界/60schemaevents回归通过。字面auth echo跨chunk拦截，不进入model/Blob/event，完整关闭store扫描为空；正常字节全切分等同原始值，真实timeout保unknown无retry。

随包Pi已直接用ProviderAgentModelAdapter/SwiftURLSession/Rust C ABI/生产账本，移除该集成fixture对测试Rust可执行文件的依赖；standalone/实际签名App资源闭包13双向帧、8SQLite事实、原args/ACK后unavailable、强杀后原文档手工4。实际原request/response与原providerID/model/replay/usage/finish已Blob回读核对。Release/strict签名/371DTO/17Python/旧ledger回归及两授权模型guard后真实live均通过，完整原始输入/SQLgzip/hash/失败在provider-durable-request.json。R4.3.04步骤1–4勾实测，物理锁屏未验故仍in_progress/32of98，完整ContextSnapshot属于R4.4.12未完成；接下来先独立R4.3.05只读真实工具，保持用户系统设置。


## D053 · 2026-10-14 只读工具数据服务基础（R4.3.05进行中）

read_notebook默认outline20格、limit1–50、完整cell或32KiB UTF8边界片段、查询绑定游标、128引用/32MiB/10分钟上限；真实DocumentCommitController snapshot记录实际完整曝光集合，读取完整格使用新token，原outline/所有长格片段仍不能整格覆盖。150008字节中文/emoji/组合字符逐字节重组6页通过；历史快照明确is_current=false，scope/权限/过期/倒退时间/篡改游标/重复参数拒绝。符号与Done状态来自实际接受worker checkpoint，config变化显示stale，原checkpoint bytes不变；只读路径不执行源码或抽随机。

真实function_documents只加入257个已实现回调，按actual descriptor ID连接metadata26/原帮助/示例，示例仅not_verified；所有结果逐一通过闭合工具schema。真实Session接受let sin=fn(x)=>x+1后，用户绑定优先于内置别名，不改变原Sin保护。原闭合校验器不识别等价SHA256正则写法导致首轮读取失败，已严格支持两种相同语义拼写，非法长度/大写等仍拒绝。222 Rust（195host/shared/llm、18受影响kernel、9appleABI）、17Python、371DTO、Clippy/fmt均通过，原始输入/命令/输出/失败和SHA见readonly-services-foundation.json/raw。

本批仍是原生只读数据服务基础：未接Agent已ACK intent的原参数分发、Swift耐久结果Blob/ACK和实际随包Pi数据消费；include_outputs=true明确NotAvailable，未伪造result_ref；近期事务只取当前owner真实完成receipt，重启持久历史producer尚未接完。因此R4.3.05仅步骤1–3勾实测，Done/步骤4不勾，任务总数仍32/98，全部最终gate not_run。

bea784c CI38560769195已观察deps/Rust/frontend success、iOS failure、Mac仍运行。下载的原始xcresult/活动/截图表明失败在NotebookUITests.swift:105滑块AX值断言，后续原expression16及旋转断言通过，截图实际a=4/result16。推断系统默认AX值表示/时机与Double断言不匹配（原slider.value文本未包含于证据，不能断言它具体是什么）；ExploreOutputView现在显式发布实际参数值String，保留原数值/timeout断言，修复仍待新SHA CI验证。本机仅读取结果包，未启动iOS模拟器。完整原result归档逐文件核对，待验证清理重复下载副本；系统输入设置不变。


## D054 · 2026-10-14 撤销已唤醒的过期计时回调（R4.3.04继续）

bea784c CI38560769195终态deps/Rust/frontend success，iOS与Mac failure。Mac artifact原日志指向ProviderTransportFixtures:157复合超时断言，未记录具体哪一case/actual reason，不能把它直接归因到某种超时。复查发现独立可证实竞态：Task.sleep已经返回后，响应阶段取消旧Task仍可能调用stop。新增可信fixture probe在真实首响应timer唤醒后暂停回调，让实际URLSession收到200并推进idle阶段；EOF保持关闭，释放旧回调并等待其执行完。原实现实际错误firstResponseTimeout/responseObserved/200（RED）；加每lane不可复用ticket并在同一锁内验证/撤销后，同用例完整成功（GREEN），旧回调不得结束新阶段。生产10/60/30/180及全部原合成负例时间窗/断言均未放宽。

原URLProtocol/真实回环/4种timeout/CAS/取消回归通过，新增各case实际reason/outcome/次数在断言前落文件；CI新增always步骤保留这些attempt事实，失败后也能定位。实际Library4请求账本、原字节、echo拒绝、unknown timeout与5真实SIGKILL恢复无HTTP重放通过；Release重建和严格签名、实际重签App资源随包Pi（6helper帧/8SQLite事实/强杀后手工4）通过；17Python/371DTO通过。原始RED/GREEN源、实际输入/请求/计时事实/原CI日志见provider-timeout-race.json/raw。iOS D053显式actual AX值修复仍须新SHA CI证明，物理锁屏未验/工具接线未完，32/98/所有最终gate not_run保持。aaf48f3已推，其CI38565187538曾运行；本修复后续提交CI按新SHA独立核验。此前重复CI result下载经耐久归档逐文件核对后清除13113610字节，当前43GiB空余，所需build缓存保留。


## D055 · 2026-10-14 原生只读工具耐久结果与实际Pi两轮（R4.3.05继续）

agent_read_tool只接受已ACK proposal映射出的operation_id；host从原canonical提案保留的arguments_json取参，不接受调用者重发参数/工具名。SourceEndpoint实际执行read_notebook/search_functions/get_function_docs，冻结闭合结果、随机result_ref与原字节hash；旧ID回读同一原结果，原scope变化/取消/10分钟过期拒绝，包括已经sent的旧帧重放。Rust已验证3工具真实返回及结构化INVALID_ARGUMENT、确认前禁止send、hash/body篡改/跨scope/原始重复字段/额外参数拒绝；原outer catalog envelope漏pagination初测失败后已修，始终按原schema校验，不改schema放行。

Swift发布真实结果Blob，记录带canonical role=tool/result_ref的native_readonly_result恢复事实；独立回读SQLite事件和引用Blob原字节后agent_confirm_tool，才允许tool_result帧携带同份result_json给helper。该帧新增required nullable result_json，与不带内联内容的canonical tool_result part分开；同源377DTO和helper schema hash握手使旧资源不能混用。helper验证闭合工具结果，将原数据交Pi工具流程并继续下一轮；未实现工具仍unavailable。真实Rust C ABI→Swift Library→私有随包Pi→第二轮真实URLSession/Rust codec，实际20双向frames（11host/9helper）、14SQLite事件、原参数空白保留，第二轮body包含从引用Blob回读的tool content；无真实付费请求。

恢复时仅读Storage，不创建host/helper/HTTP，结果Blob和14事件不变；专门readonly_completed/readonly_failed状态避免把完成的只读查询误投影为未开始。SessionSemantics验证原call/name/arguments hash/turn/task/canonical ref，原Blob引用和字节长度必须存在；缺引用和篡改原参数实际append被拒绝。indexed和纯事件fallback均投影真实只读状态，不产生DocCommit/CAS effect或恢复权限。原13SIGKILL/60schemaevents、长4374事件/>16MiB分页回归通过。原脚本缺debug=0会重建调试符号缓存，本批补显式profile变量，原Rust输入输出逐字节相同。

205Rust回归、14固定Node26/Pi测试、strict自身TS/skipLibCheck、17Python、377DTO/生成项目/Clippy/fmt通过；Release重建和严格签名，standalone及重签App原资源真实两轮/持久恢复/错误资源/迟到stdin/取消/无模型拒绝/强杀后手工4通过。完整源码输入、原wire/body/事件/压缩原SQLite、原失败与SHA见readonly-native-tool.json/raw。原Node全套初次漏显式offline codec fixture环境、Node合成终结payload含undefined多余键、Swift SessionRecoveryPage不可Encodable等测试错误原日志保留，已在测试调用/自描述投影修正。

include_outputs=true仍明确NOT_AVAILABLE，尚未绑定真实ResultRuntime producer；recent_transaction_refs仍只来自live owner真实完成receipt，重启持久事务历史producer待接。因此R4.3.05不勾Done/步骤4、32/98/所有finalgate not_run保持；R4.3.04物理锁屏及R4.2.03真实候选窗口未验，系统输入设置不变。f74371b CI38567271292最后观察Rust/frontend/deps success，iOS在iPad流程、Mac已越过transport到后续Agent/存储流程，未当最终成功；新提交CI按新SHA另核。完整计划与最终发行继续required。


## D056 · 2026-10-15 真实输出引用与持久历史（R4.3.05完成）

read_notebook(include_outputs=true)现通过独立ResultRuntime worker查询实际保留结果，按读取快照的kernel revision与精确cell/source hash选原producer，返回现有HMAC作用域result_ref；不保留、抑制输出或新host无实际kernel结果时返回null。读取不求值、不抽随机、不改checkpoint，source owner只轮询同operation，取消/过期/变scope拒绝迟到结果。实际1/3引用可检查，真实第二次计算后旧snapshot仍指producer1，当前指producer2；暂停结果worker时主状态/取消仍可响应。

Swift在单一document串行读中获取confirmed head与最多32个持久事务事实，再以head hash/native scope回绑；history列与原summary/hash/identity一致性均检查。实际206次SQLite提交回归通过，伪造operation列拒绝；新host只从重开的存储读原事务，无live owner receipt也可返回原history，未恢复任何旧授权。历史snapshot保留原refs。

实际随包Pi fixture先通过DocCommitPort产生真实rename事务，再由KernelCommitPort实际计算2+2，模型原参数include_outputs=true经ACK→native→真实输出ref/事务ref→Library Blob/独立回读→helper→第二轮HTTP原body；standalone及签名Release App均20帧/14SQLite事件，恢复阶段只读Storage且不建host/helper/HTTP，随后单独显式新host验证持久history；新host无kernel结果如实null。原SIGKILL后同文档手工计算4、取消/迟到帧/缺资源/错架构/篡改/原授权隔离仍通过。当前route/context factory为合成fixture，完整ContextSnapshot/Registry/UI/live闭环属于后续任务。

全workspace1297 Rust/0失败、Clippy -D warnings、14 Node26/Pi、17 Python、strict自身TS(skipLibCheck)、382同源DTO/生成工程/fmt、Release build/strict签名通过。完整输入/命令/原始失败/真实wire与SQL压缩字节及SHA见readonly-outputs-history.json/raw。满足原R4.3.05 Done/Tests，步骤4与任务勾选，33/98；下一R4.3.06，不能重造scope或把源码excerpt升级成整格写权限。

21c47a2 CI38573532854已终态：Rust/frontend/iPhone+iPad成功；依赖因10月14日新披露四项Rustls公告失败，Cargo.lock仅0.23.45→0.23.46与checksum变化，cargo deny --locked check通过；Mac total负例实际先触发合法idleTimeout，隔离该case idle guard为1秒，total100ms与独立idle30ms负例/生产10/60/30/180秒不改，实际transport全部回归通过。原CI/失败case与官方公告原文均封存；新提交CI另核，不能用本地代替。

按成功Cargo compiler-artifact清单，仅删除216个target/debug/deps中profile.test=true普通可执行测试缓存，先记录各SHA/字节/清单与无编译进程证据，释放3,841,244,032字节（3.58GiB），可用约44GiB。依赖库、CLI/examples、当前App/runtime、用户文件及全部证据保留。R4.3.04物理锁屏、R4.2.03真实候选仍未验，保留用户系统输入设置、不重问；无子代理/本机iOS模拟器。32最终gate全部not_run/no candidate/ready=false/released=false，运行版本仍.3，dev/no main/旧公开tag包不改，完整计划与真实发行继续。


## D057 · 2026-10-15 实际Agent冻结预览与三轮上下文（R4.3.06进行中）

R4.3.05完成批次已推dev b15a39fe8b4ec688a54567a5ca74375b77d6bd5a，CI38595323283最后观察Rust/frontend/dependencies成功，iOS已进入iPad、Mac进入实际提交/存储流程，尚未当全CI成功。完整98任务/32最终gate/真实发行范围不变。

tools/patch.rs已将ACKed原始preview_source参数直接传给现有DocumentCommitController/PreviewService，以原read_notebook真实snapshot及当前Agent任务/grants验证，不新建手工scope、不扩大完整源码曝光。source-only无committable ref，真实有效整个patch才冻结原计划。原有native结果hash/ref→Swift耐久Blob/canonical recovery→独立回读→确认→Pi原数据路径也接入preview_source；此结果只表示检查/冻结完成，不执行源码或提交。

31项Rust回归覆盖真实Agent读取→预览、旧outline经后续full read仍不能整格覆盖、手工task=None端口不能消费Agent预览、source-only无提交ref、patch第二项失败不改源码/revision/epoch及未知参数闭合拒绝。15 Node26/Pi、strict自身TS(skipLibCheck)、Clippy host全部targets -D warnings、17 Python、382DTO/生成工程/fmt、Release及strict签名通过。

新增独立实际Pi三轮：真实读取格a完整2+2→实际冻结3+3 patch→第三轮HTTP body逐字使用已落库预览结果，standalone和签名Release App资源闭包都29帧（16host/13helper）、21SQLite事件、2个readonly结果/3个真实回环URLSession请求，confirmed head仍原hash/revision0、无source history/运行/保存。Storage-only重开恢复原21事件与2个只读完成事实，无host/helper/HTTP/旧授权；原两轮20帧/14事件、真实输出/持久history、SIGKILL后原文档手工4及资源/取消/架构负例均继续通过。

原始失败完整保留：fixture遗漏cells必需cell_ids被Pi schema先拒；合成context工厂将多轮assistant排在tool结果之前，被原生codec在第三次prepare拒，已按实际持久事件顺序组装；21事件被fixture误作默认16条首页，已按原16条页/after游标读取，21事件恢复用现有支持32条页，生产默认/上限及精确21断言不变。Swift fixture三个实际类型误用与原编译失败也保留。完整输入/原wire/原body/Blob/压缩SQL与原失败源文件、命令、SHA见patch-preview-foundation.json/raw。

R4.3.06尚未完成：apply_notebook_patch仍unavailable；下一让原ACKed apply意图仅解析preview_ref，接真实DocCommitPort/SessionToolPort耐久admission与原operation效果回执。最终source COMMIT屏障必须核实当前真实AgentScope，不能用手工task=None或恢复旧grant；冲突/IME/未知状态要结构化且不假报保存/运行。该任务所有步骤与Done仍未勾，33/98保持。完整ContextSnapshot producer仍属R4.4.12；当前factory是合成route+实际数据fixture，非Registry/UI/live完成。用户OS输入设置保留、R4.2.03真实候选/R4.3.04物理锁屏仍未验；无子代理/本机iOS模拟器，本批无付费live；.3/dev/no main/旧公开tag包不动，32finalgates not_run/no candidate/ready=false/released=false。


## D058 · 2026-10-15 真实原子修改、耐久效果与原ID恢复（R4.3.06完成）

SourceBeginAgent仅接已ACK原意图的native operation_id，从原canonical参数严格解出preview_ref，以同一真实Agent scope解析现有冻结PreviewService计划；不接重新发送的doc/grants/source，也不另签手工snapshot。实际DocCommitPort复用原source admission、MainActor DraftStore fence、PhysicalCommitGate和同库SQLite路径，最终短SourceBarrier从当前仍有效的Agent任务/权限重新派生scope；取消/编辑状态/marked text在COMMIT前拒绝，已过屏障则按原物理回执核对。

会话效果顺序已实际接通：canonical ACK→SessionToolPort耐久admission/start→DocCommitPort真实提交→独立原outbox/receipt效果→Library tool_result→canonical effect结果Blob/回读→native确认→Pi原数据。NativeAgentEffectResult与纯只读结果分开，completed必须有真实匹配tool_result/target/请求hash；缺Blob、原参数hash被改、无真实效果的成功记录、native错误结果hash实际拒绝。返回实际changed/invalidated/transaction/new scoped snapshot/plan hash及saved=false，不运行/保存。

standalone和最终签名Release App资源均实际四轮38帧（21host/17helper）、31SQLite事件、3工具/4实际URLSession回环；原格2+2被原子改成3+3，仅一笔事务，未生成Kernel checkpoint，第四HTTP body逐字包含耐久原结果。Storage-only重开恢复原31事件/3工具和真实完成target，无host/helper/HTTP/旧grant；原两轮20帧14事件/真实输出与持久history/强杀后手工4、纯预览三轮29帧21事件及取消/资源/错架构负例均继续通过。当前模型route/context工厂仍为合成fixture+真实耐久数据，不是完整Registry/UI/live闭环。

五项实际C ABI+SQLite边界（合成private proposal，但canonical ACK/source writer真实）：相同原ID重复提交仍单事务；真实DraftStore marked fence拒且无写；用户源码改为5+5后旧预览失效、无Agent写；source_committed后丢ACK先保留结构化unknown+原operation，随后仅readback原receipt恢复completed，不再执行；实际SQLite事务暂停before_source_commit时直接取消，回滚且恢复cancelled。unknown/cancelled由实际Pi SDK终止本循环，不请求下一模型或重试写。DocCommitPort settleNoCommit补读原native cancel_requested，避免直接stop已赢却被后续错误结算为普通failed；indexed/fallback恢复保留历史unknown原字节，同时采用后来实际确认的completed事实。此marked测试不声称真实系统IME候选窗口已验。

全workspace1301 Rust/0失败、workspace Clippy全targets -D warnings、17 Node26/Pi（含真实SDK unknown终止）、17 Python、strict自身TS(skipLibCheck)、389同源DTO/生成工程/fmt、Release及最终strict签名通过。原13 SIGKILL、60闭合schema事件、4374事件>16MiB/37calls+37plans按5条页恢复回归通过。完整源码/命令/原失败/38帧/原HTTP/Blob/SQL压缩字节与SHA见patch-atomic-apply.json/raw。满足Done/Tests，R4.3.06全勾，34/98、下一R4.3.07。

本轮按成功Cargo compiler-artifact清单又清216个target/debug/deps普通测试harness，先验1301/0终态、无compiler或运行harness，并记录每文件SHA/字节/清单；删除3,842,580,880逻辑字节（3.58GiB），可用约43GiB。库/CLI/examples/App/runtime/证据/用户文件均保留。ea008c4 CI38599857402最后Rust/frontend/dependencies成功、iOS进入iPad、Mac进入私有Agent耐久ACK/进程回归，未当最终全CI；b15a39f旧run后因后继提交取消Mac/iOS，不冒称全通过。新提交CI单独核对。用户OS输入设置保留、不重问/不改；R4.2.03真实候选与R4.3.04物理锁屏仍未验，本批无付费live/子代理/本机iOS模拟器。运行.3/Root1/Library4/doc5/.omnb1/dev/no main/旧公开tag包不动，32最终gate全部not_run/no candidate/ready=false/released=false，完整计划和真实发行继续required。

## D059 · 2026-10-15 真实数学依赖规划与标题修改后的结果新鲜度（R4.3.07进行中）

新增Session非求值执行规划：selected只规划指定格，required仅加入实际未Done的前置，affected沿实际读写关系扩展；复用现有拓扑/环判定和live owner优先级，动态写入归属也按真实Session处理。显式native allowed集合限制扩展，最多1000格，遍历可取消/限步；与RunAll无关。仅产生有序计划、真实环/阻塞/外部未就绪前置，不求值或改defs、Out、随机及原checkpoint bytes。

Host冻结入口只接原RunCells意图与匹配参数hash，从当前实际ReadNotebookService签发的snapshot_ref取得源码，继承真实accepted checkpoint并在独立线程按KernelWorker同规则reconcile；无checkpoint且definition revision非零拒绝。历史read/source token不授权当前执行、outline不升级整格替换权限；source/epoch/config/editor/task/grants/期限变化仍拒绝，模型不能传source/grants或扩大native执行allowlist。

补验发现只读捕获原用整个文档hash判断数学freshness，真实标题事务会误报Stale。先以原实现复现，再改为比较实际数学源码、execution epoch和计算配置；只改标题保留Done/现有定义，数学源码变化仍Stale，原producer hash/revision保持原样。65项Rust（35 Kernel、30 Host）通过，真实KernelWorker状态与synthetic matching unit receipts仅是单元契约证明，不冒称SQLite耐久执行。standalone与签名App均实际计算后再改标题，原生read返回Done/原result_ref/两笔持久事务，20frames14events与第二HTTP body exact；四轮原子修改38frames31events/5实际边界/强杀后手工4也通过，Release及最终strict签名通过。17 Python、389 DTO、工程/fmt与host+kernel全targets Clippy通过，完整原输入/日志见[数学规划证据](../acceptance/pre-alpha.4/development/r43/math-run-planning.json)。

449144c CI38605677497终态为failure：Rust/frontend/dependencies/iOS成功，Mac总超时负例实际首响应超时抢先，原日志和四个实际timeout JSON留存。仅fixture使用已有trusted timer probe隔离竞争回调，原30ms首响应/idle和所有生产timeout不改；新增200ms延迟headers验证真实首响应timer已先唤醒，独立total仍正确触发，实际URLSession/CABI/取消/并行CAS回归通过。这是测试case隔离，未发现或修改生产timer行为。首次原生attempt进程结束但无terminal report，原因与exit未知、不计pass；其部分原件保留，新fresh根重用原已编译程序实际通过。

按成功Cargo compiler-artifact清单和65项测试终态核对、无编译器/被选harness运行后，清理11个普通测试可执行文件428,370,368逻辑字节（约409MiB），逐文件SHA及缺席核对留存；可用约43GiB，库/CLI/examples/App/runtime/证据保留。

本批只是执行规划基础，R4.3.07步骤和Done均不勾选，整体仍34/98。后续继续ACK原intent→真实math worker/KernelCommitPort、真实executed/partial/resultrefs、inspect_result及独立可写scratch；不造主Kernel receipt。用户选择保留系统输入设置，R4.2.03候选与R4.3.04物理锁屏未验且不重问；32最终gate仍not_run、candidate未分配、ready/released=false，完整98任务/32门禁/实际发行仍required。

## D060 · 2026-10-15 实际结果检查、不可变分页与耐久Pi回合（R4.3.07进行中）

inspect_result从已ACK原intent解析原canonical参数hash，使用当前真正Agent作用域消费read_notebook实际签发的result_ref；SourceEndpoint只调度/轮询，heavy projection在独立ResultRuntime。主状态没有重新求值/接纳或文件IO。复用现有StoredResult/ResultStore及真实producer、历史/partial currentness，手工task=None不能消费Agent ref；当前source/task/grant/config/editor变化会令待完成原生请求失效，恢复文本不恢复任何授权。

实现六个part：summary分页保留原Solve解、条件、verified和重数；value按原值路径遍历全部矩阵列及嵌套容器，精确/高精度/Unicode原式保持；diagnostics分页读原消息；steps深度优先读取真实记录节点及原path/child_count，不创造计算步骤；geometry逐原channel/object/segment页读取真实坐标/索引；preview无已注册渲染媒体时明确data_only/unavailable且media_ref=null。原result TTL不延长，opaque cursor绑定result/part/limit/完整当前scope，沿用128条/32MiB/10分钟上限；5秒/4Msteps/独立取消、100行/32列/240KiB payload与深度/route预算均有实际拒绝。

6项新增数学检查测试与8组定向42项、Host+CABI全154项/0失败、host全targets Clippy -D warnings、18 Node26/Pi、strict自身TypeScript(skipLibCheck)、17 Python、389 DTO/工程/fmt通过。实际测试覆盖35列×2行70个不同元素、7页嵌套精确与高精度值、20重根7页、12条完整原步骤、ODE原record、384曲面顶点44页及独立数学残差，原checkpoint bytes逐字不变；无步骤/无几何/真实Error/过期/改query或权限/取消/限步保持明确。unit里的KernelWorker真实但acceptance receipt为synthetic matching契约，不能替代以下物理存储证明。

standalone及签名App资源实际四个三轮case（有理数、Solve、ODE、scene）：每例29frames(16host/13helper)、21SQLite事件、3次URLSession回环，KernelCommitPort先实际计算/同库接纳→read真实ref→独立inspect→耐久ReadonlyRecord Blob/SQLite回读/native确认→Pi，第三HTTP body逐字含原结果。检查阶段实际accepted checkpoint ID/hash不变、source/history无写；手工作用域原ref拒绝，storage-only重开21事件/2只读完成且无host/helper/旧grant。原两轮/预览三轮/原子修改四轮/5实际边界及强杀后手工4继续通过。模型route/context工厂仍合成fixture配真实存储，不冒称Registry/UI/live完成。

新增Pi测试先发现原validator不支持JSON Schema多类型type数组，合法JsonPayload被IPC_INVALID_TOOL_RESULT拒。保留RED日志/原protocol.ts后补齐类型union，六种JSON payload及无effect operation_ref规则通过；inspection完成可忠实描述Error/stale数学状态，不把它变成数学成功。源码/原输入/HTTP/wire/原SQL压缩与SHA见[实际结果检查证据](../acceptance/pre-alpha.4/development/r43/math-inspect-native.json)。

本轮在154/0终态、原生签名fixture/strict校验通过且无活跃编译器或harness后，按成功Cargo compiler-artifact清单仅清26个target/debug/deps普通Mach-O测试可执行文件（含unit harness），共715,854,624逻辑字节（约683MiB）；先记录每文件SHA/大小/类型，删除后核对缺席，库/CLI/examples/App/runtime/证据保留，可用约43GiB。

R4.3.07仍进行中、全部Steps/Done未勾，实际run_cells执行/耐久executed与partial链、独立可写scratch/result origin仍required。整体34/98、32最终gate全部not_run、candidate未分配、ready/released=false；用户OS输入设置保留、不重问/不改，真实候选与物理锁屏仍未验。无付费live/子代理/本机iOSsim，dev/no main/旧公开tag和包不动，完整98任务/32门禁/实际发行继续。

## D061 · 2026-10-15 独立可写scratch、独立结果来源与实际四轮回合（R4.3.07步骤3完成）

ACK原evaluate_scratch意图的原参数hash→当前实际read-issued snapshot→真实accepted parent→独立scratch CAS线程→独立readonly结果来源→Swift耐久Blob/SQLite回读/native确认→Pi已接通。definitions=true恢复真实定义/Out/random并按当前源码和计算配置非求值reconcile，只检查表达式所需继承定义是否就绪；false用无主定义的纯Session。独立插格入口只用于隔离副本，不触发主笔记本源码事务/失效/RunAll；实际run_cell复用可传入的Interrupt，restore/计算/编码共同受真实host时钟、min(5秒/配置/调用者期限)、4Mstep与独立cancel约束，无文件/配置/model owner。

Scratch结果通过真实独立checkpoint字节跨线程（独立document/build，16MiB包上限），在结果worker解码为原RetainedResult，单独32结果/32MiB计账及pin保留；HMAC十分钟只读ref区分origin=scratch/acceptance=isolated，包含真正parent provenance，从未创建NativeKernelReceipt或接纳为主head。inspect_result六个part复用纯投影，但保留notebook/scratch来源差异。普通运行错误返回failed/error和真实已成功记录/ref；unsupported Import保留真实symbolic值，不伪造成已执行IO。

8项新增覆盖局部Set99/原Out2/两独立job得到同一原随机样本、主checkpoint逐字不变；真实未执行前置拒绝，constant/无关/title/不继承可运行、改owner失效；原hash/闭合参数/作用域/TTL、restore及实际求值中限步/取消；暂停scratch时独立主结果查询仍能完成；返回ref实际inspect为scratch。Kernel+Host+CABI完整385项/0失败、host+kernel全targets Clippy -D warnings、19 Node26/Pi、strict自身TS(skipLibCheck)、17 Python、389 DTO/工程/fmt、Release与最终strict签名通过。unit parent KernelWorker真实，matching receipt仅synthetic单元契约，以下另有物理证明。

standalone和签名App资源各实际5个四轮case：局部Set/Out/random、Solve、ODE、scene、真实递归错误保留2条成功记录。每例38frames(21host/17helper)、28SQLite事件、4次真实URLSession回环，第三/第四HTTP body逐字含原scratch/inspect结果；Error case保持readonly_failed，其余完成。检查原accepted checkpoint ID/hash不变、无source history，实际Document SQLite仅2 kernel_operations（bootstrap+原main）/1 accepted_result；scratch没有主回执。Storage-only重开原28事件/3工具不创建host/helper/旧grant；原4个inspect场景及两轮read、预览、四轮apply/5边界/强杀后手工4继续通过。合成provider/context工厂不冒称Registry/UI/live完成。

原失败均保存：调用private方法编译拒；通用源码协调器追加格令继承定义被清理的RED；预算decode原误映射；InputForm空格断言；混合方言输入没有成功prefix，Decimal坏输入实际保留symbolic而非fatal error。最终采用原Kernel测试已有Wolfram分行递归错误，不改变数学行为来迁就fixture。首次原生exit133的完整输入/HTTP/wire/SQL原件也归档。证据见[独立scratch/Pi](../acceptance/pre-alpha.4/development/r43/math-scratch-native.json)。

本轮按成功Cargo test-profile executable清单、385/0终态、签名fixture/strict签名、无活跃compiler/harness核对，仅清74个target/debug/deps普通Mach-O测试harness，2,304,030,944逻辑字节（2.15GiB），逐文件SHA/大小/类型和删除后缺席留存；库/CLI/examples/App/runtime/证据保留，可用约41GiB。

只勾R4.3.07步骤3；仍须真实run_cells按selected/required/affected执行、组/子操作的真实executed/partial/unknown/结果ref/原回执恢复，步骤1/2/4和Task Done不勾，整体34/98。32最终gate仍not_run、candidate未分配、ready/released=false；完整98+32门禁+实际发行required。用户OS输入设置保持，真实候选/物理锁屏未验；无子代理/本机iOS模拟器/付费live，dev/no main/旧公开tag包不动。

上一提交b6825f1 CI38616794772：Rust/frontend/dependencies成功；iPhone远端xcresult核对35/0失败，iPad在启动simctl bootstatus时由CoreSimulator devicePair内部断言abort134，尚未执行iPad测试，不算平台通过。原job完整日志、远端xcresult及本机仅xcresulttool读取摘要留存；未启动本机模拟器，未改验收断言或1秒门槛。Mac仍运行，完整本批CI待新提交单独核对。


## D062 · 2026-10-15 显式计划的真实主计算执行器（R4.3.07进行中）

- 冻结原RunCells intent/参数SHA、当前真实read-issued snapshot、scope、general SHA和实际accepted parent；PreparedRun只暴露不可变规划/原continue_on_error。新增RunExecutor使用现有KernelEndpoint/KernelWorker/WorkingSession，按真实selected/required/affected拓扑逐格执行，不调用RunAll。bootstrap及每child身份由原operation/原argsSHA/序号稳定派生；新的模型调用保留独立身份。
- 新增AwaitingAdmission阶段：每个bootstrap/child先返回固定RunAdmission，未获trusted native acknowledgment之前不创建主kernel operation、不求值；后续Swift组admission/start持久ACK必须接到该边界，本批尚未实现该存储接线。报告不是可恢复grant，无法Deserialize重建执行器。
- 真实候选仍需原Blob/hash、最终barrier和原receipt后才列入accepted_cells；bootstrap回执单列。只允许该运行组当前child进行physical command，barrier重查真实task/grant/editor/source/epoch/config/general/actual head，自己的回执可推进definition_revision；手工source改变停止后续admission。
- 接受真实cell error的成功statement前缀；continue_on_error=true只可继续实际就绪的独立分支。关闭自动reactive时，计划执行仍使用WorkingSession原依赖分析/ready检查；原手工run_current_cell路径保持原配置语义。cycle、blocked和未接纳失败与accepted receipt分开。cancel停止后续并signal原token，已经physical commit获胜的原回执仍保留。
- Unknown立即锁住后续admission，包括unknown和receipt先于下一poll的竞争；仅原ID回执/absence可settle。若原最后一个child实际已完整接纳，可由原receipt证明completed；还有未运行目标则保留partial，绝不新ID重放。
- Rust全量398项/0失败/78套（新增13项真实执行器场景）；Host/Kernel/AppleFFI全targets Clippy-Dwarnings、17项Python、389 DTO、fmt/工程检查通过。Release库已重建，原生C ABI/KernelCommitPort/SQLite真实计算、部分错误、取消、lost ACK/fullsync unknown与原回执恢复全部通过；该项仅验证现有原生路径，不计作新RunExecutor的Swift/Pi联调。已按成功Cargo inventory清理75个普通test-profile Mach-O可执行缓存2,347,068,000bytes（约2.19GiB），保留libs/CLI/examples/Release/evidence/原失败/用户数据，逐项SHA/类型/路径和删除后缺失审计。
- 验证与原始证据见math-run-executor.json/raw；Rust测试中的IO receipt明确是matching synthetic acknowledgment，不冒充Swift/SQLite/Pi联调。本批不改变R4.3.07步骤1/2/4及Task Done：Swift耐久run组/各child实际admission/start/receipt恢复、原Agent producer/C ABI/Pi dispatch、真正Agent scope结果refs仍需继续接通。整体34/98，32最终gate仍not_run，无candidate/发行。
