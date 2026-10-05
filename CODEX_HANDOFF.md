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
