# OpenMath 当前接续入口

> **pre-alpha.5 已公开发布，按用户调整后的范围收尾。** [公开发行页](https://github.com/He-RT/om-openmath/releases/tag/v0.1.0-pre-alpha.5) · [计划与最终记录](docs/plan/PRE_ALPHA_5.md) · [安装与发布记录](docs/acceptance/pre-alpha.5/local-install.md)。A–G3及本地三包安装已通过；公开后验证由用户明确取消，不能记为实测通过。视觉依据仍为 [workbench-v2.md](docs/design/workbench-v2.md) 和四张原型截图。

## 2026-10-21 最新发布与文档收尾

- **用户追加 CLI 补包已完成。** macOS ARM64 / Windows x64现有终端功能，不加助手、不重跑GUI或真实模型。专用run`39539290622`两平台构建、实际ZIP解压后的.5版本及求根2/3均通过；两包及独立`cli-release-manifest.json`追加到原.5 Release，产品源码仍`86da0620…`，原tag/三包/原清单不改。具体大小、哈希、最小检查与历史失败见[CLI补包记录](docs/acceptance/pre-alpha.5/cli-release.md)。发行清单6项通过，提交带skip ci避免无关全套回归；只显式派发CLI构建。真实调用仍69/80。
- **发行源码`86da06206c1ce547921ff2fb6ada892c5b090b48`，已获用户明确批准并公开发布`v0.1.0-pre-alpha.5`。** 三包及manifest均上传成功，后续仅文档提交不改变包内revision。新增现代NSApplicationDelegate.application(_:open:)文件URL回调与根视图代理绑定；现有WorkbenchFixtures --file-events及完整AppHost Finder冷启动两包均通过，下方旧“欢迎页未处理”状态已过时。
- 三包与manifest位于`target/pre-alpha.5/86da06206c1ce547921ff2fb6ada892c5b090b48/release-assets/`，具体哈希/安装位置/实测见`docs/acceptance/pre-alpha.5/local-install.md`。DMG/appZIP4226文件逐字节一致、严格验签、无fixture键；均真西瓜和2/3，ZIP保存→DMG读回重算通过。Mac关于窗口及Web设置均.5+完整SHA；原生两包已正常退出。OpenPanel间歇Open禁用仍保留已知限制，Finder可用；未改文件默认关联或系统权限。
- 同SHA CI`39533915333`已完整success：Rust/frontend/dependencies/native全部通过，iOS按本版范围跳过；原始状态g4-final-ci-status.json。构建`g4-modern-file-events-native-package.log`/`g4-modern-file-events-web-build.log`均exit0。用户在发布成功后要求“不用验证了，同步更新readme等文档”，已取消匿名下载、哈希复核和公开包重复安装回读；不再继续验证或询问发布权限。计划保留原标准并明确记录本次用户调整，G4按调整后的范围完成，不声称原五项全部实测通过。
- 中英文README、文档导航、安装/开发/模型配置/发行指南、发行说明与验收记录已同步到.5。公开GitHub发行正文同步对应说明；旧tag与资产不动，文档更新不重建或替换发行包。
- 真实调用仍69/80，全部已结束，不重跑。用户旧钥匙串问题已回复“已处理”，不再问。不开子代理、不改系统设置、不启动iOS、不stage无关.gitignore/.vscode/test-results，不合main、不动旧tag。所有旧候选/失败保留。
- CUA持久绑定g4FinalZip/g4FinalDmg已退出，勿getAX隐式重启；g4Web为真实解压Web HTTP4180（exec59778），当前无计算。g4Finder仍自有目录。g4NativeRun(app,n)可用；旧g4About闭包捕获7bc1e81d路径，不要复用：它把86da0620的about误存旧目录，已逐字核对并复制到正确86目录，旧目录about-capture-location.txt说明其归属，旧会话工具输出仍保留。后续截图保存应显式传当前root参数。旧自有4176/4177/4178/4179服务已精确停止，本轮DMG均已卸载，包/失败记录保留；旧用户4175不动。

## 2026-10-20 及后续历史记录（最新状态以上节为准）

- `dev`/origin已到 **ef76f794（G1）**，A–G1完成；G2–G4未勾选。每任务完成一笔提交推dev。无关 `.gitignore`、`.vscode/`、`test-results/` 不stage、不删；不合main、不移动旧tag。用户明确不开子代理、不改系统输入法/安全/权限、不启动iOS模拟器、不再看别的App。
- **用户已书面同意原型浏览方式例外**：file://策略拒绝且禁止绕过，允许四张静态图+HTML源码。不要重复询问或改用HTTP/CDP绕过。其余标准保留。
- **第16项真实中文候选已通过**：用户亲测Mac与Web均“只上屏，未开始发送”，2026-10-20已写计划。不要再问，不是模拟或豁免。
- **两项最新已确认，勿再问（2026-10-20）**：用户真实拖入TXT后新增附件卡片；明确同意仅Mac截图高度使用本机808–860替代900，宽1440/1000/800、20项其余部分和数学/性能标准保留。G1的静态模式实测与最终4图已完成，下一项G2。

## G1工作树与验证

- 共享Markdown支持`\(…\)`/`\[…\]`（包括TeX内空白），代码/不完整表达式/货币保持原规则；Transcript同lexer。Mac解析/TextKit检查通过。
- 两端数值积分等非标量输出显示“结构化结果”，不再泛称“精确值”；基于真实kernel输出kind。Web宿主owned `agentWork` 提供具体单元格/scratch运行提示和行内停止；Mac同样具体状态/停止。未配置Mac模型禁用发送，草稿保留，原生用例已通过。
- Rust新增`Solution.required_condition`保留MaxExtraConditions默认隐藏的真实有效性条件；Kernel展示/检查点保留它，默认表达式格式不变。重复分支相同值仍去重，隐藏有效性条件取并集，无条件分支占优。补全部构造器包括`tests/corpus/solve_support.rs`。**最终相关135 suites：776 passed / 0 failed / 1原有ignored**，日志`g1-affected-rust-final.log`；ignored是另要求release手跑的旧微基准，不能当G3性能通过。
- 最新原生实际图已有2/3解与`k≠0`芯片，编辑a的旧结果与“依赖的定义a已改变”真实过期；应用内设置已实际切浅/深，系统设置未改。
- **减少动态效果已完成**：Theme.swift新WorkbenchProgress替代18处不定进度，系统减少动态效果或仅隔离fixture包Window.reduceMotion=true时用静态沙漏/原标签。最终Release`g1-motion-native-final-build.log`成功（exec43421已退出0），完整AppHost实际factor(x^10007-1)运行截图为静态轨道和沙漏，两个时点轨道2325像素差0，显式停止显示已停止（fidelity/g1-motion-frame-a/b.jpg）。系统设置未改，当前隔离G1包reduceMotion=true；正常发行包无该键。首次只读Environment transform编译失败保留，已改为只读系统值OR受保护fixture开关。

- Web最新49受影响单测/lint、9结果/文字生产E2E、2助手真实Worker/事务回环、最终条件E2E1、减少动态效果真实Worker停止E2E1通过。日志`g1-ui-final-unit.log`、`g1-ui-final-lint.log`、`g1-final-output-browser.log`、`g1-assistant-fidelity-browser.log`、`g1-final-union-browser.log`、`g1-reduced-motion-browser.log`。合成供应商不冒充真实模型。
- 原生`g1-native-composer-verified.log`通过（含新未配置门槛、marked Return、快捷键、35%和滚动、模型能力、引用、失败/新草稿保留）；`g1-native-window-regression.log`通过1042/861/640/1100保留宿主/编辑器/选区/滚动、真实5/6、长CAS编辑停止。**Swift功能fixture与Release不要并行**，共享cmark对象会被修改。`test-workbench.sh`已修Bash3空数组，功能编译用Onone，正式Release/性能脚本没改。旧EOF/共享产物/编译失败日志保留，不计通过。
- 同一个已编译功能fixture可直接执行，避免反复编译全Swift：`target/macos-workbench-tests/workbench-fixtures FRESH_ROOT [--composer|--cell-states|…]`；真实模型限制不变，别运行--live。
- 最新窗口截图`target/macos-workbench-tests/g1-final-nbG9LM/{mac-light,mac-dark,mac-1000,mac-800}.png`是实际宿主内容1440/1000/800×808；完整AppHost CUA为1440×860。**不是900**，本机被窗口系统限制；未改分辨率。最终4张并排比较在`docs/acceptance/pre-alpha.5/g1-{mac-light,mac-dark,web-desktop,web-phone}.png`，图已视觉核对，高度例外获用户同意；其中Mac浅色源图有CUA 2x空白填充，比较图仅裁掉空白，原图保留。最终可重新抓稳定图再生成，明确实际高度与不同输入场景。

## 完整Mac隔离副本 / CUA

- 自有完整AppHost：`target/plan-5/g1-ui/OpenMath Workbench G1.app`，bundleID `org.openmath.OpenMath.WorkbenchG1`，Info.OpenMathFixtureRoot=`g1-ui/storage`，Window键1440/900。它读普通完整AppHost/Settings/菜单，不是删减fixture。重建后先确认退出再ditto/改Info/adhoc签名。当前为最新成功Release（有条件/外观/静态忙碌指示），Window.reduceMotion=true，不要更新正在运行的副本。
- 此前打开`g1-ui/二次方程与图形.omnb`验证2/3与条件；重启静态模式后当前是一个自有未命名格，factor(x^10007-1)已显式停止，无活动计算。模型目录只有public假元数据，10104无服务、钥匙串引用不存在，不会付费调用。此前9张TXT附件由原生OpenPanel导入，用户真实拖入后回读为10张；重启后输入草稿不持久，当前新文稿无附件。CUA `scroll()`未改变位置，但容器暴露的`performSecondaryAction(index,'Scroll Right')`实际从01/08移到07、再06/05，横向滚动已验证；图在fidelity。
- CUA压缩后先`rewriteDocumentation`。现有绑定：g1Mac、g1Fs、g1State、g1Finder、e1Tab、e1Browser。Finder当前attachments目录，留给用户拖放，不碰其余文件。Mac新鲜完整AX会重编号；菜单ID易失，可同一次call读AX后按已观察文本提取index再click。OpenPanel选row再shift末row能多选；CmdA并未多选。CmdO面板慢可能CUA超时，先恢复已有面板，不重复开。
- IAB browser2，tab1自有5173，providerTabId `browser-use:e4322a1d-00fc-4042-94fc-7d0378f61461`；tab2 about:blank。当前tab1是1440×900、13格设计本，仍已加载旧WASM；真实条件最终headless测试已通过，别据旧页面判新dist失败。Web本地回环供应商已通过UI把10103改为**无服务10104**以供人工IME，原授权F5配置没改。临时viewport收尾reset并每turn重新markHandoff。旧用户4175不动。

## 当前 G2 / G3（2026-10-21，唯一最新进度）

- **最新覆盖**：f936f2e0的单Window onOpenURL亦未处理Finder文件，失败包保留。现按Apple现代NSApplicationDelegate.application(_:open:)接单file URL，移除视图onOpenURL，保留onAppear代理绑定；冷启动队列区分旧openFiles回执。已有WorkbenchFixtures新--file-events真实冷/热源码加载、非file/多URL拒绝、存储关闭全部过（g4-file-url-events-regression.log）；尚需提交、完整Release包及真实Finder验收。无活动原生包，Web旧f936自有4179，未公开，调用仍69/80。
- **覆盖下条**：33617ee5已打三包/验签/4226文件一致，新Web西瓜/2/3/.5+SHA通过；Finder冷启动仍欢迎页，onAppear代理绑定尚不足以覆盖SwiftUI文件分发，现加根视图onOpenURL→同一workspace.open。需提交重建并真Finder验证，未算通过；当前运行ZIP路径`/Users/hert/Applications/OpenMath pre-alpha.5 ZIP 33617ee5/OpenMath Preview.app`欢迎页。所有旧失败/候选保留，不公开。
- **最新G4发现**：7bc1e81d三包已构建、同SHA CI全绿（39488786752），Web与DMG西瓜/2/3/.5+SHA均通过。ZIP4226文件与DMG逐字节相同且验签过，但OpenPanel复现Open禁用；安装到Applications亦如此。Finder“打开方式→其他”指定ZIP app后仍欢迎页，实际代码AppHost声明delegate却无attach调用，外部打开事件一直pending。现补根视图onAppear绑定，需提交/重建/实际Finder验证；旧包保留不发布。当前ZIP运行路径`/Users/hert/Applications/OpenMath pre-alpha.5 ZIP 7bc1e81d/OpenMath Preview.app`，欢迎页；已关闭自身面板。不得修改系统权限或文件默认关联。
- 同范围goal active。**G2已提交推dev=40dae357**；G3已提交推dev=f98be447。G4正在统一.5并准备包，尚未公开或请求公开确认。不开子代理，不改输入法/安全/权限，不启动iOS模拟器，不看其他App找参考；不stage或删无关`.gitignore`、`.vscode/`、`test-results/`。
- **用户上限80，账本69/80，全部finish，无活动模型任务。** `target/plan-5/live-task-runs.jsonl`为唯一账本。Mac gpt-6.1-sol、gpt-6-luna、DeepSeek S1–S8；Web sol S1–S6；Mac图片/PDF真实发送全部通过，不重跑。
- DeepSeek：S1实际5/6；S2 retry02实际scene2592三角形、2数学格无诊断/完整回复（22请求222秒）；S3双曲线400点/−π..π（9请求16秒）；S4真实缺括号修复为f=x²+1/10（12请求21秒）；S5 retry02先查目录、Wolfram NIntegrate/gauss_kronrod/0.3102683017233811（13请求36秒）；S6讨论scratch6、原源码/接纳2/3标识不变；S7实际长算childIndex1 started后CANCELLED、源码保留无新有效结果（15秒6请求，后读仍6）；S8从S2-01整轮撤销回0格、无重复撤销按钮。旧失败全部保留。
- Image-01真实请求有一个JPEG data URL，原128×96图片尺寸/颜色准备后对应，模型正常complete/stop返回绿色/棕色/棋盘纹，1请求。**象限位置描述不完全准确**，如实记录为模型视觉能力局限，不能声称空间识别准确。PDF-01真实PDFKit提取第1页文字，provider body含公式和页码，回复正确显示x²+y²=1、2+2=4及页1，2请求。讨论均无文稿修改。用户已回复“已处理”并手动处理当前签名钥匙串窗口；原任务延续、无重发、不再问旧提示。
- 本轮生产修复：Settings先初始化同根再保存模型；切文稿按session/assistant身份绑定；Provider两层按字节合并而非32碎片限流，保留8MiB/UTF8 offset/canonical/取消；干净源码缩短/emoji替换合法收回选区、原子回写；诊断以Data(UTF8)索引防NFC/NFD ID冲突；两端transport按4096字节喂decoder防OS合并读取误触batch。分别现有Settings/ProviderTransport/SourceEditor/test-drafts(3真实SIGKILL)/Node protocol11+bridge10/native protocol通过。日志见计划。
- S2-01与S5-01失败的确切终止点均为已32工具提案后新增提案撞原宿主上限。**未扩大32上限**；FFI将IPC_TASK_BUDGET归类BUDGET_EXCEEDED，Session显示步骤上限，停止优先。真实32→33拒绝/仅1耐久消息测试通过g2-proposal-budget-native-2.log；首轮fixture旧代次失败保留。语言卡补Modern方括号是索引、Wolfram兼容调用需正确dialect；生成输出和test-prompt通过。
- 最新完整候选`target/plan-5/g2-ui/OpenMath Candidate G2.app`，bundle org.openmath.OpenMath.CandidateG2/root g2-ui/storage，revision ef76f794+G2-budget-dialect-fix，含全部修复；严格AgentRuntime+Release `g2-budget-dialect-native-build.log` exit0、签名已验。当前PDF讨论界面，无活动任务。替换前CUA正常退出，按Info.CFBundleExecutable核对确切PID为空，再ditto/保留fixture/bundle键/验签。
- **G3实测通过**：fresh-root`target/macos-performance/g3-pipe-optimized-44tax98p`普通Finder打开，53全foreground=true、整次max270.150292ms；100次10KiB输入p95=33.315834ms/max68.01925ms，首workspace643.5145ms/RSS157581312。独立53正确性g3-native-pipe-correctness.log全过。旧65.417/66.473ms失败与无效前台尝试全保留，不挑最快、不放宽50ms。App已退出，勿getApp/截图重启覆写旧root。最后仅提示词/预算状态不影响计时路径。
- G3其余既有通过：host-service28 suites/181；storage/saves/recovery/undo/kernel-runtime五脚本；CLI53正确/max46.6965ms；Web生产55+开发核2补跑、输入p95=32.8/max34.2、53max87ms。相机原7936三角形60Hz/静止0新帧已在C8通过。G3提交测试入口app/e2e/performance.spec.ts、PerformanceFixtures及完成记录；G2提交含已验证输入路径优化。
- GPT仅`/Users/hert/.config/openmath/testing/pre-alpha-4-api.json`，DeepSeek仅`/Users/hert/.config/openmath/testing/pre-alpha-5-deepseek-api.json`；0600、不输出密钥/认证头、不动用户10100服务。DeepSeek官方/默认thinking未禁用，reasoning_content replay已支持。
- CUA压缩后rewriteDocumentation、精确路径绑定；fresh AX提数字再操作，`/^\s*\d+/`避focused重复行。GoToWindow需先getAX确认，再setValue PathTextField再Return，避免异步面板未就绪读到旧路径。当前g2Mac/g2State/g2ClickID/g2Fs，g2Db为只读sqlite连接。新对话会重置默认DeepSeek/执行，发媒体前显式选sol/讨论。CmdN保存确认选“关闭，保留本地修改”。截图invalid-size先核对文件和账本，避免重复调用。
- G4旧候选`5139465e8dd34ba85345887697ac8ef61cdc8242`三包/manifest已生成且本地安装通过：DMG/appZIP4226文件逐字节相同、严格验签、无fixture键；原生成功OpenPanel打开自有西瓜示例/生成真图/求根2和3/保存，正常退出后appZIP重开四格源码再计算通过；Web真实解压目录4176亦实算西瓜及2/3，Mac/Web版本均.5+完整SHA。证据在该SHA目录local-acceptance。普通.omnb源码格式不缓存结果，重开不自动运行。两原生发行副本已正常退出，勿getApp隐式重启旧测量包。f226deab/5139465e旧包均保留不发布。
- **发布前CI发现并修复**：5139465e的Rust两处collapsible_if、Web外部提示词fs.allow、App测试缺JSDOM ResizeObserver。全工作区all-targets Clippy、Web155单测/lint/typecheck、受影响Rust52项（宿主28/结果12/回环传输12）与发行5项通过。证书依赖升级rustls-webpki0.103.16/root-certs1.0.10修复两漏洞，deny通过；Linux keyring上游未迁移zbus6，三个仅停维护告警逐项例外/notice，未豁免漏洞。需提交本次修复再重建三包/manifest并安装回读，随后才问具体包公开授权。
- G4准备：Rust/npm/原生Info/独立pre-alpha.5目录/发行入口均.5，保留.4不读不迁移；agent86依赖记录不变。发行说明docs/release-0.1.0-pre-alpha.5.md，包脚本要求HEAD干净并记录精确SHA。真实模型仍69/80不重跑；用户对旧钥匙串问题已回复“已处理”，不用再问。
- 不并行Swift功能fixture与Release/cmark，不在编译期间改输入；当前无构建。G4需统一.5、DMG/appZIP/WebZIP+manifest/发行说明；先本地三包验收，再请求用户明确确认公开pre-release，公开后匿名下载校SHA并安装回读；此前goal不能complete。

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

G1补记：首次用map(fn(k)=>sin(k),range(1,1000000))在截图前已返回尚未运行，没有本轮验证其终止原因；改用factor长算显式停止完成静态模式检查。G3若涉及超时/失败状态需核对，不将首次map算成功。
