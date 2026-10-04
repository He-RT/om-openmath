# OpenMath 0.1.0-pre-alpha.2

原生 iOS/iPadOS 27 客户端与桌面、Web、CLI 使用同一 Rust 数学内核，增加 SwiftUI/UIKit 笔记本、源码编辑、原生公式与 Markdown、真实步骤与 Canvas 绘图、AI 与系统 Keychain、UIDocument 文件操作。保留此前公开的 `pre-alpha.1`。

## 分发

继续提供 Windows x64 中文 EXE/MSI、macOS ARM64 DMG/应用 ZIP、两平台 CLI ZIP 与 Web 静态 ZIP；增加：

- `OpenMath_0.1.0-pre-alpha.2_ios_simulator_arm64.app.zip`：仅 ARM64 iOS 27 模拟器。
- `OpenMathKernel_0.1.0-pre-alpha.2.xcframework.zip`：真机/模拟器 ARM64 Rust 桥接静态库。

完整源码工程随仓库交付。`release-manifest.json` 记录同一版本、Git 提交、字节大小和 SHA256。缺失任何附件或同提交 CI 未通过时禁止发布。

## 安装及已知边界

见[安装说明](../README.md)与[移动端指南](ios.md)。桌面应用仍未签名/公证。iPhone/iPad 真机通过本机 Xcode 自动签名安装，不提供通用 IPA，不提交商店。最低 iOS/iPadOS 27，首版单窗口、同时编辑一个笔记本；无自动 iCloud 同步、多窗口或专用手写识别。

## 验收

[移动端验收记录](ios-acceptance.md)保存实际运行结果、失败修复、设备、计时与截图。原 53 条数学权威语料保持不变；Rust/桌面/Web 原有检查与移动端门禁需一同通过。本文件随最终验收结果更新，编译成功不等于完整发布验收。

本轮已验证 iPad 最大字号、普通键盘和减少动态效果。按用户决定，VoiceOver 完整朗读/焦点遍历、浮动键盘及真机窄窗口等剩余人工组合场景暂缓，未计为通过；iPhone 真机界面测试驱动的 code 74 也保留为已知工具限制。模拟器自动化门禁继续执行，真实计算、恢复、HTTP/凭据隔离和性能断言保持不变。
