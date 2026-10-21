# 发布流程

当前公开版本为 [0.1.0-pre-alpha.5](https://github.com/He-RT/om-openmath/releases/tag/v0.1.0-pre-alpha.5)，标签 `v0.1.0-pre-alpha.5`，发行源码为 `86da06206c1ce547921ff2fb6ada892c5b090b48`。原版发布原生 Mac DMG、app ZIP、Web ZIP 和 manifest，随后追加 macOS ARM64 / Windows x64 CLI ZIP 及独立补充清单；后续文档提交不改变包内 revision。完整实施与验收见 [PRE_ALPHA_5.md](plan/PRE_ALPHA_5.md)。

## 版本与构建

同步 Rust workspace、内部依赖及 Cargo 锁文件、app/agent 的 npm 版本及锁文件、Tauri 配置、原生 Mac/iOS Info.plist、函数目录、发布脚本与发行说明。历史 Windows 配置的 WiX 数字版本映射也由版本检查覆盖，不表示本版发布 Windows。

```sh
python3 scripts/release.py verify --tag v0.1.0-pre-alpha.5
```

在固定源码提交上构建，记录完整 SHA。Mac 使用 Xcode 27 / macOS 27 SDK、Apple Silicon；Swift 功能 fixture 与 Release 构建不并行，避免共享 cmark 产物冲突。

```sh
bash macos/Scripts/package-native.sh release-assets
npm ci --prefix app
npm run build --prefix app
```

Web 将 `app/dist` 的内容直接打包为 `OpenMath-web_0.1.0-pre-alpha.5.zip`，保留许可文件；构建元数据记录源码 SHA。三包齐备后生成清单：

```sh
python3 scripts/release.py finalize --tag v0.1.0-pre-alpha.5 --root release-assets --sha FULL_SOURCE_SHA
```

`finalize` 拒绝缺失、额外、空文件或链接资产，并记录每包的大小和 SHA256。清单及包内 revision 必须对应构建提交，不能用后续文档提交替换。

## CI 与本地验收

[发行工作流](../.github/workflows/release.yml)仅通过 `workflow_dispatch` 构建 Mac/Web 并上传 Actions artifacts，不直接创建公开 Release，也不由 tag push 自动触发：

```sh
gh workflow run release.yml --ref dev -f tag=v0.1.0-pre-alpha.5
```

该流程包含生产 WASM 原 53 条与性能检查、原生 Mac 打包及完整 manifest。产品提交还需核对 Rust、frontend、dependencies、native 各项实际结果，不能只看总体 green；Windows/iOS 不属于本版发行门禁。

公开前按计划完成设计、真实助手场景、数学、性能、存储／保存／恢复／撤销回归，并从实际本地三包安装或解压，验证示例西瓜、求根 2/3、版本和完整 SHA。`.5` 的[源码 CI](https://github.com/He-RT/om-openmath/actions/runs/39533915333)与[本地三包验收](acceptance/pre-alpha.5/local-install.md)已通过。

## 公开发布

用户明确批准具体源码提交与三包后，创建新 GitHub pre-release，上传以下四个文件；不合并 `main`，不移动已公开 tag 或替换旧版资产：

- `OpenMath_0.1.0-pre-alpha.5_aarch64.dmg`
- `OpenMath_0.1.0-pre-alpha.5_macos_arm64.app.zip`
- `OpenMath-web_0.1.0-pre-alpha.5.zip`
- `release-manifest.json`

发行说明来自 [release-0.1.0-pre-alpha.5.md](release-0.1.0-pre-alpha.5.md)，如实说明 ad hoc 签名、未公证、macOS 27 / Apple Silicon、HTTP/CORS、独立 Preview 数据目录及已知限制。发布过程不使用真实模型密钥。

本次用户已批准公开，Release 于 2026-10-21 创建成功。原计划的公开后匿名下载、核对哈希、重新安装回读，随后由用户明确要求“不用验证了，同步更新readme等文档”而取消；只记录已完成的本地验收，不将这部分标为实测通过。此调整仅适用于本次发行，不自动免除后续版本的验收。

## CLI 补充发行

[独立 CLI 工作流](../.github/workflows/release-cli.yml)从指定完整 SHA 检出源码，分别在 macOS 和 Windows 构建现有 `om-cli`，不构建图形应用或接入新助手。打包脚本保留运行文件、`BUILD.json`、使用说明、项目和 Rust 依赖许可，并在各平台解压 ZIP 后检查 `--version` 和求根 2/3。

```sh
gh workflow run release-cli.yml --ref dev -f source_sha=86da06206c1ce547921ff2fb6ada892c5b090b48 -f version=0.1.0-pre-alpha.5
python3 scripts/release.py finalize --scope cli --tag v0.1.0-pre-alpha.5 --root CLI_ASSETS_DIRECTORY --sha 86da06206c1ce547921ff2fb6ada892c5b090b48
```

CLI 输出目录只放两个对应版本的 ZIP；清单命名为 `cli-release-manifest.json`。检查完成后追加这三个文件到原 Release，不覆盖原 `release-manifest.json`、三包或 tag。本次用户明确要求补两平台现有 CLI、不加助手、不做无关验证，因此仅执行新增清单逻辑及各平台实际 ZIP 的启动／版本／求根检查，不重复旧版图形或模型验收。

## 历史多平台发行

`.3` 的 Windows EXE/MSI、独立 CLI、移动模拟器应用与 XCFramework 仍保留在[旧发行页](https://github.com/He-RT/om-openmath/releases/tag/v0.1.0-pre-alpha.3)。其 Windows WebView2、Tauri 安装类型标记和 iOS 签名流程属于历史多平台发行；当前 Mac 为 SwiftUI/AppKit 原生包，不能以旧 Tauri 包或旧移动门禁替代本版验收。历史细节见 [`.3` 发行说明](release-0.1.0-pre-alpha.3.md)、[`.4` 计划](plan/PRE_ALPHA_4.md)及 Git 历史。
