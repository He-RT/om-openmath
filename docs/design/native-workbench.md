# 原生工作台接线

R4.2.01 的窗口、分栏、导航和权威状态投影已完成。完整助手、步骤树、变量值与图形展示仍由后续专门任务交付；导航完成不代表这些能力完成。

`WorkspaceView` 接入已有单文档 `NativeFileWorkspace`，不创建第二个文档、源状态或计算执行器。标题、目录、单元格与编辑器来自原已确认源码；运行和停止由文档持有的 `NotebookRuntime` 连接原 `KernelCommitPort`。结果只能从已耐久接纳的原回执和只读 `ResultClient` 获取，完整输入式在后台核对分块/散列后显示。普通打开不计算。源编辑先由原 DocCommitPort 确认并注册真实撤销，再运行。

分栏使用同一 `NSSplitView` 和三个稳定 `NSHostingView`，1042/861/640 按实际内容宽度提供三栏/两栏/单栏。折叠隐藏视图不会销毁编辑器、执行 owner 或当前结果。窄窗口用原生 popover 访问目录和检查器；没有计数不生成百分比。目录明确导航时才滚动，编辑器的真实 first responder 只更新选中单元格，避免点击编辑器就重置滚动。分栏拖动由 AppKit 处理并保留宽度。

`WorkbenchNavigation` 由文档的运行 owner 持有，提供文档、结果、助手、步骤、变量五个稳定目的地；重建 View、折叠或切换不会创建第二个运行 owner。基础步骤目的地读取实际不可变记录的首个根页，没有记录时明确显示未记录，不生成解释。变量目的地只显示原 kernel 的已接纳版本/定义是否仍对应当前源码，实际草稿变化立即标需核对；完整值列表仍待 R4.2.11。助手未接通时显示未启用，不因开发测试文件存在而假装应用已配置模型。

运行显示真实 queued/running/waiting_source/candidate/committing/unknown 状态；停止按原 operation ID 发送一次，确认前保留 pending。已提交结果不被晚停止覆盖。计算提交与结果读取失败区分，不能因读取出错声称未执行。未知提交只能核对原 ID，不能自动重算。关闭文档后才关闭运行 owner，并等待实际准备/写入/只读请求结束；面板隐藏不触发关闭。

结果保留原始精确 InputForm，现已接通真实只读 LaTeX 的[原生公式](native-math.md)，原 LaTeX/Modern/Wolfram 均可复制。读取失败显示真实状态与源码出口；结构化值、真实步骤树、变量、图形/Metal、助手和更多命令继续由 R4.2.09–18/R4.3–4.4 实现。没有为未实现的端口制造返回结果。

本轮使用 `telegram-ui-reference` 的 Markdown 基础算法，未读取源码归档或应用可选特效：

- [目的地状态](</Users/hert/Documents/ChatGPT/ui-learning/02-layout-and-navigation/tabs-and-navigation-stacks.md>)：保持选择和 owner，重复选择不重置状态。
- [实际宽度布局](</Users/hert/Documents/ChatGPT/ui-learning/09-adaptation-and-accessibility/screen-size-and-orientation.md>)：使用实际空间及任务规定断点，保存草稿/选择。
- [真实操作反馈](</Users/hert/Documents/ChatGPT/ui-learning/06-state-and-feedback/progress-and-result-feedback.md>)：实际 producer 与取消回执独立，不以动画或未知取消声明成功。

实际验证由 `test-workbench.sh` 创建私有测试根、真实 AppKit 窗口/TextKit 编辑器和生产组件：1042/861/640/1100 下中心宿主对象、原字符串/UTF-16 选区保持；SQLite/C ABI 原计算得到5及6；真正运行长CAS时仍能缩窗、编辑、显式停止，后来的中文/emoji草稿保持。另一个持续的隔离 UI fixture 经实际 AX/截图检查，运行得到2及3，拖分栏208→238、目录隐藏及结果检查器切换保持原结果。cacheDisplay 的离屏图不能代替实际屏幕检查。

后续补验真实笔记本滚动100点，五个目的地反复切换/隐藏后同一 NSScrollView 和原滚动位置保持。2026-10-13 的实窗检查得到2/3，步骤显示原记录未录制、定义状态随真实源码编辑失效、助手显示未启用。Mac runner 缺少 `rg` 导致上一 SHA 的工作台测试在执行前退出，脚本已改用 Python 的确定性源码枚举并核对集合完全相同；该失败原件保留，新 SHA 独立查 CI。

2026-10-13 的 f875658 CI 实际触发目的地切换重置滚动位置。修复后中心宿主只在文档会话实例变更时重新赋 rootView，现有 workspace/runtime 观察订阅负责更新；同文档切换检查器保持原 SwiftUI scroll state，而同 ID 的新会话仍绑定新的 owner。本地原断言、实际100点滚动、四个宽度和真实运行/停止回归通过，旧失败保留，新提交 CI 独立核对。
