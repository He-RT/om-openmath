# Mac 原生界面重写与 Agent 整合方案

[下一版计划](../plan/NEXT_RELEASE.md) · [Agent 架构](notebook-agent.md) · [工具契约](agent-tools.md) · [输入框](agent-composer.md)

2026-10-08，用户提出彻底重写 Mac UI，参考 telegram-ui-reference，采用完全原生界面，并询问是否适合与 Agent 同版。建议将原生 Mac 客户端与 Agent 作为同一目标版本的主线，按下列阶段实施验收；这是版本范围建议，当前尚未开始重写或锁定新发行版本。

原生编辑器与 Agent 共用文档状态、事务、执行和撤销入口，统一设计可以保持手工编辑与 Agent 修改的一致性。此次属于 Mac 客户端换代，范围包括笔记本编辑和全部结果展示，不只是右侧助手样式。

用户随后要求同时重做 UX/动效，并明确允许直接研究 Telegram GitHub。完整窗口、笔记本/助手/文件/设置流程与减少动态效果见 [UX 与动效规格](macos-ux.md)；[40 组件族清单](macos-ui-inventory.json)的设计占比为 Apple 标准 75%、原生自定义 22.5%、第三方原生数学排版 2.5%，平台原生 UI 目标 100%。当前仅为设计统计，尚无原生实现测量。

## 原生技术方向

| 部分 | 建议实现 | 必须保持的行为 |
|---|---|---|
| 窗口、分栏、工具栏和设置 | SwiftUI 与 AppKit 的原生窗口/控件 | 实际窗口尺寸决定布局，状态/草稿/选区/滚动不随重排丢失 |
| 数学编辑器与助手输入 | NSTextView 与 TextKit 2，接入 SwiftUI | 原生选区、IME、UndoManager、复制粘贴、键盘、补全及诊断 |
| 公式和 Markdown | 原生数学视图及 Markdown AST 映射 | Root、矩阵、区间、条件和长式正常显示，未知排版保留原式 |
| 二维图与结构化结果 | Canvas/Path 或 AppKit/Core Graphics | 显示内核采样与实际数据，缩放/平移、条件和结果引用保持 |
| 三维图 | 优先验证 Metal/MetalKit 原生渲染方案 | 使用 Rust 网格/颜色/法线，相机与展示不重新计算函数 |
| 文件、菜单、粘贴与媒体 | 原生文档/系统面板、NSPasteboard 和已接通媒体服务 | `.omnb` v1、保存/导出回执、图片/文件粘贴与处理状态保持 |
| 计算和业务服务 | 共享 Rust 内核及经过审计的原生桥接 | 数学语义、预算、中断、版本和事务不因 UI 重写改变 |
| Agent 运行 | Pi Core 本地进程，经原生 Mac 宿主连接业务接口 | 保留完整工具声明、身份绑定、上下文快照及实际回执 |

窗口、编辑器、公式、Markdown、图形和助手均采用原生视图；不以 WebView 承载主要编辑或结果内容。Rust 和 Pi 继续作为后端。原 HTML 草案只作布局/交互参考，不作为新的 Mac 运行界面。

NSTextLayoutManager 属于 macOS 原生 TextKit 网络；MetalKit 提供 MTKView 等图形组件。[Apple TextKit](https://developer.apple.com/documentation/appkit/nstextlayoutmanager)、[Apple MetalKit](https://developer.apple.com/documentation/metalkit)

SwiftMath 固定版本 1.7.3 的包声明支持 macOS 12，具有可复用基础；实际 Mac 数学视图、字体资源、许可和语料需重新验收。[SwiftMath 包定义](https://github.com/mgriebling/SwiftMath/blob/1.7.3/Package.swift) 最低应用 macOS 版本单独确定，不由单个依赖的最低版本自动决定，也不沿用旧包系统版本声明。

## 可以复用与需要重做的部分

可复用 Rust CAS、协议 DTO、原数学语料、结果/导出数据、函数能力目录，以及 Swift 客户端的请求/取消机制和部分不依赖平台的排版/Markdown逻辑。iOS SwiftUI 输出视图与控制器作为参考，逐项检查平台依赖后拆出共享部分。

需要实现 AppKit 编辑器与文档层、原生 Mac 窗口/菜单/快捷键、完整结果展示和三维 renderer。当前 MathEditor/DocumentStore 等依赖 UIKit；不能只给 iOS 工程增加一个 Mac target 就宣称完成。

当前 om-ios-ffi 创建会话时设置 HostPlatform::Ios，三维展示能力因此与 Mac 不同。Mac 桥接必须明确提供 Desktop 平台能力；在独立 C ABI 边界复用或通用化请求、缓冲释放和并发中断，继续维护 iOS 原行为。只编译通过不等于两端生命周期和渲染能力都正确。

## 原生宿主与 Agent 边界

Swift/AppKit 宿主负责窗口、编辑状态、文件/粘贴入口、任务展示和 Pi 进程生命周期。共享业务服务负责文档版本、冻结预览、提交、幂等回执、计算及撤销；这些契约不放在 View 的按钮回调中，也不改成由模型控制鼠标坐标。

此前 Tauri host 是当前桌面实现的可复用机制参考，新 UI 目标使用原生 Mac 宿主。是否直接复用 Rust 服务或抽出共享 host crate 在桥接阶段验证；完整工具/上下文协议保持，不用 UI 平台变化另造一套数学或 Agent 语义。

手工输入与 Agent 修改进入同一文档事务入口。原生 UndoManager 的编辑分组与 Agent 事务撤销明确协调；停止直接触达正在执行的计算与模型请求，不能依赖右栏仍打开或消息队列空闲。

## 分阶段交付

| 阶段 | 交付 | 验收门槛 |
|---|---|---|
| N0 | Mac 原生工程、桥接、文档状态和真实计算 | Release 应用可启动，源笔记本可打开/保存，取消与会话销毁真实正确 |
| N1 | 编辑器、公式/Markdown、解与步骤、结构化结果、二维/三维 | 当前 `.3` Mac 主要功能逐项对齐，原数学期望不变，真实结果可操作 |
| N2 | Pi 运行进程、冻结预览/事务、执行/试算/检查和撤销 | 完整编辑—计算—修正闭环，手工编辑竞争、IME、停止与重复调用正确 |
| N3 | 原生助手输入、模型/媒体、上下文和提示词管理 | 既定媒体/模型能力准确，草稿/引用/实际发送快照及持久化回执正确 |
| N4 | Mac 发行和跨端回归 | 无开发环境也能运行；旧文档、配置/凭据迁移、安装与附件门禁通过 |

每阶段产生可运行开发构建，不能只做一层原生窗口就宣布替换 Mac 客户端。Agent 和 UI 可以在服务接口稳定后交错开发；功能对齐与事务门禁仍按依赖关系验收，避免一次大提交同时改变所有路径。

同版发行要求原生 `.3` 功能对齐与已确定的 Agent 范围均通过。若实际范围/进度需要调整，另行明确版本边界，不静默削减已接受功能或用占位按钮凑齐界面。旧 `.3` 公开包与 tag 保留，开发预览不覆盖现有用户安装和数据。

## 原生验收重点

- 数学源码编辑、真实 Preview/Complete/Hover、ghost 补全、诊断修复、UTF-16/UTF-8 转换、中文/emoji 和原生撤销。
- 解的条件/重数/精确/数值、Root、矩阵、步骤、表格分页、二维采样、完整西瓜三维场景与实际导出。
- 窗口宽窄、分栏拖动、浅深主题、长公式、选区/滚动恢复、响应链、菜单、键盘和基础无障碍。
- 模型配置及 Keychain 命名/权限、旧笔记本往返、打开/另存为/保存失败和未保存状态，避免数据迁移丢失。
- Agent 与手工编辑共用版本、源码检查、事务和回执，用户编辑不被旧回复覆盖；原生媒体只声明实际接通能力。
- 原 53 数学语料、Swift 原生及 Mac 界面/桥接测试、安装包版本/依赖/许可证检查。其他平台保留原 UI并继续必要回归；本机不启动 iOS 模拟器。

## Skill 映射

使用 [telegram-ui-reference SKILL.md](/Users/hert/.agents/skills/telegram-ui-reference/SKILL.md) 的本地语言无关实现契约。Mac 控件适配到 AppKit/SwiftUI，真实业务回执和数学数据由 OpenMath 提供；不复制 Telegram UIKit 类体系或构建其源码。

具体本地依据：[窗口尺寸与方向](/Users/hert/Documents/ChatGPT/ui-learning/09-adaptation-and-accessibility/screen-size-and-orientation.md) 的状态保留与可用空间算法、[导航栈](/Users/hert/Documents/ChatGPT/ui-learning/02-layout-and-navigation/tabs-and-navigation-stacks.md) 的稳定身份/焦点/滚动恢复，以及 [文本选择和编辑](/Users/hert/Documents/ChatGPT/ui-learning/05-input-and-actions/text-selection-and-editing.md) 的原生选区、单次编辑事务与 Unicode 边界。右栏继续沿用已读的输入框、进度、菜单和撤销契约。

当前为可评审的整体方案与同版建议，Mac 原生重写未开始，最低系统版本和最终发行范围未锁定。
