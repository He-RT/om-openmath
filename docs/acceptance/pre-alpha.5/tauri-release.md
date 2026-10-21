# pre-alpha.5 Windows / Android 完整封装补包

2026-10-22，用户要求使用 Tauri 2，Windows 只提供 MSI，Android compileSdk / targetSdk 为 35，并在完成后公开发布；明确“体验不能有任何差别，完整封装”。本次复用 `.5` Web 的完整工作台、WASM 计算、二维／三维绘图、现有 Web Worker 助手、设置、图片附件与导出，Tauri 提供窗口和系统文件选择器。没有另开发助手，也没有裁剪现有 Web 功能。

## 源码与生成工程

共享产品源码为 `29481853927eb1603f94489426bb5781b38a1f86`。Windows 打包流程提交 `50b0ea986af6e62f8e8d165423e4fd4026fb0493`，Android 打包与生成工程脚本提交 `b01184696e38facd14878bc0aff76e7897f3c229`。打包工作流分别检出产品源码与打包脚本，包内 `version.json` 记录共享产品 SHA；Android 的 `MainActivity`、SDK 固定和启动图标由该打包脚本生成。复现需同时使用对应产品与打包提交，不能只检出旧 tag。

Windows 使用 WebView2；MSI 内附运行时引导程序，未安装运行时的系统需要联网补装。Android minSdk=24、compileSdk=35、targetSdk=35，版本代码 1005，NDK 28.2.13676358，包含 ARM64 和 x86_64；生成工程将 app 及所有 Android library 的 compileSdk 固定为 35。API 35 的系统栏、挖孔和输入法留白在容器处理，启动图标来自仓库既有 OpenMath 图标。

Android 用 build-tools 35.0.0 对齐后以持久 RSA 3072 发行身份签名；证书 SHA256 为 `260c4c87a10548a77838dd7c4f2772514deff46cfd3cafd8838a89c2c0c3c4fb`。签名材料不在仓库和公开资产中，后续更新沿用同一身份。Windows 没有 Authenticode 签名。

## 构建与直接相关检查

Windows [run 39635714219](https://github.com/He-RT/om-openmath/actions/runs/39635714219) 已通过：实际 MSI 安装、安装载荷核对、启动安装版 WebView2、在真实编辑器输入 `solve(x^2 - 5*x + 6 = 0, x)`，得到 `x = 2`、`x = 3`，并读回 `.5` 与完整产品 SHA。核对载荷只允许 Tauri 的 MSI 安装类型标记变化，其余字节一致。

Android [run 39644931736](https://github.com/He-RT/om-openmath/actions/runs/39644931736) 全部通过：正式签名 APK 安装至 API 35 x86_64 系统，在真实 WebView 编辑器输入同一求根式，得到 `x = 2`、`x = 3`，读回 `.5` 与完整产品 SHA。签名 v2/v3 验证通过，包清单确认 compileSdk=35、targetSdk=35、minSdk=24。最终截图显示顶部工具栏避开状态栏，系统栏文字可读；APK 中已是 OpenMath 图标。ARM64 原生库同包交付，未做 ARM64 实机检查；没有把模拟器结果写成真机结果。

相关前端平台选择／客户端测试 8 项、TypeScript 类型检查和发行清单检查 7 项已通过。本次没有重跑旧数学与性能完整矩阵，没有新增真实模型调用；助手、图片和完整交互沿用既有 Web 实现，未宣称在这两个新宿主逐项重新实测。模型服务仍需满足 WebView 的 CORS 条件；显式记住的密钥存于应用 WebView，不写入 `.omnb`。PDFKit 提取 PDF 文字是原生 Mac 功能，不属于 Web 功能集。

## 资产与发布

| 资产 | bytes | SHA256 |
|---|---:|---|
| OpenMath_0.1.0-pre-alpha.5_x64_zh-CN.msi | 15262316 | 4012c997d398831179f7161143d30e5cb7589d046979bd9227cb0b2f7e2935ea |
| OpenMath_0.1.0-pre-alpha.5_android.apk | 38383100 | b44e86b0fdbb0a314a8c7100c740948e5ca96e936559275139d607b230ddcb69 |

2026-10-22，MSI、APK 和 `tauri-release-manifest.json` 已追加到[原 `.5` Release](https://github.com/He-RT/om-openmath/releases/tag/v0.1.0-pre-alpha.5)。原 tag、Mac/Web/CLI 文件及其清单保留；新清单记录共享产品与各平台打包提交。没有再执行公开下载后的重复安装；上述结果来自实际构建产物的安装版。下载方式见[安装指南](../../install.md)。

## 已保留的失败记录

首轮 run 39631970867 的 Android SDK 安装引用旧 `tools` 包、Windows 将 VBScript 当作 optional feature，分别已修复。Windows run 39633042086 实际已计算得到 2/3，但外部检查沿用旧 `.solution-chip` 类误报超时；现以当前 `.solution-entry` 的精确 aria-label 核对。

Android run 39632833377 缺 KVM 访问导致软件模拟启动慢，已取消；后续启用临时 CI 虚拟机加速。run 39635860346 / 39639322376 的 release APK 安装成功，但 AndroidX JUnit 测试入口先后缺少被 release R8 精简的 Trace / Kotlin 类，未执行到计算，不算通过。正常启动检查 run 39638609209 显示工作台和 Kernel ready，另发现并修复 API 35 系统栏遮挡；后续又补系统栏文字对比度和 OpenMath 启动图标。最终检查改用 Android 系统自带 Instrumentation，测试 APK 不进入发布包，也不启用发布包调试。

纯系统检查的首轮 javac bootstrap 参数和模拟器镜像归档下载失败亦保留于 Actions；重试仅针对相关故障，不将失败、取消或未运行步骤记作成功。
