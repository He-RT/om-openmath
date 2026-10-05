# 项目接续提示词

请接续当前 OpenMath 项目，不要从零重建或重新制定计划。

先阅读 `docs/plan/PLAN.md`、`docs/plan/PROGRESS.md`、`docs/plan/DEVIATIONS.md` 和 `docs/plan/QUESTIONS.md`，再检查现有代码、测试以及 `git status`、`git diff`、`git log`，据此确认实际进度。若存在 `.superpowers/sdd/PLAN/`，也参考其中的执行账本和当前任务简报；这些本地记录不是必需依赖。

保留并接续尚未提交的工作，从当前未完成任务开始，按计划逐任务实施、测试和验证，不重复已完成工作，不把占位代码当成完成。由你直接执行，不使用子代理或多代理 workflow。

在 `dev` 分支工作，及时更新进度与裁决，按任务或较大批次积极提交。用户已授权推送到 `https://github.com/He-RT/om-openmath.git` 的 `dev` 分支；仅推送已验证的成果，不自动合并到 `main`。无需逐任务询问是否继续。

下一版 `.3` 的新增范围见 `docs/plan/NEXT_RELEASE.md`，现代语法见 `docs/design/modern-language.md`，全景目录见 `docs/reference/README.md`。先按进度核对R3.0/R3.1/R3.2状态，不把规划示例作为当前可运行能力。本机不启动模拟器；移动端模拟器测试交GitHub CI。Notebook Agent/原子事务/幂等/撤销/框架适配为后续预留，未实现，不安装Pi/Rig；Agent会话和凭据不得写入.omnb。

当前进度补充（2026-10-05）：R3.1与R3.2已完成；R3.3a新增46实际回调已验证，但R3.3整体与R3.4–R3.6仍未完成，不能宣布全部功能或.3发行完成。下一任务继续R3.3：机器矩阵与LU/QR/Cholesky/SVD/实对称特征/最小二乘、特殊函数/概率随机、纯数据CSV/JSON、SI单位以及angle/projection与help/functions/options/capabilities求值入口。当前矩阵消元仅精确有理数，分布quantile未接入，目录均如实标partial。source-only.omnb与预留Agent边界保持。所有验证命令及批次在PROGRESS/P119；R3.2的CI37239127048已全通过。本机不启动模拟器。

当前发行裁决（2026-10-05）：用户明确「仍按原完整.3计划」，不缩小范围；活动目标为「尝试完善并发行」，仅当全部原.3能力与发行门禁完成后可标完成。新增机器数值层/七个回调的批次见R3.3b/P120；接着完成机器rank/null_space和精确/机器欠定或矩形求解，再继续特殊函数/随机概率/解析/单位等。当前仍不得发行.3，版本保持.2；不使用子代理、不运行本机模拟器。全程更新实际状态，不把分批进度当作完整交付。

最近验证的R3.3b：om-analysis基础层及机器LU/QR/Cholesky/实对称Jacobi/SVD/最小二乘已接入，描述版本5、179回调/174身份；机器det/inverse和显式numeric方阵linsolve可用。933Rust/2ignored、前端59、Python15、Clippy/纯WASM/deny/TS漂移均通过。下一步保持完整.3范围：机器rank/null_space，精确欠定返回真实自由轴与矩形机器求解/最小范数，再继续原计划剩余内容。SVD固定种子48低秩/矩形性质验收已通过，旋转不可表示的微小相关残列可按机器阈值视数值零，独立微小列保留；不可表示极端尺度仍明确失败。不标目标完成、不提前发.3。

R3.3c当前：精确/机器矩形欠定全解、机器rank/null_space、LS最小范数与angle/projection已验证，唯一解仍列表，非唯一为参数化Record。NEXT_RELEASE「欠定保留自由轴」已恢复，前轮拒绝样例改真正不相容，未动53数学语料。当前181回调/176身份，描述6，应用.2；下一任务从特殊函数/随机概率/CSVJSON/SI单位/help等R3.3剩余开始，随后R3.4–R3.6。活动目标保持完整.3，不提前发行；本机不运行模拟器。

R3.3d最新：特殊函数/实主值补齐与正态/均匀分布、PDF/CDF/分布Quantile、SplitMix64会话随机已验证，197回调/192身份、描述7、运行时仍.2。最终946Rust/2ignored、Python15、前端59/lint/typecheck、全Clippy/fmt、纯WASM、TS无漂移与deny均通过；原53数学期望未改。小尾部、LogGamma接近1和2的微小值、高精度拒绝、随机失败/取消不推进和只读隔离均实际测试。下一任务继续R3.3的CSV/JSON纯数据、SI单位及help/functions/options/capabilities求值，再R3.4–R3.6，不提前发行、不缩减完整范围。接口/算法裁决见P122和special-probability.md；本机不运行模拟器，不使用子代理，Agent/事务仍预留。

R3.3e已验证：CSV/JSON四真回调、DataTable只追加数据头、空表头/Unicode/无执行/精确数值/预算往返完整，201回调/196身份，描述8，应用仍.2。951Rust/2ignored、Python15、前端59、Clippy/fmt/纯WASM/TS/deny已通过。上一批ceea0b5的CI37280109336仅iOS原始#18耗时1306ms超1s失败，Rust/前端/依赖成功；实际xcresult已下载target/ci-evidence/r33d，本机不要开模拟器/不要放宽门槛。接续先分析原18的Swift总时间与kernel timing，再继续基础表格操作、SI单位和帮助查询，然后原R3.4–R3.6与完整发行；保持goal active不提前发行。data-formats.md/P123记录真实类型与限制。

性能接续P124：R3.3e提交5adb742已本地验证。离线CI附件原#18 Swift1306ms/kernel152ms，#17 Swift968ms/kernel262ms。Swift KernelClient响应原双遍完整JSON已改单遍顶层桥接错误+packet解码；本地5025bytes原18包1.323→0.613ms，不足解释1s额外延迟。验收附件新transport字段提供queued_ms/ffi_ms/decode_ms/resume_ms，wire/.omnb不含；原整次<1s门槛未改。离线错误/往返、SDK27 arm64类型检查和generic无签名iOS build-for-testing通过；没有本机模拟器。原Rust bridge53及Clippy/Python通过。接下来查新CI分段结果，若主线程恢复/排队慢则按实测修复；不能宣布移动门禁全通过。之后继续基础表格操作、SI单位/帮助和R3.4–R3.6原完整范围。

最新R3.3f：基础DataTable行操作/列字段/记录排序/正确预算已验收，Slice补真正range参数并新增目录个数契约。955Rust/2ignored、前端59、Python16、Clippy/fmt/纯WASM/TS/deny通过，201回调/196身份/描述9/.2。P124的CI37296195523全通过：iPhone53最慢628ms、iPad417ms，原18为73/69ms，原1s门槛与数学期望未改；附件在target/ci-evidence/p124。接着实现SI单位与help/functions/options/capabilities，再R3.4–R3.6全原范围，不提前发行、不标goal complete。本机不运行模拟器，直接执行不使用子代理。

最新R3.3g：SI标量单位四回调、精确因子/七维量纲、受预算纯单位DSL及单位算术已验证；205回调/200身份、描述10/.2，959Rust/2ignored、前端59/Python16/Clippy/fmt/纯WASM/TS/deny通过，原53不动。下一步帮助查询，再R3.4–R3.6全原范围。R3.3f的CI37299136370 iPhone原17总1096ms，实际内部queued/FFI/decode/resume仅约212ms、kernel209ms；约884ms在现有request计时之外（入口前/编码/返回边界待区分），需核对Swift执行器边界。附件target/ci-evidence/r33f。保持1s原门槛，不能改用kernel时间冒充通过；本机不运行模拟器，单代理直接执行。

P127 Swift接续：acde078的失败分段总1096ms vs方法内212ms，不据此认定计算慢。request/feed/perform已显式nonisolated(nonsending)保持caller actor，减少generic executor往返；JSON编码移到串行工作队列并新增encoded_ms，编码后检查取消并清理running状态。原1s整次断言不改，旧wire/.omnb不加字段。Swift6.4离线caller actor跨queue连续性断言通过，iOS27 ARM64 typecheck和generic无签名build-for-testing成功，Python16通过；没有本地模拟器/真机执行，不宣称新的端到端门禁已通过。单位提交b668869；接下来核对新CI分段后继续help/functions/options/capabilities并完成R3.4–R3.6全原范围。

最新R3.3h：help/options/functions/capabilities四真查询完成，209回调/204身份、描述11/.2；964Rust/2ignored、前端59/Python16、Clippy/fmt/纯WASM/TS/deny通过。P127 CI37302584053全部通过，phone53最大339ms/pad315ms，原1s与数学期望未动。附件target/ci-evidence/p127。R3.3审计发现cbrt/nth_root仍规划（旧cbrt仅名字映射/编译数值路径，不等于当前求值回调）及diag未提取矩阵对角，先补真路径后进入R3.4–R3.6。normal为Series依赖留R3.4。保持完整.3/goal active，不提前发版，本机不启动模拟器，不使用子代理。

最新R3.3i完成并关闭R3.3：CubeRoot/NthRoot两真回调、矩阵diag提取、held原极点保留及采样转换通过。211回调/206身份、描述12/.2；968Rust/2ignored、前端59/Python16、Clippy/fmt/纯WASM/TS/deny通过，原53未改；完整范围审计没有R3.3基本/线代/统计/单位/runtime规划漏项，normal留R3.4 series。接下来直接R3.4真实微积分/ODE/优化/拟合，然后R3.5–R3.6全计划。不要仅因R3.3完成标goal complete或发.3；继续dev、单代理、不运行本地模拟器。P127的CI37302584053全通过（phone53最大339ms/pad315ms），新查询CI37305191067当时仍运行，后续查同SHA。

最新R3.4a：六笛卡尔微分入口真实求导/局部坐标/readonly与恒等式/独立差分已完成；217回调/212身份、描述13/.2；973Rust/2ignored、59前端/16Python、Clippy/fmt/纯WASM/TS/deny通过，原53不动。接着一维GK15/7积分（含无限区间/断点/误差与失败）、符号积分、极限/级数、ODE、优化/拟合，再R3.5–R3.6。

CI37307173022 iPhone原17总1325ms，kernel291ms/FFI291ms/decode2ms/resume1032ms，原18 resume772ms；真正主线程恢复阻塞已定位，保持整次1s门槛不能仅用kernel时间替代，后续查主线程启动/布局工作。证据target/ci-evidence/r33i。目标更新为尝试完善并发行+随时清理无用构建缓存；仅仓库缓存。刚清21GiB target/debug/incremental及两个iOS中间目录（构建已停止），free5.1→23GiB；dev/test incremental=false，保留release/安装包/当前deps/验收附件，不开启本机模拟器，单代理直接执行。

最新R3.4b数值积分已验证：om-analysisGK15/7纯callback、有限/反向/无限/断点、真实错误估计/部分失败与预算，Integrate只显式numeric/NIntegrate实际接通，raw poles/readonly/local axis保留；默认exact未实现保持partial，后续符号分支不许自动近似。219回调/213身份、描述14/.2；980Rust/2ignored、59前端/16Python、全Clippy/fmt/纯WASM/TS/deny通过，原53不动。上一批25f5c75 CI37341713046已全通过。接着真实符号积分、极限/级数，再ODE/插值/优化/拟合，R3.5–R3.6仍完整待完成；goal不要标complete/不要发.3。最新free24GiB，dev/test incremental=false，及时仅清本仓库未用缓存（保留release/产物/当前运行与源码）。本机不启动模拟器，单代理直接执行。

最新R3.4c精确规则积分已验证：默认exact，256次以内poly/常用仿射初等反三角反双曲实cbrt/链式/≤16次有限分部/Q一次二次因子及≤32重复递推/Gaussian→Erf。候选真正求导及精确残差，近似系数拒绝伪证，必要系数/实轴/主值条件保留；定积分证明局部条件/真实极点/原函数分支连续并消去哑变量条件，Q可去孔洞允许，未知参数可积/复路径拒绝；Gaussian整条/半无限支持。219回调/213身份、描述15/.2；986Rust/2ignored、前端59/Python16、Clippy/fmt/纯WASM/TS/deny最终完整通过，原53不动。下一任务limit/series/series_coefficient/normal，再ODE/插值/优化/拟合，随后原R3.5–R3.6和完整发行，不提前发.3/不标goal complete。

最新iOS线上失败CI37352256560：Rust/前端/依赖成功，phone原17/18整次1276/1328ms，kernel319/161ms、MainActor恢复954/1152ms，原1s断言保留，证据target/ci-evidence/r34b。仍需专门解决主线程恢复延迟，不能把数学通过当作平台全通过。磁盘free约22GiB，incremental关闭，及时清仅本仓库已停止使用的缓存，保留运行release/源码/发行与验收产物。不启动本地模拟器，不用子代理，继续dev。

最新R3.4d：Limit/Series/SeriesCoefficient/Normal四真回调，SeriesData只追加数据头且HoldAll。Q有理阶/解析真实导数/有限单侧双侧无穷/实函数夹逼/正底幂和倒数坐标、普通Taylor0..64/真实系数/首未知阶/normal、结构/形式导数/截断/readonly/取消已验收；223回调/217身份、描述16/.2。995Rust/2ignored、59前端/16Python、Clippy/fmt/纯WASM/TS/deny通过，原53不动。微积分静态依赖也正确屏蔽局部坐标并保留参数/边界，不声明被拒绝的readonly写入为定义。下一任务ODE Dormand–Prince/插值/简单终止事件、优化BFGS/Brent/凸二次保证与拟合QR/LM，然后原R3.5–R3.6全部展示/三维/导出/发行；goal不标complete，不提前发.3。

CI37363594946 c970dae：Rust+iOS全部成功，前端/依赖cancelled且日志404无法确定原因，整体failure不当作全通过。实际phone53最大572ms/pad523ms，原1s门槛/数学期望不改；附件target/ci-evidence/r34c。新同SHA继续检验完整CI。当前free20GiB、incremental=false；及时仅清仓库已不用缓存，保留运行release/源码/证据。本机不启动模拟器，单代理直接执行dev。
