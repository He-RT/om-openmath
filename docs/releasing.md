# 发布流程

当前版本 `0.1.0-pre-alpha.1`，标签 `v0.1.0-pre-alpha.1`。[工作流](../.github/workflows/release.yml)从同一提交构建 Windows、macOS 与 Web；所有验证通过后发布预发行版。

## 版本与触发

同步根 `Cargo.toml` 的 workspace 包和内部依赖版本、`Cargo.lock`、npm 包及锁文件、Tauri 配置，以及发布脚本、测试和发行说明：

MSI 内部产品版本只能是数字，Windows 配置将 `0.1.0-pre-alpha.1` 映射为 WiX `0.1.0.1`；展示版本、资产名、标签及其余元数据仍为完整预发行版本。升级时同步这一映射。

```sh
python3 scripts/release.py verify --tag v0.1.0-pre-alpha.1
python3 -m unittest discover -s tests -v
```

推送已验证的 `dev`，等待同一提交 CI 成功。首个发行版手动触发，标签在门禁通过后由 GitHub Release 创建：

```sh
gh workflow run release.yml --ref dev -f tag=v0.1.0-pre-alpha.1
```

也支持 `v*` 标签推送，但仍要求版本一致、同一提交 CI 成功。发布不合并 `main`。失败时修复、验证、提交后重新触发；不移动已公开标签。

## 门禁

- Web：实际 WASM、lint、单元测试、开发与生产 UI，静态包包含许可。
- macOS arm64：原生 app/DMG、fmt、Clippy、工作区测试、协议漂移、CLI 原始语料及性能、DMG 校验。
- Windows x64：原生 EXE/MSI、Clippy、工作区测试；分别安装、启动真实中文窗口，验证精确根、步骤和响应式 3→6，再卸载。
- 发布：确认同一 SHA 的 CI 成功，拒绝缺失、额外、空文件或链接资产，生成 SHA256 清单。

Windows 仅在 CI 设置 `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9222`，并为管理员 runner 临时设置 HKLM WebView2 调试策略（结束后清除），Playwright 连接安装后的实际 WebView2；应用默认不开放调试端口。独立配置使用 `OPENMATH_CONFIG_PATH`。截图及结果在 Actions 的 `windows-install-evidence` artifact 中。

## 发布资产

七个安装/应用/CLI/Web 文件加 `release-manifest.json`，名称见 README。清单记录版本、标签、完整提交 SHA、字节数及 SHA256。

收齐、校验后创建完整 draft，再改为 pre-release；构建或安装测试失败不发布。说明来自 `docs/release-0.1.0-pre-alpha.1.md`；流程不使用真实模型密钥。

当前包未代码签名，macOS 未公证。说明保留架构、WebView2 联网要求和求解边界，后续签名使用 CI secret 管理证书。

WebView2 150+ 对管理员进程忽略环境变量/HKCU 参数；GitHub Windows runner 以管理员运行，所以 CI 临时设置仅针对 om-desktop.exe 的 HKLM AdditionalBrowserArguments，并在 always 清理。依据[微软 WebView2 权限说明](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/security#for-an-elevated-host-app-use-appropriate-override-flags)。分发程序不包含这项策略。

安装后程序逐字节核对各自构建输入，只允许 Tauri SDK 已记录的唯一 `__TAURI_BUNDLE_TYPE_VAR_UNK` 标记变为 NSIS 的 `NSS` 或 MSI 的 `MSI`，任何其他字节差异都拒绝。依据锁定 tauri-utils 2.10.0 的 platform.rs；两个实际安装文件已证明恰好只有这三字节变化。原始输入、安装后程序和散列报告保留为 Actions artifacts。
