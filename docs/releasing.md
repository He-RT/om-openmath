# 发布流程

当前公开版本为 [0.1.0-pre-alpha.5](https://github.com/He-RT/om-openmath/releases/tag/v0.1.0-pre-alpha.5)，标签 `v0.1.0-pre-alpha.5`，原 Mac/Web 与 CLI 产品源码为 `86da06206c1ce547921ff2fb6ada892c5b090b48`。原版发布原生 Mac DMG、app ZIP、Web ZIP 和 manifest，随后追加两平台 CLI ZIP，以及 Tauri 2 Windows MSI / Android APK。各补包使用独立清单，Tauri 清单记录实际封装源码，不冒用原 tag SHA。完整实施与验收见 [PRE_ALPHA_5.md](plan/PRE_ALPHA_5.md)。

## 版本与构建

同步 Rust workspace、内部依赖及 Cargo 锁文件、app/agent 的 npm 版本及锁文件、Tauri 配置、原生 Mac/iOS Info.plist、函数目录、发布脚本与发行说明。Windows 的 WiX 数字版本映射也由版本检查覆盖。

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

该流程包含生产 WASM 原 53 条与性能检查、原生 Mac 打包及完整 manifest。产品提交还需核对 Rust、frontend、dependencies、native 各项实际结果，不能只看总体 green；后追加的 Windows / Android 使用下方独立流程，iOS 不属于本版发行门禁。

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

## Tauri 2 Windows / Android 补充发行

[独立封装工作流](../.github/workflows/release-shells.yml)从 `source_sha` 检出产品代码，并单独检出当前流程的打包与检查脚本；`OPENMATH_SOURCE_REVISION` 写入实际产品 SHA，允许仅修打包流程时不重建已通过的平台。`platform` 支持 `all`、`windows`、`android`。

```sh
gh workflow run release-shells.yml --ref dev -f platform=all -f source_sha=FULL_TAURI_SOURCE_SHA
python3 scripts/release.py finalize --scope tauri --tag v0.1.0-pre-alpha.5 --root TAURI_ASSETS_DIRECTORY --sha FULL_TAURI_SOURCE_SHA --windows-packaging-sha FULL_WINDOWS_PACKAGING_SHA --android-packaging-sha FULL_ANDROID_PACKAGING_SHA
```

该目录仅含 `OpenMath_0.1.0-pre-alpha.5_x64_zh-CN.msi` 与 `OpenMath_0.1.0-pre-alpha.5_android.apk`，生成 `tauri-release-manifest.json`；`revision` 记录共享产品源码，`packaging_revisions` 记录两平台打包与生成工程源码。Windows 只构建 MSI，包内 WebView2 引导程序用于补装运行时。Android app 与所有库 compileSdk=35，targetSdk=35、minSdk=24；构建 ARM64 / x86_64 通用 APK，用 build-tools 35.0.0 `zipalign -P 16` 后签名。持久签名材料保存在受限本地目录和 GitHub Secrets，不放仓库或公开资产；后续更新必须沿用同一身份。

实际包检查只覆盖新增封装：Windows 安装 MSI、核对安装载荷、启动安装版 WebView2 并实际计算；Android 安装同一正式签名 release APK，通过仅用于 CI 的 instrumentation APK 在 API 35 系统 WebView 内实际计算，发布包不启用调试。均核对 `.5`、完整产品 SHA 及双根 2/3。此补包不重复真实模型调用或原版完整性能／数学矩阵。已做检查及未测范围见[Tauri 补包记录](acceptance/pre-alpha.5/tauri-release.md)。

用户已明确授权封装完成后公开发布。将两包和独立清单追加到原 `.5` Release；不使用 `--clobber`，不移动 tag、不覆盖原 Mac/Web/CLI 包或清单。同步安装文档与发行正文中的源码、签名、运行环境及 CORS 条件。

## 历史多平台发行

`.3` 的 Windows EXE/MSI、独立 CLI、移动模拟器应用与 XCFramework 仍保留在[旧发行页](https://github.com/He-RT/om-openmath/releases/tag/v0.1.0-pre-alpha.3)。其 Windows WebView2、Tauri 安装类型标记和 iOS 签名流程属于历史多平台发行；当前 Mac 为 SwiftUI/AppKit 原生包，不能以旧 Tauri 包或旧移动门禁替代本版验收。历史细节见 [`.3` 发行说明](release-0.1.0-pre-alpha.3.md)、[`.4` 计划](plan/PRE_ALPHA_4.md)及 Git 历史。

## 统一来源索引（后续构建）

三个发行工作流在各平台构建和适用检查成功后，额外输出独立 `*-sources.json`。它记录实际检出的产品 SHA、打包/工作流 SHA、workflow 文件散列、构建参数、工具版本与 runner 镜像、依赖锁和工具链声明的散列、run/attempt URL、检查范围，以及原资产的大小和 SHA256。不可用工具明确记为 unavailable；托管镜像和外部下载并未全部固定，不保证字节级复现。签名私钥和密码不记录。

Mac/Web 工作流在合并现有 manifest 后自动生成并验证 `release-sources.json`。CLI/Tauri 仍允许单独补构建某个平台；下载所选成功 run 的原资产及 source artifacts 后，在本地将所有包、原 manifest 和 source sidecars 放入同一目录，生成新的统一索引。例如仅汇总一次 CLI 补包：

```sh
# 从每个实际 source record 中取完整 SHA，先确认 origin 为本项目，再逐个取回。
git fetch origin FULL_PRODUCT_SHA
git fetch origin FULL_PACKAGING_SHA
python3 scripts/release.py sources index --repo . --assets-root target/new-release \
  --manifest target/new-release/cli-release-manifest.json \
  --record target/new-release/cli-macos_arm64-sources.json \
  --record target/new-release/cli-windows_x64-sources.json \
  --output target/new-release/release-sources-cli-01.json
python3 scripts/release.py sources verify --repo . --assets-root target/new-release \
  --index target/new-release/release-sources-cli-01.json
```

完整发行可重复传入三份 `--manifest` 和全部七份平台 `--record`，得到单一入口；补包使用新的索引文件名，并明确覆盖哪些资产。验证要求每个资产恰好有一份来源，来源提交/工作流/依赖锁可从 Git 取回，并与原 manifest、实际资产及 sidecar 的字节散列一致。缺失来源、错误提交、错误平台/工作流/依赖散列、资产篡改和重复来源都会失败；输出文件已存在时拒绝覆盖。索引用于关联可审查的构建记录，不是经过独立签名的供应链证明。

公开新发行或补包时，把索引、它引用的 source sidecars 和原 manifest 一起放入持久 Release；不能只保留会过期的 Actions artifacts 或本机临时文件。校验下载后的资产时使用同一 `sources verify` 入口。

准备新版本时，在适用 CI 和实际产品验收通过的最终产品提交上使用附注标签，记录可能不同的各平台打包 SHA。既有 `.1`–`.5` 轻量标签、清单和公开资产均保持原样；`.5` 的原产品、CLI 和 Tauri 补包具有多提交来源，不能追记为单标签构建。旧版来源取回演练只使用已有产物，不新派发发行或真实模型任务。
