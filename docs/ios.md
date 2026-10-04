# iOS / iPadOS 原生客户端

目标发行版本为 `0.1.0-pre-alpha.2`。最低 iOS/iPadOS 27，支持 iPhone 和 iPad、横竖屏、单窗口及硬件键盘。使用 SwiftUI/UIKit/TextKit 2、SwiftMath 和原生 Canvas；所有计算复用 Rust 内核。

## 构建与模拟器安装

需要 Apple Silicon Mac、Xcode **27.0**、iOS 27 SDK/模拟器，以及仓库固定的 Rust **1.94.0**。执行：

```sh
bash ios/Scripts/verify-environment.sh
bash ios/Scripts/build-kernel.sh
python3 ios/Scripts/generate-project.py
xcodebuild -project ios/OpenMath.xcodeproj -scheme OpenMath \
  -configuration Release \
  -destination 'platform=iOS Simulator,name=iPhone 18 Pro,OS=27.0' \
  -derivedDataPath target/ios-derived \
  -parallel-testing-enabled NO -collect-test-diagnostics never test
```

工程和包锁文件随源码提交。`build-kernel.sh` 产生 `ios/Frameworks/OpenMathKernel.xcframework`，包含真机 ARM64 与模拟器 ARM64，两者均为 Release Rust 静态库。编译产物不进入 Git。缺少指定 Xcode/SDK 时直接失败。

发行模拟器 ZIP 解压为 `OpenMath.app`，只能在 ARM64 iOS 27 模拟器中运行：

```sh
xcrun simctl boot 'iPhone 18 Pro'
xcrun simctl install booted /绝对路径/OpenMath.app
xcrun simctl launch booted org.openmath.OpenMath
```

## 本机真机安装

1. 连接并解锁设备，信任 Mac，开启开发者模式。
2. 在未跟踪的 `ios/Local.xcconfig` 写入 `DEVELOPMENT_TEAM = 你的TeamID`。工程通过可选 include 读取此文件。
3. 使用 `xcrun devicectl list devices` 获取 UDID，运行 `bash ios/Scripts/install-device.sh UDID`，或在 Xcode 打开工程、选择设备、运行。
4. 如系统提示签名尚未受信任，在设备「设置 → 通用 → VPN 与设备管理」中信任你自己的开发者配置后启动。

自动签名需要本机 Xcode 的 Apple 账户、证书和可用的开发描述文件。个人配置、证书、描述文件以及签名后的真机应用不作为公开附件。首版不提交 TestFlight 或 App Store；模拟器 ZIP 不是通用 iPhone 安装包。

## 使用

- 数学、文本、Ask 单元格可添加、删除、移动和切换方言。Modern 赋值使用 `let a=2`，Wolfram 赋值使用 `a=2`；执行定义后再执行 `a+1`，将定义改为 5 并执行会重算为 6。
- ⌘/⇧ Return 运行，Control Space 本地补全，Tab 接受，Escape 取消；触屏提供相同操作。中文合成时禁止运行及接受补全。`\alpha` 加 Tab 输入 α。
- 窗口可用宽度达到 696 pt 显示检查面板；较窄窗口使用底部面板。安全区与键盘由原生容器统一处理。
- 解卡片保留精确、数值验证、条件和重数；步骤来自内核记录。绘图使用真实采样，支持平移、缩放、复位和参数滑块。
- 设置中配置 AI 模型、功能映射和连接探测。首次发送披露目标；探测只发送短请求，不保存草稿。建议必须明确插入或运行。
- API 密钥、额外请求头和额外请求参数保存在系统 Keychain；笔记本只保存 `.omnb` v1 源码。不要把密钥作为单元格内容输入。
- 文件菜单提供打开、保存、另存为、分享及 Markdown/LaTeX 导出。自动保存源码；后台取消计算/AI 并保存草稿，恢复时不自动运行全部单元格。

## 接口和所有权

`om-ios-ffi` 独立 crate 提供 C ABI：创建/销毁会话、JSON 请求、HTTP 原始字节、中断、AI 取消和返回缓冲释放。Swift 串行工作队列调用专有 Rust 会话线程，UI 在 MainActor 更新；计算取消直接设置共享标志。每个打开的文档新建会话，旧请求通过文档代次隔离。

请求使用现有 `Envelope<Request>`，回复为 `{response, events}`；事件 id 为 0，回复 id 匹配请求。返回的 `OmBuffer` 仅在释放前可读，调用者必须恰好释放一次，禁止保留旧指针或访问已释放内存。无效 JSON/未知句柄返回错误对象。unsafe 只存在于审计过的 C ABI 输入边界；核心保持禁止 unsafe。

`om-kernel/external-host` 使用 Native 提供商请求格式，并由 Swift 的 URLSession/Keychain 提供网络和凭据；生产 iOS 依赖图不包含桌面配置、Rust keyring 或 Tokio HTTP。拒绝 HTTP 重定向，按原始字节反馈内核解码，取消和超时保留真实失败状态。

SwiftMath 1.7.3 和 swift-markdown 0.9.0，以及 swift-cmark 0.9.0 的提交在 Package.resolved 锁定。数学渲染只将平面 ASCII `operatorname` 映射到 SwiftMath 支持的 upright `mathrm`，原 LaTeX 和可复制源码保持不变；未知命令显示源码。Markdown 不自动下载远程图片。

## 验收与许可

参见 [移动端验收记录](ios-acceptance.md)、[实施进度](plan/PROGRESS.md) 和 [第三方声明](../THIRD_PARTY_NOTICES.md)。CI 在 `xcode-27` 显式选择 Xcode 27.0，验证两架构链接、Swift 单元测试和 iPhone/iPad 界面测试；沿用 Rust/桌面/Web 门禁。官方镜像依据：[runner-images #14404](https://github.com/actions/runner-images/issues/14404)。

首版不提供自动 iCloud 同步、多窗口、专用手写数学识别或 Metal 粒子效果。
