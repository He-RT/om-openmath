# 项目接续提示词

请接续当前 OpenMath 项目，不要从零重建或重新制定计划。

先阅读 `docs/plan/PLAN.md`、`docs/plan/PROGRESS.md`、`docs/plan/DEVIATIONS.md` 和 `docs/plan/QUESTIONS.md`，再检查现有代码、测试以及 `git status`、`git diff`、`git log`，据此确认实际进度。若存在 `.superpowers/sdd/PLAN/`，也参考其中的执行账本和当前任务简报；这些本地记录不是必需依赖。

保留并接续尚未提交的工作，从当前未完成任务开始，按计划逐任务实施、测试和验证，不重复已完成工作，不把占位代码当成完成。由你直接执行，不使用子代理或多代理 workflow。

在 `dev` 分支工作，及时更新进度与裁决，按任务或较大批次积极提交。用户已授权推送到 `https://github.com/He-RT/om-openmath.git` 的 `dev` 分支；仅推送已验证的成果，不自动合并到 `main`。无需逐任务询问是否继续。

下一版 `.3` 的新增范围见 `docs/plan/NEXT_RELEASE.md`，现代语法见 `docs/design/modern-language.md`，全景目录见 `docs/reference/README.md`。先按进度核对R3.0/R3.1状态，不把规划示例作为当前可运行能力。本机不启动模拟器；移动端模拟器测试交GitHub CI。Notebook Agent/原子事务/幂等/撤销/框架适配为后续预留，未实现，不安装Pi/Rig；Agent会话和凭据不得写入.omnb。
