# pre-alpha.4 实施进度

[完整实施计划](PRE_ALPHA_4.md) · [机器门禁](../acceptance/pre-alpha.4/gates.toml) · [裁决](DEVIATIONS.md) · [问题](QUESTIONS.md)

2026-10-11。目标`0.1.0-pre-alpha.4`，当前运行/公开版本`.3`；开发`dev`，不合main、不改旧标签、本机不启动iOS模拟器。本文只记录结果，规格及完成标准唯一见主计划；计划编写本身不计实施完成。

**当前状态：** 19任务完成（全部R4.0与R4.1.01–08）；其余79任务尚未完成；32最终运行/发行gate全部not_run；candidate未分配，ready_to_publish=false，released_verified=false。设计/HTML/Schema通过不填运行时pass。

**下一任务：** R4.1.09继续：已有真实worker和独立Blob/SQLite耐久接纳，接通NativeHost/CABI/Swift运行入口、共享source逻辑门与接纳事件。真实逆事务、原生文本/IME group、200+pin及物理裁减回执已通过；完整工作台与Agent仍继续后续任务。保持同candidate完整门禁，R4.5.07在R4.5.05冻结前完成。

## 已知外部前置

- 原生宿主、事件恢复和首代/source物理存储已运行；DocCommitPort已通过实际跨语言验证；完整工作台、checkpoint及Agent尚未接通，运行源码版本仍`.3`。
- 上轮只读预检没有Developer ID Application材料；执行阶段复核并按R4.0.11准备接口，真实公开签名门禁不能降级。
- 现场模型只使用新Registry中的授权测试配置；不自动读取旧Key/环境/profile。当前live gate未运行。

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
| [ ] [R4.1.09](PRE_ALPHA_4.md#task-r4.1.09) | 主KernelWorker、候选接纳和直接取消 | in_progress | [worker](../acceptance/pre-alpha.4/development/r41/main-worker.json) / [同库接纳](../acceptance/pre-alpha.4/development/r41/kernel-acceptance.json)；实际Host/共享逻辑门待完 | 3ce0320；本批待补 |
| [ ] [R4.1.10](PRE_ALPHA_4.md#task-r4.1.10) | 不可变结果库和只读检查通道 | planned | — | — |
| [ ] [R4.1.11](PRE_ALPHA_4.md#task-r4.1.11) | NSDocument打开保存与冻结快照 | planned | — | — |
| [ ] [R4.1.12](PRE_ALPHA_4.md#task-r4.1.12) | 会话outbox、草稿与启动恢复 | planned | — | — |
| [ ] [R4.1.13](PRE_ALPHA_4.md#task-r4.1.13) | 引用租约、备份、quota与GC | planned | — | — |
| [ ] [R4.2.01](PRE_ALPHA_4.md#task-r4.2.01) | 原生工作台窗口、分栏和投影 | planned | — | — |
| [ ] [R4.2.02](PRE_ALPHA_4.md#task-r4.2.02) | NSTextView和严格SourceIndexMap | planned | — | — |
| [ ] [R4.2.03](PRE_ALPHA_4.md#task-r4.2.03) | 原生IME、UndoManager与草稿屏障 | planned | — | — |
| [ ] [R4.2.04](PRE_ALPHA_4.md#task-r4.2.04) | 非求值Preview/Complete/Hover和高亮 | planned | — | — |
| [ ] [R4.2.05](PRE_ALPHA_4.md#task-r4.2.05) | 本地snippets、希腊快捷、Fix和缩进 | planned | — | — |
| [ ] [R4.2.06](PRE_ALPHA_4.md#task-r4.2.06) | 长笔记本布局、复用与scroll anchors | planned | — | — |
| [ ] [R4.2.07](PRE_ALPHA_4.md#task-r4.2.07) | SwiftMath原生公式和原式fallback | planned | — | — |
| [ ] [R4.2.08](PRE_ALPHA_4.md#task-r4.2.08) | Markdown AST、数学扫描与连续原生流 | planned | — | — |
| [ ] [R4.2.09](PRE_ALPHA_4.md#task-r4.2.09) | 结果状态、解条件Root与数值投影 | planned | — | — |
| [ ] [R4.2.10](PRE_ALPHA_4.md#task-r4.2.10) | 矩阵、表格、记录与科学诊断 | planned | — | — |
| [ ] [R4.2.11](PRE_ALPHA_4.md#task-r4.2.11) | 真实步骤、变量与独立讲解入口 | planned | — | — |
| [ ] [R4.2.12](PRE_ALPHA_4.md#task-r4.2.12) | CoreGraphics二维和实际采样控制 | planned | — | — |
| [ ] [R4.2.13](PRE_ALPHA_4.md#task-r4.2.13) | Metal三维原生基础管线 | planned | — | — |
| [ ] [R4.2.14](PRE_ALPHA_4.md#task-r4.2.14) | 参数探索、视窗保留和真实图形预览 | planned | — | — |
| [ ] [R4.2.15](PRE_ALPHA_4.md#task-r4.2.15) | 原生菜单、命令、文件导出和复制 | planned | — | — |
| [ ] [R4.2.16](PRE_ALPHA_4.md#task-r4.2.16) | 原生主题、可访问语义与初步组件审计 | planned | — | — |
| [ ] [R4.2.17](PRE_ALPHA_4.md#task-r4.2.17) | 本地计算首启、欢迎恢复与助手空态 | planned | — | — |
| [ ] [R4.2.18](PRE_ALPHA_4.md#task-r4.2.18) | 关于、手工更新、诊断与新数据管理 | planned | — | — |
| [ ] [R4.3.01](PRE_ALPHA_4.md#task-r4.3.01) | 规范Agent消息、任务与IPC身份 | planned | — | — |
| [ ] [R4.3.02](PRE_ALPHA_4.md#task-r4.3.02) | 固定Pi Core与随包生产helper | planned | — | — |
| [ ] [R4.3.03](PRE_ALPHA_4.md#task-r4.3.03) | sans-IO单轮模型codec基础 | planned | — | — |
| [ ] [R4.3.04](PRE_ALPHA_4.md#task-r4.3.04) | Swift URLSession和SecretStore传输 | planned | — | — |
| [ ] [R4.3.05](PRE_ALPHA_4.md#task-r4.3.05) | 读取笔记本与真实函数资料工具 | planned | — | — |
| [ ] [R4.3.06](PRE_ALPHA_4.md#task-r4.3.06) | 预览与原子修改工具接入 | planned | — | — |
| [ ] [R4.3.07](PRE_ALPHA_4.md#task-r4.3.07) | 执行、结果检查和隔离试算工具 | planned | — | — |
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
