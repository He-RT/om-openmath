# pre-alpha.4 实施进度

[完整实施计划](PRE_ALPHA_4.md) · [机器门禁](../acceptance/pre-alpha.4/gates.toml) · [裁决](DEVIATIONS.md) · [问题](QUESTIONS.md)

2026-10-09。目标`0.1.0-pre-alpha.4`，当前运行/公开版本`.3`；开发`dev`，不合main、不改旧标签、本机不启动iOS模拟器。本文只记录结果，规格及完成标准唯一见主计划；计划编写本身不计实施完成。

**当前状态：** 2任务完成（R4.0.01/11）；其余96任务尚未完成；32最终运行/发行gate全部not_run；candidate未分配，ready_to_publish=false，released_verified=false。设计/HTML/Schema通过不填运行时pass。

**下一任务：** R4.0.08续行修复正在实施和验证；随后推进R4.0.02依赖/许可闭包与原生工程。基线和环境预检已完成。先读完整计划第0节。按DAG推进，R4.5.07必须在R4.5.05候选冻结前完成，不机械按编号。

## 已知外部前置

- 原生工程/宿主/Agent均尚未实现，Rust/程序版本仍`.3`。
- 上轮只读预检没有Developer ID Application材料；执行阶段复核并按R4.0.11准备接口，真实公开签名门禁不能降级。
- 现场模型只使用新Registry中的授权测试配置；不自动读取旧Key/环境/profile。当前live gate未运行。

## 任务账本

同次实现提交更新任务状态、主计划复选框与下面记录；SHA在后续记账避免自引用。`[x]`只表示该任务Done/Tests满足，不自动改变最终candidate门禁。blocked应写具体前置并继续独立任务。

| 任务 | 目标 | 状态 | 验证/证据 | 提交 |
|---|---|---|---|---|
| [x] [R4.0.01](PRE_ALPHA_4.md#task-r4.0.01) | 建立基线、任务账本和恢复入口 | completed | [基线](../acceptance/pre-alpha.4/baseline/index.json) | 下次补记 |
| [ ] [R4.0.02](PRE_ALPHA_4.md#task-r4.0.02) | 依赖与许可闭包核对 | planned | — | — |
| [ ] [R4.0.03](PRE_ALPHA_4.md#task-r4.0.03) | 原生Mac工程和数学链接骨架 | planned | — | — |
| [ ] [R4.0.04](PRE_ALPHA_4.md#task-r4.0.04) | 冻结框架无关DTO与新MacABI契约 | planned | — | — |
| [ ] [R4.0.05](PRE_ALPHA_4.md#task-r4.0.05) | 安全host-service所有者与调度骨架 | planned | — | — |
| [ ] [R4.0.06](PRE_ALPHA_4.md#task-r4.0.06) | 新CABI句柄、缓冲与直接取消 | planned | — | — |
| [ ] [R4.0.07](PRE_ALPHA_4.md#task-r4.0.07) | Swift客户端、事件泵和权威投影 | planned | — | — |
| [ ] [R4.0.08](PRE_ALPHA_4.md#task-r4.0.08) | 共享Modern自动续行与根因诊断 | in_progress | — | — |
| [ ] [R4.0.09](PRE_ALPHA_4.md#task-r4.0.09) | 原语言和数学基线持续兼容 | planned | — | — |
| [ ] [R4.0.10](PRE_ALPHA_4.md#task-r4.0.10) | fixture与真实故障注入基础设施 | planned | — | — |
| [x] [R4.0.11](PRE_ALPHA_4.md#task-r4.0.11) | 发行前置、签名和现场环境预检 | completed | [环境](../acceptance/pre-alpha.4/baseline/environment.json) / verify-env.sh | 下次补记 |
| [ ] [R4.1.01](PRE_ALPHA_4.md#task-r4.1.01) | SQLite单写者与新通道初始化 | planned | — | — |
| [ ] [R4.1.02](PRE_ALPHA_4.md#task-r4.1.02) | 权威源码、事务和幂等表 | planned | — | — |
| [ ] [R4.1.03](PRE_ALPHA_4.md#task-r4.1.03) | 不可变Blob和资源引用发布 | planned | — | — |
| [ ] [R4.1.04](PRE_ALPHA_4.md#task-r4.1.04) | 文档操作与非执行失效计划 | planned | — | — |
| [ ] [R4.1.05](PRE_ALPHA_4.md#task-r4.1.05) | 冻结Preview与新格身份分配 | planned | — | — |
| [ ] [R4.1.06](PRE_ALPHA_4.md#task-r4.1.06) | DocCommitPort、编辑屏障与未知提交 | planned | — | — |
| [ ] [R4.1.07](PRE_ALPHA_4.md#task-r4.1.07) | 统一源码撤销和后续编辑冲突 | planned | — | — |
| [ ] [R4.1.08](PRE_ALPHA_4.md#task-r4.1.08) | 完整可写计算状态与无损codec | planned | — | — |
| [ ] [R4.1.09](PRE_ALPHA_4.md#task-r4.1.09) | 主KernelWorker、候选接纳和直接取消 | planned | — | — |
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
