# pre-alpha.4 交付检查（scope 2）

[当前计划与顺序](../../plan/PRE_ALPHA_4.md#0-执行守则与完成语义) · [实际进度](../../plan/PRE_ALPHA_4_PROGRESS.md) · [机器账本](gates.toml)

2026-10-15用户明确要求优先macOS与Web、保留性能、取消妨碍下个pre-alpha的形式工作。本轮只有8项`delivery_check`作为完成条件，全部当前not_run；候选未分配，两个发行状态仍false。原32项`gate`/133条目保留作scope 1参考，mandatory/pre/post均false，不表示这些功能已通过或未来不做。98任务是详细待办，不再全数阻塞本版。

| 交付检查 | 实际结果 |
|---|---|
| R4D01 | Release Mac自包含启动、生产Web/WASM构建与启动 |
| R4D02 | 两端原53独立数学正确性、科研/精度/语法与.omnb兼容 |
| R4D03 | 新建/编辑/中文输入/undo/计算/停止/保存/退出重开 |
| R4D04 | 真实公式、Markdown、步骤、矩阵、2D和基础3D；已有Web导出 |
| R4D05 | Mac基础模型配置和Pi读/改/运行/检查/停止；Web原能力不退化 |
| R4D06 | CLI原53每条<200ms，Mac整次/Web WASM每条<1s，两端10KiB输入p95≤50ms |
| R4D07 | 数据恢复、原ID未知效果核对、权限和秘密边界 |
| R4D08 | Mac DMG/ZIP与Web ZIP、简明清单、真实安装和公开下载/程序回读 |

ready_to_publish要求R4D01–07及R4D08打包/安装部分通过；公开后完成R4D08下载回读，才设置released_verified，避免发布前必须先公开的循环。

开发只跑受影响检查，优先复用已有测试；低风险可逆改动不机械新增测试。候选集中验收，修复后按影响重测，不每批跑全workspace/原生故障矩阵/全平台。保持原数学期望与性能阈值，不把被省略的检查写成passed。

Windows/iOS/XCFramework与CLI分发、全媒体协议/12工具矩阵、自动记忆/压缩/FIM、组件比例/完整人工无障碍专项、极限负载和九资产汇总移到后续。Mac保留可运行签名和真实安装；公证材料可用则做，缺失按实验性pre-alpha如实说明签名和Gatekeeper情况，不等待证书才开发/交付。

证据只记结果、实际输入/版本与关键日志。复用提交与CI artifacts，不逐批复制源码/数据库/全部attempt资源，不为纯记账重建/重签名。原历史证据保留，报告Schema仍可读取旧记录，但不要求先开发完整聚合器。资产集合以机器账本`asset_set`的3项为准；原`.3`发行脚本/公开包保持原样，本次不提前切版本或发布。

## scope 1条目定义（仅参考，不是本轮完整必过集合）

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
