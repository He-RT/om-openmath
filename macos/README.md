# 原生 Mac 开发与发行

原生 SwiftUI/AppKit 客户端已随 [0.1.0-pre-alpha.5](https://github.com/He-RT/om-openmath/releases/tag/v0.1.0-pre-alpha.5) 公开发布，要求 **Apple Silicon、macOS 27+**。本版提供 DMG 和 app ZIP，应用名为 **OpenMath Preview.app**；最终用户无需安装 Rust、Xcode、Node 或 npm。下载与已知限制见[安装指南](../docs/install.md)及[发行说明](../docs/release-0.1.0-pre-alpha.5.md)。

## 当前实现

- 工作台 v2：目录、数学源码槽、纸面结果、文字阅读／编辑、独立代码块、二维／Metal 三维，以及助手／步骤／变量检查器。
- Rust `om-apple-ffi` 提供真实隔离计算、能力查询、源码计划和回执；Swift 负责 AppKit 编辑、事件投影、SQLite 存储及原生撤销。
- 随包 Pi/Node 助手支持执行与讨论、按消息的操作记录、差异、撤销、停止、模型配置、图片与 PDF 文字。结果依据宿主实际回执，模型文字不充当执行证明。
- `.omnb` v1 保存源码，打开不自动运行；模型、密钥和助手会话不写入笔记本文件。Finder 文件事件已接通实际文稿打开。

本版独立 Bundle ID 为 `org.openmath.OpenMath.NativeMacPreview`，数据根为系统应用支持目录下的 `OpenMath/NativeMacPreview/pre-alpha.5`。不自动迁移 `.4` 的配置、密钥或会话，不读取旧 Tauri 数据。模型必须显式配置，密钥保存在系统钥匙串。

## 环境与构建

需要 Xcode 27.0 / macOS 27.0 SDK、Swift 6、Rust 1.94.0，以及构建脚本所需的 GnuPG 和锁定 npm 依赖。在仓库根目录执行：

```sh
npm ci --prefix app --ignore-scripts
npm ci --prefix agent --omit=dev --ignore-scripts
bash macos/Scripts/verify-env.sh
python3 macos/Scripts/generate-project.py
python3 macos/Scripts/generate-project.py --check
bash macos/Scripts/build-native.sh
```

产物为 `target/macos-native/Build/Products/Release/OpenMath Preview.app`。工程可直接用 Xcode 打开，不需要第三方工程生成器。脚本构建 Rust 静态库及随包 helper，应用不依赖用户系统 Node 或本机 Homebrew 动态库。

正式打包使用固定、已提交的源码；脚本记录完整 revision，拒绝覆盖已有同名包，并严格检查签名和运行资源：

```sh
bash macos/Scripts/package-native.sh release-assets
```

该命令生成原生 DMG 和 app ZIP；Web ZIP 与 manifest 按[发布流程](../docs/releasing.md)生成。当前签名为 **ad hoc，未公证**，不等于 Developer ID 发行签名。

## 回归与维护

按改动选择已有脚本，避免每次重复全部验收。以下为常用入口；Swift 功能 fixture 与 Release 构建不能并行，因为它们共享 cmark 产物：

```sh
bash macos/Scripts/test-native-contracts.sh
bash macos/Scripts/test-workbench.sh
bash macos/Scripts/test-storage.sh
bash macos/Scripts/test-saves.sh
bash macos/Scripts/test-recovery.sh
bash macos/Scripts/test-undo.sh
bash macos/Scripts/test-kernel-runtime.sh
```

`test-workbench.sh --file-events` 覆盖真实文件 URL 的冷／热打开、非文件与多 URL 拒绝及存储关闭。性能使用现有 `measure-performance.sh` 的前台 AppKit 模式；不要用离屏绘制替代实际窗口性能或 Metal 视觉验收。真实模型调用另需遵守本版预算与授权，不以功能 fixture 或合成服务冒充。

发行源码 `86da06206c1ce547921ff2fb6ada892c5b090b48` 的本地两包安装、Finder 冷启动、西瓜和求根 2/3、保存读回与版本检查均已通过，同 SHA CI 已通过。公开下载后的重复验证由用户明确取消；详见[安装与发布记录](../docs/acceptance/pre-alpha.5/local-install.md)。OpenPanel 间歇禁用“打开”仍为已知限制，可用已实测的 Finder“打开方式 → 其他”指定本次安装应用。

## 设计与交接

- [当前计划及完成记录](../docs/plan/PRE_ALPHA_5.md)、[工作台 v2 设计](../docs/design/workbench-v2.md)、[交接入口](../CODEX_HANDOFF.md)
- [宿主与状态](../docs/design/macos-host-state.md)、[存储与恢复](../docs/design/macos-storage-recovery.md)
- [编辑与渲染](../docs/design/macos-editor-rendering.md)、[源码历史与撤销](../docs/design/source-history-retention.md)、[文件保存](../docs/design/native-file-saves.md)
- [安装发行设计](../docs/design/macos-installation.md)、[助手工具契约](../docs/design/agent-tools.md)

设计文档中的未来能力以实施和真实验收记录为准，不能据设计文字声称已经实现。旧 `.4` 及更早实现阶段保留在 Git 历史和旧计划中。
