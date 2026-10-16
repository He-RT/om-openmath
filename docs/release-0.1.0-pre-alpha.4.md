# OpenMath 0.1.0-pre-alpha.4

本次优先提供原生 macOS 应用和 Web 版。Mac 使用 SwiftUI/AppKit 编辑界面、Rust 计算内核、原生公式与 Markdown、二维曲线及基础 Metal 三维场景；基础助手支持配置模型、对话、读取和修改笔记本、运行及检查结果。

## 下载与运行

- **Mac：** 需要 Apple Silicon 和 macOS 27 或更新版本。打开 DMG，将 **OpenMath Preview.app** 拖入“应用程序”；也可解压 app ZIP 后放入应用程序目录。Node、Pi、公式字体及其他运行资源已随包提供，无需安装 Rust、Xcode、Node 或 npm。
- **Web：** 将 Web ZIP 解压到 HTTP(S) 静态网站服务器，通过浏览器访问。`file://` 直接打开不支持模块/WASM 加载。版本与源码身份可在设置底部的 OpenMath 信息中查看，Rust 版本由实际运行的内核读取。
- 三个分发包和 `release-manifest.json` 同版，清单提供源码提交、大小和 SHA256。Mac 的“关于 OpenMath Preview”显示实际包版本及源码提交。

## 签名与数据

这是实验性 pre-alpha。Mac 包使用本地 ad hoc 签名，尚未完成 Developer ID 签名及 Apple 公证；首次下载后的 Gatekeeper 检查可能阻止启动。签名完整性检查不等于已通过公证。

原生应用使用独立 Preview 数据目录，不自动读取或迁移旧版应用配置、密钥、聊天和计算状态。普通 `.omnb` v1 文件保持源码格式，打开不会自动运行。模型服务地址和密钥需在新应用中显式配置，密钥保存在系统钥匙串中；密钥和助手会话不写入 `.omnb`。

## 当前边界

- Mac 基础助手使用兼容 Chat Completions 的 HTTPS 服务。修改和运行需打开本次授权开关，网络中断后的未知操作通过原 ID 核对。
- Web 保留已有模型入口，直接访问服务受浏览器 CORS 策略约束。
- 本版不提供新的 Windows、iOS 或独立 CLI 安装包；已有旧版资产仍在原发行页。
- 高级媒体、长会话压缩、完整供应商协议、变量列表和极限网格交互继续后续开发；三维透明物体使用近似排序。
