# pre-alpha.4 实施进度

[完整实施计划](PRE_ALPHA_4.md) · [机器门禁](../acceptance/pre-alpha.4/gates.toml) · [裁决](DEVIATIONS.md) · [问题](QUESTIONS.md)

2026-10-09。目标`0.1.0-pre-alpha.4`，当前运行/公开版本`.3`；开发`dev`，不合main、不改旧标签、本机不启动iOS模拟器。本文只记录结果，规格及完成标准唯一见主计划；计划编写本身不计实施完成。

**当前状态：** 10任务完成（R4.0.01/02/03/04/05/06/07/08/09/11）；其余88任务尚未完成；32最终运行/发行gate全部not_run；candidate未分配，ready_to_publish=false，released_verified=false。设计/HTML/Schema通过不填运行时pass。

**下一任务：** R4.0.10故障fixture基础与R4.1.01 SQLite单写者（原生宿主/事件/确认投影已接通）；R4.0.10故障fixture可按接口前置开始。R4.0.08/09同提交全部CI已通过。按DAG推进，R4.5.07必须在R4.5.05冻结前完成。

## 已知外部前置

- 原生工程与真实Rust元数据链接已经实现；完整宿主、存储及Agent尚未接通，运行源码版本仍`.3`。
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
| [x] [R4.0.07](PRE_ALPHA_4.md#task-r4.0.07) | Swift客户端、事件泵和权威投影 | completed | 纯reducer / 真实ABI丢帧恢复 / 原生AX运行与停止 | 下次补记 |
| [x] [R4.0.08](PRE_ALPHA_4.md#task-r4.0.08) | 共享Modern自动续行与根因诊断 | completed | 91 DTO / Rust+Swift / CI37880165376 | 21c9254 |
| [x] [R4.0.09](PRE_ALPHA_4.md#task-r4.0.09) | 原语言和数学基线持续兼容 | completed | 91 DTO / Rust+Swift / CI37880165376 | 21c9254 |
| [ ] [R4.0.10](PRE_ALPHA_4.md#task-r4.0.10) | fixture与真实故障注入基础设施 | planned | — | — |
| [x] [R4.0.11](PRE_ALPHA_4.md#task-r4.0.11) | 发行前置、签名和现场环境预检 | completed | [环境](../acceptance/pre-alpha.4/baseline/environment.json) / verify-env.sh | e876c16 |
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
