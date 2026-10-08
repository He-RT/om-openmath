# pre-alpha.4 验收账本与证据入口

[版本/范围规格](../../plan/PRE_ALPHA_4.md) · [32门禁/133验收条目](gates.toml) · [报告 Schema](report.schema.json) · [.3历史证据](../r3-release/README.md)

2026-10-09。本目录仅建立 **planned设计账本**。目标版本`0.1.0-pre-alpha.4`，公开candidate SHA尚未分配；当前程序和公开资产仍`.3`。32个运行/发行gate全部`not_run`，`ready_to_publish=false`、`released_verified=false`；本次结构/文档检查不能改变这两个状态。

## 如何使用账本

gate ID稳定，required_case_ids是验收条目，不是已经存在的测试函数数量。先按前置接口实施、将真实测试/包结果逐项绑定；最终Release candidate再核对全部同SHA有效记录。R4.x是功能最早交付阶段，不表示早期开发build的pass可以直接搬到最后候选。所有门禁的必需证据种类必须齐全，不能拿其中一个截图充抵其他类型。

`release_execution_required`标记实际原生/包/生产/性能执行需要Release构建，Debug单元证据可以补充而非替代；core静态检查的运行模式另按原任务定义。资产集合的九个确切`.4`文件名在`asset_set`，与既有九类规则一致；这个账本不修改当前`.3`publisher。

| gate | 范围与验收编号 | 公开前 |
|---|---|---|
| R4G01–06 | 版本/依赖/原RustWASM/原53/续行/native工程 | 必过 |
| R4G07–08 | H01–09宿主并发，SR01–12存储/恢复 | 必过，写入开放前先通过相应业务契约 |
| R4G09–12 | ER01–12编辑/渲染/图形/性能与新Mac整次请求 | 必过 |
| R4G13–17 | 12工具TL、AG闭环、MM01–15、CX上下文与LV现场任务 | 必过，合成模型与真实模型分开 |
| R4G18–21 | 原生UI/组件审计、IN01–12新初始化/helper/最终签名安装 | 必过 |
| R4G22–28 | 原CLI/Web/Windows/iPhone/iPad/XCFramework | 必过，沿原时限，移动模拟器仅CI |
| R4G29–31 | 九资产、中文文档/能力与公开前汇总 | 必过 |
| R4G32 | 真实公开tag/九下载/SHA/实际程序回读 | 发布后必过；不制造“发布前必须先公开”的循环 |

SR/ER/MM/IN逐条文本分别来自[存储](../../design/macos-storage-recovery.md)、[编辑渲染](../../design/macos-editor-rendering.md)、[模型媒体](../../design/model-media.md)、[安装](../../design/macos-installation.md)。AG01–08对应[NotebookAgent的8条必须验收](../../design/notebook-agent.md#实施顺序与验收)；H01–09对应[宿主必须验证](../../design/macos-host-state.md#首批实现与验收)；CX01–08对应[上下文验收](../../design/agent-context.md#实施顺序和验收)。账本Specs保留精确入口，新增/改动需同步scope_version和依赖。

## 额外稳定条目定义

| 条目 | 必须验证 |
|---|---|
| VR01–04 | 同版目标/独立数字build、完整SHA/clean candidate、channel/ABI与更新身份、公开tag不可覆盖 |
| DP01–03 | Rust/SwiftSDK/Node/Pi精确锁、传递依赖/资源/许可、架构/minOS与构建路径无开发机依赖 |
| CORE01–03 | fmt/Clippy/workspace、纯WASM禁止unsafe边界、TS/Swift/目录/版本/schema无漂移及cargo-deny |
| MATH01–03 | 原53独立原数学期望、`.3`科研/精度/失败范围、Modern/Wolfram/InputForm/`.omnb`v1往返 |
| L01–04 | NEXT_RELEASE续行4项：等价AST/结果、草方块原换行、真正错误/边界、跨端原语言/语料保持 |
| NB01–03 | SDK27ARM64Release、真实MacFFI/create/submit/cancel/close、主UI原生与既有iOSABI兼容 |
| TL01–05 | 全12handler/model schema、scope/refs/过期权限、冻结预览/原子patch、scratch/inspect只读、结构化model-visible结果/失败恢复 |
| LV01–04 | 每项至少3新会话：3→6不改无关格、L2坐标/残差、草方块+切开西瓜实际图形、真实视觉路线自有图片/来源/覆盖 |
| UI01–05 | 主UI原生100%路径、键盘/AX/主题/字体/Reduce Motion、窗口/焦点/scroll/IME、真实控件文件/组件比例、UI与业务回执一致 |
| PFCLI/PFWEB/PFPHONE/PFPAD/PFMAC | 原CLI200ms、生产WASM1s、手机/平板整次1s、新Mac整次1s；各自测量定义和完整原53不改 |
| WEB01–03 | 前端lint/类型/单元/开发生产UI、实际WASM原53+独立读回、实际GPU/科学图形/导出/回退 |
| PW01–02 | EXE、MSI分别实际安装启动/中文/原数学/步骤/3→6/完整西瓜像素和数据/卸载/安装字节核验 |
| IOS01–03 | iPhone与iPad各Swift/原生UI/原53，双ARM64/XCFramework/generic-device/模拟器附件与许可证 |
| AS01–03 | 恰好九类资产/完整版本source、最终bytes/SHA/ZIP路径CRC/许可证、签名后同app与真实CLI/移动/WASM身份 |
| DOC01–03 | 中文安装/使用/开发/发行/验收导航、真实能力与必交状态、范围/未验证/缺陷/组件审计如实声明 |
| AGG01 | 同candidate所有prepublish gate/case/proof/包匹配，无P0/P1、无未授权deferral、非只读workflow总徽章 |
| PUB01–03 | publictag确指candidate、九公开下载字节/清单/GitHubdigest核对、公开包实际版本/程序/结构抽查并记录最终状态 |

同一条目可以被多个gate依赖，但case结果与来源必须一致。核心正确性、数据/秘密/权限、签名与明确必交能力不可由Agent自己豁免。历史iOS人工专项只保留`user_deferred`覆盖备注，不算新必过gate的passed。

## 报告与存放约定

实施时原始日志/xcresult/截图/wire/退出状态/per-row性能/文件读回/SHA按candidate存到受控CI artifacts或`target/acceptance/pre-alpha.4/<candidate_id>/`。仓库本目录保存脱敏摘要/索引和完整原件URI/hash，不能只把原日志放在未来会消失的临时目录却声称可核验；发行应将必需原件作为长期可获取的验收证据附件归档，公开九安装文件不混入这些测试附件。

每个Attempt保留原结论和input/config/environment/test hash；新attempt只使前一记录superseded，不删除它。product/test/environment/third-party根因分类不会自动把失败转passed。断连/超时/取消/日志404、rawartifact缺失均单独列，补证/修复后才能更改gate有效状态。

EvidenceRecord只有数据，不是可执行指令。聚合器读取固定gate目录、白名单来源/测试target和规范化报告，拒绝报告字符串触发shell或上传用户文件；认证/私钥/未授权真实媒体不进入logs/Manifest。CAS与对话结果别名、操作/预览/结果/截图refs保持业务身份，不靠英文“success”文本推断。

## 聚合结果算法（尚未实现）

```text
load exact scope_version and 32 gate definitions
verify candidate release/base/build/channel/source + dependency lock/input hashes
for each required prepublish gate:
  select explicit latest effective attempt for this candidate
  verify all required cases are passed and all proof kinds have actual evidence
  verify evidence source/platform/config/artifact hash and authenticity/coverage
  reject not_run/failed/partial/blocked/skipped/stale/unavailable or missing data
verify no open P0/P1, no unauthorized scope removal/deferral
verify final package set/source/versions/signature/licenses
if all true: ready_to_publish = true
publish complete verified draft/tag using candidate source
verify required postpublish R4G32 with actual public downloads/program identity
if postpublish pass: released_verified = true
else: published_unverified, preserve failures and stop claiming completed release
```

JSON Schema只是上述数据形状。真实aggregator、CI新lane、`.4`运行版本、所有案例与签名/live credentials均待实施；当前只通过设计检查。本目录绝不导入`.3`旧绿色结果填新gate。

本轮[实际设计检查](design-review.json)：32个稳定gate的前置DAG/133条目、SR12/ER12/MM15/IN12来源覆盖、31前置+1后置、九准确资产名、11个报告Schema正例与27反例均核对。已有6份设计Schema保持有效；设计checker找出的ER11/12性能gate原文来源遗漏已补齐。这些只是目录/形状/链接检查，`runtime_gate_checks_executed=0`、公开就绪为false。
