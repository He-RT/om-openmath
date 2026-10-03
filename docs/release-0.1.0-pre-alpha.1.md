# OpenMath 0.1.0-pre-alpha.1

首个 GitHub 预发行版，提供 Windows x64 中文安装包、macOS Apple Silicon 桌面版、两平台 CLI 和 Web 静态包。

## 主要功能

- Rust 共享 CAS、现代语法与 Wolfram 语言子集。
- 精确/数值求解、实数有理不等式、消元、实际验证证据和推导步骤。
- 响应式笔记本、公式预览、交互绘图、中文/英文界面。
- `.omnb` 源码笔记本及 Markdown/LaTeX 导出。
- 模型接入：自然语言建议、讲解、只读 CAS 工具对话、修复和补全；建议等待用户操作。
- 中文默认 README、安装、语言、求解、AI、协议、开发及发布文档。

## 下载选择

| 场景 | 资产 |
|---|---|
| Windows 桌面，推荐 | `OpenMath_0.1.0-pre-alpha.1_x64-setup.exe` |
| Windows MSI 部署 | `OpenMath_0.1.0-pre-alpha.1_x64_zh-CN.msi` |
| macOS Apple Silicon | `OpenMath_0.1.0-pre-alpha.1_aarch64.dmg` / `OpenMath_0.1.0-pre-alpha.1_macos_arm64.app.zip` |
| Windows / macOS 终端 | `om-cli_0.1.0-pre-alpha.1_windows_x64.zip` / `om-cli_0.1.0-pre-alpha.1_macos_arm64.zip` |
| 静态网站 | `OpenMath-web_0.1.0-pre-alpha.1.zip` |
| 校验清单 | `release-manifest.json` |

Windows 支持 10/11 x64，包含 WebView2 引导程序，缺少运行时时需联网。此次没有 Intel Mac、Windows ARM64/32 位和 Linux 包。

## 验证与限制

同一提交构建，要求 CI、各平台构建及 Windows 两种包安装后的真实窗口验收通过。清单提供字节数、SHA256 和提交 SHA。原始 53 条语料的精确/700-bit 验收及本地性能见[验收记录](https://github.com/He-RT/om-openmath/blob/v0.1.0-pre-alpha.1/docs/pre-alpha.md)。

这是 pre-alpha，有界求解子集不等同于完整 Mathematica。不支持的输入保持原样并给出诊断。桌面包未代码签名，macOS 未公证，系统可能显示来源警告。Web 必须通过 HTTP(S) 提供资源，模型服务需允许 CORS。密钥不随发行包提供，也不写入笔记本。

[中文安装指南](https://github.com/He-RT/om-openmath/blob/v0.1.0-pre-alpha.1/docs/install.md) · [求解范围](https://github.com/He-RT/om-openmath/blob/v0.1.0-pre-alpha.1/docs/solve.md) · [反馈](https://github.com/He-RT/om-openmath/issues)
