# OpenMath 当前接续入口

更新：2026-10-16，scope 2；只推进原生 Mac 与生产 Web 的下一个 pre-alpha。历史事实见 `docs/plan/PRE_ALPHA_4_PROGRESS.md`，旧接续文本可由 Git 查看，不在此重复叠加。

## 先读与禁止重复

1. `docs/plan/PRE_ALPHA_4.md` 第0节：8项 delivery_check / 三资产交付；旧98任务、32gate只作详细规格和后续参考。
2. `docs/plan/UI_REFERENCE_LOG.md`：已看应用、页面、采用理由和实现状态。**已看页面不重新遍历；只有具体未解问题才补看。** D066–D069旧记录已汇总，本轮补齐快捷指令、地图、磁盘工具。
3. 进度末尾 D069–D071 与工作区，继续下列实际交付工作。纯文档不用重新编译签名，不新增形式门禁。

## 不变约束

- 用户明确要求单代理；不用子代理、不在本机启动 iOS 模拟器、不合 main、不移动旧 tag / 旧发行资产。dev 推送及本版实际 GitHub pre-release 已授权。
- 不改系统输入法、安全或权限设置，不再询问日文输入法；不扫描旧 OpenMath profile、env、密钥或用户数据。
- 仅允许精确读取 `/Users/hert/.config/openmath/testing/pre-alpha-4-api.json` 的授权 API；权限目录0700/文件0600，模型 gpt-6.1-sol / gpt-6-luna。不得输出密钥、HTTP认证头或提交凭据；泄漏扫描只输出精确匹配计数。
- 冻结的13份 `docs/design/` 来源及 baseline SHA 不改；新观察写 UI_REFERENCE_LOG / 进度。
- 外部 `.gitignore` 的 `ui-concept-demo/` 规则保留且不 stage、不检查该无关目录。`test-results/` 新出现，未确认归属前不清理或提交。
- 只清理本仓库已停止、可重建的自有缓存；保留源码、独立数学验证器、原失败、发行包、依赖与正在使用的构建。约36GiB物理可用，暂不需要大清理。
- Rust使用 CARGO_PROFILE_DEV_DEBUG=0 / CARGO_PROFILE_TEST_DEBUG=0。build.rs散列全部.rs，**编译期间不改Rust或测试输入**；只轮询确切存活句柄，不因观察超时重启。

## 当前源码与安装包

- dev / origin 最后生产提交 `689169f981bc8143501746a0225f32a8d869e08d`（原生菜单撤销路由）；前提交4865842e统一 `.4` 版本与三资产打包，c68d7280完成atan/提交竞争修复及界面精简。
- 源码运行 `.4`，公开仍 `.3`。Root1 / Library4 / Doc5 / .omnb1 不变。candidate UNSET，8checks not_run，ready / released=false；不能直接将开发记录改成发布完成。
- 当前包目录见 `target/plan-scope2/current-release-assets.txt`，实际为 `target/pre-alpha.4/689169f981bc8143501746a0225f32a8d869e08d/release-assets`。DMG / appZIP / WebZIP均已构建，689 manifest尚待生成。
- 新安装：`/Users/hert/Applications/OpenMath pre-alpha.4 verified/OpenMath Preview.app`；签名严格校验通过，自带Node26.11.1 / Pi。实际About显示 `.4`、完整689 SHA与OM图标；中文撤销 / 重做、精确2/3、保存、退出重开已实看。自建文件 `target/plan-scope2/安装验收-689169f9.omnb`。
- 首个486包发现真实默认菜单Undo/Redo灰置，689已经修复并实测本地/提交后逆事务，旧包不发。既有spctl显示security disabled override，不是标准Gatekeeper或公证证明；未改安全设置，指南如实说明ad-hoc / 未公证。

## 已完成事实，不要重跑无关全套

- Markdown AST / SwiftMath / 独立NSTableView表格、步骤子树、CoreGraphics2D、Metal真实网格及按需帧均已接生产；见D067–D069。
- c68原53由优化前独立二进制验证器核SHA后检查全部高精度期望：CLI最长125.995ms，Mac submit→MainActor整次最长832.382ms，Web WASM231ms / 往返232.5ms；10KiB各100次输入p95约33ms（Mac max82.151ms如实保留）。普通7936三角形约60Hz、静止0新增帧。
- 独立旧验证器身份在 `target/plan-scope2/pre-optimization-oracle.json`，保留并先核SHA。当前 `.4` 原53与性能还需对当前生产对象执行；不得修改原期望 / 门槛或挑最好一次。
- 689 Web ZIP已解压到 `.../689.../web-installed`，逐文件字节比较通过；从该目录运行的35项已有生产E2E **全部通过**，日志 / wasm-corpus.json / web-input.json 在 `.../689.../web-acceptance`。当前Web输出仍待独立旧验证器读回与简要指标记录。
- Web本地服务已在4174运行，确切exec句柄78460；从实际unzip目录提供文件，不重复开服务。当前原生应用没有遗留运行：factorial(1000000)已真实超时结束，不计为手动Stop通过。
- 早期真实助手成功记录 `target/macos-live-assistant/attempt-5RlZQf`（实际read→preview→apply→read→run→inspect，原blob x→2/3）；当前689包内助手验收继续，不据旧记录直接宣称新包全部通过。

## 当前助手测量入口与失败

- `target/pre-alpha4-acceptance/{performance,live,pi}`链接689生产Release -O对象与当前.4 Rust库；临时entry用@testable内部接口，跨模块actor let访问已补await，本身未改产品输入。字体资源齐备。
- Pi第一次module attempt在StorageService新root内预先放run.log，正确触发recoveryRequired；第二attempt-empty-root确切句柄2901已结束，exit -5 / StorageError.inUse。先检查入口 / root释放位置 / 同root进程，不盲目重跑或放宽存储锁。
- 日志与fresh工作root必须分开；live以授权API文件、实际installed app、空workroot和gpt-6.1-sol为参数，3分钟watchdog。仅使用原授权文件，输出不能含key。
- 安装UI停止尚未通过：map百万列表Native返回INTERNAL_ERROR（CLI能完成）记录保留；factorial百万自动timeout，CUA稳定等待错过按钮不能算手动停止。先用已有长计算路径实际按停止，再验证正常小计算 / 源码保存。

## 下一步（按此推进）

1. 把本轮参考固化并停止泛看应用；修正接续入口，避免压缩后重复。
2. 定位Pi inUse的测试入口问题，确认真实包内Pi确定性链与当前.4 live助手；补实际UI停止 / 正常恢复。
3. 对689当前CLI / Mac生产对象测原53与输入，复用已通过Web原35项输出做独立数学读回；记录诚实性能与环境。相机无新改动，无理由不重测。
4. 查看689 CI紧凑结论。结合原保存/恢复/权限回归，记录8项实际结果；仅修新发现的具体问题。必要产品变更会生成新源码候选与相应包。
5. 冻结实际候选，三资产manifest / 中文指南；确认新tag不存在，发布本版GitHub pre-release，再下载三资产核hash，实际安装 / Web启动与程序版本读回。未做公开下载前released_verified必须false。
6. 最后按真实状态更新账本并完成goal；不能因文档或测试数量结束。

## CUA 使用经验

UI只用cua_repl；压缩后先rewriteDocumentation。getAXState可能自动重启已退出app，验证退出用精确ps路径。截图后若按AX index操作需重新完整取树；disableDiffing会重新编号。中文typeText / 初次focus可能不生效，使用paste / setValue并读回，不把未输入误判成IME bug。不要用Shell / AppleScript控制UI。
