# OpenMath 当前接续入口

> **2026-10-16 起的下一版：pre-alpha.5。** 执行入口是 [docs/plan/PRE_ALPHA_5.md](docs/plan/PRE_ALPHA_5.md)，按它第 0 节先读 [工作台 v2 设计](docs/design/workbench-v2.md) 并打开 [交互草案](docs/design/prototypes/workbench-v2.html)，再看它第 6 节进度表。每个界面任务开工前、完成后都要对照草案截图。下面的 `.4` 内容只作历史参考。

> **2026-10-19 状态：** A–D、E1–E4已实现并按任务提交；下一步 **E5（Web步骤与变量）**。E4已接250高二维/下方图例/悬停控制、400高主题三维/复位全屏、默认收起图形信息、状态行动作与SVG/PNG/CSV/OBJ导出；受影响24单测、真实浏览器6项、最终类型/构建/lint通过，失败日志保留。CUA1440浅深色、1000/390浅色图在`target/plan-5/e4-*.png`；手机无横向溢出，实际三维拖动已看图。视口已reset，自有后台e1Tab已markHandoff；生产预览5173 pid29595（先核对存活，exec59752），旧用户4175未改。预算27/60，余33次。**用户最新授权API端点为`http://localhost:10100/v1`，模型仍sol/luna；旧记录4Router已过时，真实调用前须核对唯一私有配置，本项未读取/修改凭据。** G1仍须真实中文候选、文件拖放、原生附件横向手势与完整设计矩阵；G3/G1仍须核对k*x==1内核缺condition字段；G2须图片/PDF真实发送。file://原型交互受工具限制，未绕过、未获例外批准。E5–G与整版交付未完成；旧历史快照不替代候选包性能，存储和.omnb版本不变。


接续临时资料在同机 `target/plan-5/`：`live-task-runs.jsonl` 是调用账本，`live_case.py` 准备隔离场景（不发模型请求），`live-ui/data/` 保存场景与宿主回读；不复制进新的证据框架。当前 Release 构建、提示词、操作记录、端点与撤销检查已通过。A5 完成后按 B→C→D→E→F→G 继续，不重做 A1–A4。`.5` 发布按新计划 G4：先完成可审阅安装包，用户确认后创建 GitHub pre-release。

**以下均为 `.4` 的历史接续记录；其“当前候选”“已授权发布”及旧完成条件不适用于 `.5`。新设计、计划及本页顶部状态优先。**

更新：2026-10-16，scope 2；只推进原生 Mac 与生产 Web 的下一个 pre-alpha。历史在 Git 与 `docs/plan/PRE_ALPHA_4_PROGRESS.md`，这里不叠加失效的下一步。

## 用户现场复核覆盖此前完成声明

用户2026-10-16截图指出实际产品明显偏离原设计，不能继续沿用“计划完成”。发布/下载/数学性能事实成立；R4D04/R4D05已改failed，ready/released_verified=false，公开tag不动。当前只是可安装且基本读改算通道已通，完整UX/Agent未完成。不要重复泛看App或用旧green覆盖此问题。

已定位：NativeAssistantView固定TextEditor与全局字符串trace，缺设计中的输入框模型/模式和按消息动作；NativeMarkdown的代码块被扁平化为普通带背景run而没有独立横向滚动；AssistantContext只用一段短system，没有接入原笔记本操作指引/语言图形短例；loop_stopped直接显示已完成，未验证用户目标；文字格默认reading=false。截图里的emoji写入不能算真实绘图。截图“正在执行”不能单凭静态图定为卡死，尚需单独复现；不要擅自停止用户活动任务。详见进度D074。

## 先读，避免重复

1. `docs/plan/PRE_ALPHA_4.md` 第0节：本版只使用8项 delivery_check，旧98任务/32gate是参考。
2. `docs/plan/UI_REFERENCE_LOG.md`：已看应用、确切页面、采用理由、已实现状态。**不要再次泛看已看页面**；只为具体未解产品问题补看。
3. `docs/acceptance/pre-alpha.4/delivery-summary.md`：当前候选验收与限制；`docs/acceptance/pre-alpha.4/gates.toml`是发行状态。

## 不变约束

- 用户要求单代理；不用子代理、不启动本机iOS模拟器、不合main、不移动旧tag/旧发行资产。dev push和本版实际GitHub pre-release已授权。
- 不改系统输入法、安全或权限设置，不再问日文输入法；不扫描旧OpenMath配置、env、密钥或用户数据。
- 只允许精确读取 `/Users/hert/.config/openmath/testing/pre-alpha-4-api.json` 的授权API，目录0700/文件0600；模型gpt-6.1-sol / gpt-6-luna。不得输出密钥或认证头，泄漏检查只报告精确计数。
- 冻结13份 `docs/design/` 来源和baseline SHA不改。外部 `.gitignore` 的 `ui-concept-demo/` 规则保留、不stage、不检查该目录；未知归属 `test-results/` 不stage/不删。
- 仅清理自有、已停止、可重建缓存；不删独立数学验证器、失败根、发行包、依赖或当前构建。本轮已清5个旧fixture约256MiB，约32GiB空闲，暂不大清理。
- Rust `CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0`；build.rs散列全部.rs，编译期间不改Rust/测试输入。不因观察超时重启确切存活句柄。

## 唯一当前候选

- `dev/origin`生产与测量入口提交：`e1d6afd11a48dec807b0c6cce09938adaeeffdd6`。前提交baa075d7修复助手参数错误回路、完整模型工具说明、求根候选热点和状态轮询。
- 当前 `.4`，Root1/Library4/Doc5/.omnb1不变。发布、匿名下载、安装/程序读回已完成；用户现场复核发现产品缺陷，R4D04/R4D05failed，ready/released_verified=false。
- 包目录：`target/pre-alpha.4/e1d6afd11a48dec807b0c6cce09938adaeeffdd6/release-assets`；三个包和manifest齐备，勿用689/486旧包。
- 实际DMG安装：`/Users/hert/Applications/OpenMath pre-alpha.4 e1d6afd1/OpenMath Preview.app`，严格签名、包内Node26.11.1/Pi通过。实际About `.4`/完整e1d；中文撤销重做、精确2/3、运行中手动停止后2+2、保存退出重开均通过。自己的文件 `target/plan-scope2/新包验收-e1d6afd1.omnb`；此候选副本已退出；当前运行公开下载副本，见下文。
- Web从实际ZIP解压目录 `.../e1d.../web-installed` 提供127.0.0.1:4174（旧句柄88871已停止；当前公开包4175句柄99745）。原35项生产检查与独立原53全部通过，浏览器设置读回 `.4`/Rust `.4`/完整e1d。
- 当前安装资源live `.../e1d.../live-installed`：sol七工具 read/read/preview/apply/read/run/inspect成功，原Blob核SHA独立读回当前已接纳x→2/3。当前候选5603文件密钥精确匹配0。不要无故再次调用收费模型。

## 已有有效验收，勿重跑全矩阵

- 原53 CLImax166.721292ms、Mac submit→MainActor整次max813.039833ms、WASMmax229ms/往返231ms。原精度/误差/正确性由优化前独立验证器核SHA验证，三端全通过。
- Mac正常AppKit前台，100次10KiB输入p95=49.620792ms/max83.137458ms；Webp95=31.600000ms/max51.700000ms。原50ms门槛未改；旧超标/未完成根保留，没有重测挑最快。
- 原生首workspace909.1125ms/RSS188547072字节不是整个应用冷启动。WebFCP148ms，实际SwiftShader Vulkan。未变的普通相机7936三角形约60Hz、静止0新帧。
- 原生Markdown/SwiftMath/表格/步骤/CoreGraphics2D/Metal3D完整应用视觉已经通过。参考记录已固化，不再泛看应用。
- 当前真实内核回归已通过早停回滚、晚停前缀、丢ACK、真实unknown原ID核对、长CAS编辑；入口 `target/plan-scope2/current-kernel-regression.txt`。
- 23 Node/Pi/protocol与om-poly138项/20suites、Clippy已通过。旧Pi完整fixture最后group恢复inUse原失败保留；独立七类撤销/重启及50次实际Storage close/reopen通过，未改锁，勿再发明故障平台。

## 已公开资产事实（不代表设计完成）

- 公开发行：https://github.com/He-RT/om-openmath/releases/tag/v0.1.0-pre-alpha.4；新tag精确e1d，旧tag/资产未改。
- 三个公开资产与manifest匿名下载，字节/SHA全部相同；原始文件/核对/实际UI证据 `.../e1d.../public-download/`。公开DMG安装至 `/Users/hert/Applications/OpenMath pre-alpha.4 public download/OpenMath Preview.app`，当前实际运行此路径，About .4/完整e1d，中文文件及实际2/3/4通过。公开appZIP4224文件与DMG安装逐一一致。
- 当前公开Web解压目录 `.../e1d.../public-download/web-installed` 在127.0.0.1:4175（确切句柄99745），实际−3/1、.4/Rust .4/完整e1d已读回。浏览器绑定finalWebPreview为用户保留。旧4174已停止。
- 删除本轮已停止的ZIP比较临时解压235579847字节；保留原下载资产、失败与独立验证器。
- CI38906287178已全部成功：frontend、dependencies、Rust、native_macos；iOS按本轮范围跳过。旧验收对图形目标和交互覆盖不足，已被上述现场复核更正。后续按实际产品问题推进，不重新启动旧98/32工作或遍历参考应用。

## CUA

压缩后先rewriteDocumentation。当前公开原生绑定publicMac，Web绑定finalWebPreview；公开实际截图在当前e1d根public-download/mac-result.png及web-result.png。getAXState可能自动重启已退出app，退出验证用精确ps。截图后取fresh AX再用index；中文用paste/setValue且读回。UI只用CUA，禁止Shell/AppleScript控制。
