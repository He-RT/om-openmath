# 安装指南

从[官方发行页](https://github.com/He-RT/om-openmath/releases)下载 `0.1.0-pre-alpha.1`。这是供试用的早期版本；支持范围见[求解指南](solve.md)。

## Windows 10/11 x64

推荐下载 `OpenMath_0.1.0-pre-alpha.1_x64-setup.exe`，按中文向导安装。EXE 默认安装到当前用户目录，开始菜单提供入口。在“设置 → 应用”中找到 OpenMath 可卸载。

也可下载 `OpenMath_0.1.0-pre-alpha.1_x64_zh-CN.msi`，双击安装，或在管理员终端执行：

```powershell
msiexec /i "OpenMath_0.1.0-pre-alpha.1_x64_zh-CN.msi" /qn /norestart
```

两种包包含 WebView2 引导程序；已有运行时直接使用，没有则需要联网下载。它们不是完整离线运行时包。包尚未签名，Windows 可能显示“未知发布者”；确认来源及校验值后继续。此次不提供 Windows ARM64/32 位包。

## macOS Apple Silicon

下载 DMG，打开后把 OpenMath 拖入“应用程序”，再启动。也可解压 `.app.zip`，将应用移动到“应用程序”。当前包未签名、未公证；首次启动被拦截时，在“系统设置 → 隐私与安全性”中确认来源并允许打开。卸载时将应用移入废纸篓，笔记本和配置独立于应用。

## 终端版

下载对应平台 `om-cli` ZIP，解压到固定目录。Windows 运行 `om.exe`，macOS 运行 `./om`；需要在任意目录使用时加入 PATH。首次运行不需要 AI 密钥。

```sh
om --no-config -e 'solve(x^2-5x+6=0,x)'
om run example.omnb
om config path
```

## Web 版

解压 `OpenMath-web_0.1.0-pre-alpha.1.zip` 到站点根目录，通过 HTTP(S) 提供静态资源。不要双击 `index.html` 使用 `file://`。本地试用在解压目录执行 `python3 -m http.server 8080`，随后打开 `http://localhost:8080`。

数学运算在本地 WASM Worker 中进行；AI 直接访问您配置的服务。服务拒绝 CORS 时使用桌面或终端版。

## 校验下载文件

发行页的 `release-manifest.json` 列出名称、字节数和 SHA256。计算下载文件的散列，与对应条目比较：

```powershell
Get-FileHash .\OpenMath_0.1.0-pre-alpha.1_x64-setup.exe -Algorithm SHA256
```

```sh
shasum -a 256 OpenMath_0.1.0-pre-alpha.1_aarch64.dmg
```

## 常见问题

| 现象 | 处理方式 |
|---|---|
| Windows 窗口无法启动 | 确认 WebView2 安装成功；首次安装允许运行时下载 |
| 需要中文界面 | 在“偏好设置 → 语言”选择简体中文；菜单语言在启动时确定 |
| 模型连接失败 | 检查地址、模型、密钥和网络，先用“测试连接” |
| 浏览器模型请求失败 | 确认服务允许当前站点 CORS，或使用桌面版 |
| 求解调用保持原样 | 查看诊断和支持范围；不支持不表示无解 |
| 导出 LaTeX 中文无法编译 | 选择支持 Unicode 的 TeX 引擎及字体 |

报告故障时提供步骤、平台和公式；不要公开密钥。
