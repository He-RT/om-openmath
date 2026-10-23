# pre-alpha.5 iOS / iPadOS 补包

2026-10-23。用户要求开始制作并发布 `.5` 的 iOS / iPadOS 版，先完成内容，只有获得其后续明确许可才能启动模拟器验证。沿用单代理、原生实现、不改系统权限/输入法/外观、按需清理自有构建缓存的约束。此追加范围覆盖旧 `.5` 计划中排除 iOS 的条款；Git 走短期分支 → PR → 适用 CI → dev。

## 交付目标

- SwiftUI / UIKit 原生工作台，遵循 [原生玻璃草稿 02](../design/ios-ipados-workbench-02.md)：清晰纸面、浅灰源码槽、状态左轨、悬浮原生玻璃工具组。
- iPhone 单栏与底部检查器；iPad 宽窗口目录 / 文稿 / 检查器，窄窗口回到单栏。Duo 以真实容器尺寸布局，保留同一文稿、编辑、检查器和滚动状态；不把草稿硬编码的安全边缘 / 折叠区域当作硬件数据。
- 复用 `.5` Rust 数学内核，原生数学/Markdown、二维与三维、步骤、变量、文件读写/导出和模型设置；补齐移动助手的执行/讨论、操作记录、撤销、附件和持久草稿。不得将旧只读 CAS 对话称为 `.5` 助手完成。
- 在已有 GitHub `.5` pre-release 追加独立移动资产和来源清单，原标签和已有资产保持不变。默认交付可由用户自行签名的真机开发包、模拟器开发包、XCFramework；不是 App Store / TestFlight 或无需签名的通用安装包。

## 顺序与检查边界

1. 核对并补齐生产内容、工程生成、资源与包内版本。
2. 无模拟器阶段：针对新逻辑的宿主/契约检查、Rust FFI 回归、真机目标的 unsigned 构建，以及模拟器目标的 **build-only**（不得 boot / install / launch / test）。日志进入自有 `target/ios-pre-alpha.5/`；不重复 Mac/Web 冻结验收、不调用真实模型。
3. 内容和候选构建可评审后，请用户明确允许启动模拟器。许可前不派发原 `ios.yml`，该工作流会自动启动模拟器。
4. 获许可后，按实际改动验证 iPhone / iPad、文件保存回读、编辑/键盘、布局与状态、求根/二维/三维及助手回环；真实模型如需调用另核对既有预算与范围。记录实际覆盖，未测项不记为通过。
5. 通过后按 Git 管理规则集成并发布新增资产，更新安装文档、README 和独立补包记录。公开后不覆盖已有同名资产。

## 当前进度

- 基线 `41a49068f33e78e12bee002d35d2d190ccce853f`，分支 `codex/ios-pre-alpha-5`。Xcode / iPhoneOS SDK 27.0、Rust 1.94.0；两个 iOS ARM64 目标已安装。最近磁盘约 78 GiB 可用。
- [x] 原生工作台内容：纸面、源码槽、状态轨、原生玻璃工具组、宽度布局与控制器持有的编辑/阅读状态。
- [x] 原生三维实现：Rust 真实采样、SceneKit Metal 显示、相机交互与 OBJ 导出；几何边界检查已过，实际 GPU/触控交互待验证。
- [x] 移动 `.5` 助手实现：Pi 1.0.4 + JavaScriptCore、共享模型 codec、执行/讨论、源码事务/撤销、附件和持久状态。停止后等待旧任务退场；保留同轮中已完成工具的消息配对，不丢弃整条提案。
- [x] 工程 / 资源 / 包内版本与开发包来源记录入口：独立 `.5` bundle ID 和存储、生成资源一致性、build-only/打包脚本及默认不开模拟器的补包工作流。最终公开来源记录尚未生成。
- [ ] 最终提交的双目标候选构建与打包（开发源码真机 Release 已过，准备生成准确 revision）。
- [ ] 用户允许模拟器后验证
- [ ] 集成、公开补包与文档同步

已完成的无模拟器检查：

- Rust FFI/能力/三维相关 26 项通过；`target/ios-pre-alpha.5/native-math-tests-02.log`。其后 Rust 仅格式调整，当前 `cargo fmt --all -- --check` 通过。
- 当前 Swift 真机 ARM64 unsigned Release 通过；`device-build-current.log`。较早的 `simulator-build-only.log` 是编译成功记录，不代表最新源码已在模拟器运行。
- `check-native.sh` 通过；`host-native-current.log`。使用真实 macOS JavaScriptCore/Pi 调度，覆盖两轮、真实工具调用与持久配对；纯 Swift 覆盖源码冲突拒绝、事务/草稿/附件持久化、部分停止轮次消息配对、运行依赖与非法几何。没有网络请求。
- 发行契约 8 项、iOS 静态契约 2 项、生成资源一致性、工程生成、actionlint、脚本语法、Git 忽略边界和 diff 空白检查通过。

尚未执行 iPhone/iPad 的 XCTest/UI、安装回读、硬件键盘/触控、性能、真实模型和 Duo 折叠验证；未创建、启动、安装或运行模拟器。27.1 专用折叠区域/纵向工具栏 API 不在本机 27.0 SDK 内，不能记为 Duo 设备验收。没有 Apple Distribution 签名身份，真机资产为需自行签名的开发应用，不是通用 IPA / TestFlight / App Store 包。
