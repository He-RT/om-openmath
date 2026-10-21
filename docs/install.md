# 安装指南

当前版本为 [0.1.0-pre-alpha.5](https://github.com/He-RT/om-openmath/releases/tag/v0.1.0-pre-alpha.5)，提供原生 Mac 与 Web。Windows、独立 CLI 与 iOS 附件继续从 [pre-alpha.3](https://github.com/He-RT/om-openmath/releases/tag/v0.1.0-pre-alpha.3) 下载。本项目仍为实验性 pre-alpha；功能边界见[求解指南](solve.md)。

## macOS 27+ Apple Silicon

下载 `OpenMath_0.1.0-pre-alpha.5_aarch64.dmg`，打开后把 **OpenMath Preview.app** 拖入“应用程序”。也可解压 `OpenMath_0.1.0-pre-alpha.5_macos_arm64.app.zip`，再移动应用。包内自带 Node、Pi、公式字体及运行资源，无需另装 Rust、Xcode、Node 或 npm。

应用使用 ad hoc 签名，尚无 Developer ID 签名或 Apple 公证；签名完整性检查不等于公证。首次启动若被 Gatekeeper 拦截，请在核对来源后按 macOS 提示处理。卸载时将应用移入废纸篓；笔记本和配置独立于应用。

本版使用独立 `.5` Preview 数据目录，不自动读取或迁移 `.4` 的配置、密钥和会话。模型需重新显式配置，Mac 密钥保存在系统钥匙串；更新 ad hoc 签名候选后可能再次询问访问许可。

通过“文件 → 打开”选择 `.omnb`。系统文件面板间歇出现有效文件的“打开”按钮禁用时，可在 Finder 右键文件，选择“打开方式 → 其他”，精确指定本次安装的 OpenMath Preview.app；此路径已在本版两种安装包上通过冷启动与保存读回验收。

打开[示例笔记本](examples/README.md)后手动运行；普通 `.omnb` 只保存源码，不缓存结果或自动计算。添加数学单元格并运行 `solve(x^2 - 5*x + 6 = 0, x)`，可得到 2、3。“关于 OpenMath”显示应用版本、内核版本和源码提交。

## Web 版

解压 `OpenMath-web_0.1.0-pre-alpha.5.zip` 到站点根目录，通过 HTTP(S) 提供静态资源。不要双击 `index.html` 使用 `file://`。本地试用可在解压目录执行：

```sh
python3 -m http.server 8080
```

随后打开 `http://localhost:8080`，导入 `.omnb` 并运行。数学运算在本地 WASM Worker 中进行；助手访问您配置的服务，服务需允许当前站点 CORS。设置底部显示应用版本、实际 Rust 内核版本和源码提交。

## 校验下载文件

发行页的 `release-manifest.json` 列出同一源码提交构建的三个包、字节数和 SHA256。可自行计算下载文件的散列，与对应条目比较：

```sh
shasum -a 256 OpenMath_0.1.0-pre-alpha.5_aarch64.dmg
shasum -a 256 OpenMath_0.1.0-pre-alpha.5_macos_arm64.app.zip
shasum -a 256 OpenMath-web_0.1.0-pre-alpha.5.zip
```

本版发布前本地三包安装与同 SHA CI 已通过；发布后的匿名下载、哈希复核和重复安装回读按用户要求省略，未记为实测通过。详见[安装验收与发布记录](acceptance/pre-alpha.5/local-install.md)。

## Windows 10/11 x64（旧版 `.3`）

从 [pre-alpha.3](https://github.com/He-RT/om-openmath/releases/tag/v0.1.0-pre-alpha.3) 下载 `OpenMath_0.1.0-pre-alpha.3_x64-setup.exe`，按中文向导安装；也可使用 `OpenMath_0.1.0-pre-alpha.3_x64_zh-CN.msi`。EXE 默认安装到当前用户目录，可在“设置 → 应用”卸载。切换 EXE/MSI 时，安装器可能沿用之前的安装目录。

```powershell
msiexec /i "OpenMath_0.1.0-pre-alpha.3_x64_zh-CN.msi" /qn /norestart
Get-FileHash .\OpenMath_0.1.0-pre-alpha.3_x64-setup.exe -Algorithm SHA256
```

两种包包含 WebView2 引导程序；已有运行时直接使用，没有则需要联网下载。包尚未签名，Windows 可能显示“未知发布者”。不提供 Windows ARM64/32 位包。

## 终端版（旧版 `.3` 资产或源码构建）

从 `.3` 发行页下载对应平台 `om-cli` ZIP，解压到固定目录；Windows 运行 `om.exe`，macOS 运行 `./om`，需要时加入 PATH。也可按[开发指南](development.md)构建当前源码。首次运行不需要 AI 密钥。

```sh
om --no-config -e 'solve(x^2-5x+6=0,x)'
om run example.omnb
om config path
```

## iPhone / iPad（旧版开发附件）

最低 iOS/iPadOS 27。旧版提供 ARM64 模拟器应用和源码 Xcode 工程，真机通过本机自动签名安装；公开 ZIP 不含个人签名材料。本版没有新移动端安装包，详见[原生移动端指南](ios.md)。

## 常见问题

| 现象 | 处理方式 |
|---|---|
| Mac 有效 `.omnb` 的“打开”不可用 | 用 Finder“打开方式 → 其他”指定本次安装应用 |
| Mac 更新后没有旧模型或会话 | `.5` 使用独立 Preview 数据目录，需要显式配置 |
| Windows 窗口无法启动 | 确认 WebView2 安装成功；首次安装允许运行时下载 |
| 模型连接失败 | 检查设置中的地址、模型、密钥、能力和网络 |
| 浏览器模型请求失败 | 确认服务允许当前站点 CORS，或使用原生 Mac |
| 求解调用保持原样 | 查看诊断和支持范围；不支持不表示无解 |
| 导出 LaTeX 中文无法编译 | 选择支持 Unicode 的 TeX 引擎及字体 |

报告故障时提供步骤、平台和公式；不要公开密钥。签名、附件及功能限制见[本版发行说明](release-0.1.0-pre-alpha.5.md)。
