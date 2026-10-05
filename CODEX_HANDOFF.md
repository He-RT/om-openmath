# 项目接续提示词

请接续当前 OpenMath 项目，不要从零重建或重新制定计划。

先阅读 `docs/plan/PLAN.md`、`docs/plan/PROGRESS.md`、`docs/plan/DEVIATIONS.md` 和 `docs/plan/QUESTIONS.md`，再检查现有代码、测试以及 `git status`、`git diff`、`git log`，据此确认实际进度。若存在 `.superpowers/sdd/PLAN/`，也参考其中的执行账本和当前任务简报；这些本地记录不是必需依赖。

保留并接续尚未提交的工作，从当前未完成任务开始，按计划逐任务实施、测试和验证，不重复已完成工作，不把占位代码当成完成。由你直接执行，不使用子代理或多代理 workflow。

在 `dev` 分支工作，及时更新进度与裁决，按任务或较大批次积极提交。用户已授权推送到 `https://github.com/He-RT/om-openmath.git` 的 `dev` 分支；仅推送已验证的成果，不自动合并到 `main`。无需逐任务询问是否继续。

下一版 `.3` 的新增范围见 `docs/plan/NEXT_RELEASE.md`，现代语法见 `docs/design/modern-language.md`，全景目录见 `docs/reference/README.md`。先按进度核对R3.0/R3.1/R3.2状态，不把规划示例作为当前可运行能力。本机不启动模拟器；移动端模拟器测试交GitHub CI。Notebook Agent/原子事务/幂等/撤销/框架适配为后续预留，未实现，不安装Pi/Rig；Agent会话和凭据不得写入.omnb。

当前进度补充（2026-10-05）：R3.1与R3.2已完成；R3.3a新增46实际回调已验证，但R3.3整体与R3.4–R3.6仍未完成，不能宣布全部功能或.3发行完成。下一任务继续R3.3：机器矩阵与LU/QR/Cholesky/SVD/实对称特征/最小二乘、特殊函数/概率随机、纯数据CSV/JSON、SI单位以及angle/projection与help/functions/options/capabilities求值入口。当前矩阵消元仅精确有理数，分布quantile未接入，目录均如实标partial。source-only.omnb与预留Agent边界保持。所有验证命令及批次在PROGRESS/P119；R3.2的CI37239127048已全通过。本机不启动模拟器。

当前发行裁决（2026-10-05）：用户明确「仍按原完整.3计划」，不缩小范围；活动目标为「尝试完善并发行」，仅当全部原.3能力与发行门禁完成后可标完成。新增机器数值层/七个回调的批次见R3.3b/P120；接着完成机器rank/null_space和精确/机器欠定或矩形求解，再继续特殊函数/随机概率/解析/单位等。当前仍不得发行.3，版本保持.2；不使用子代理、不运行本机模拟器。全程更新实际状态，不把分批进度当作完整交付。

最近验证的R3.3b：om-analysis基础层及机器LU/QR/Cholesky/实对称Jacobi/SVD/最小二乘已接入，描述版本5、179回调/174身份；机器det/inverse和显式numeric方阵linsolve可用。933Rust/2ignored、前端59、Python15、Clippy/纯WASM/deny/TS漂移均通过。下一步保持完整.3范围：机器rank/null_space，精确欠定返回真实自由轴与矩形机器求解/最小范数，再继续原计划剩余内容。SVD固定种子48低秩/矩形性质验收已通过，旋转不可表示的微小相关残列可按机器阈值视数值零，独立微小列保留；不可表示极端尺度仍明确失败。不标目标完成、不提前发.3。
