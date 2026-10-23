# iOS / iPadOS `.5` 补包记录

2026-10-23，原生移动开发附件已追加到 [v0.1.0-pre-alpha.5](https://github.com/He-RT/om-openmath/releases/tag/v0.1.0-pre-alpha.5)。用户授权制作并发布；内容完成后允许 Device Hub 单台模拟器验收。原标签仍指向 `86da06206c1ce547921ff2fb6ada892c5b090b48`，本次移动包具有独立来源，不用原 tag 冒充其产品输入。

## 源码、构建与集成

- 产品和打包提交：`c38d678d461c9f12bf3e9ea686af192db51a033c`。
- 主实现 [PR #7](https://github.com/He-RT/om-openmath/pull/7)，产品提交 `bba0448e42c9216cc9ca22e3b72b3f0bb71019a3`，完整适用 [CI 39934092109](https://github.com/He-RT/om-openmath/actions/runs/39934092109) 通过；普通合并提交 `e060c4ad1d4de28aec79cd37f942c3329719e2b2`。
- Unicode 源码身份修复 [PR #8](https://github.com/He-RT/om-openmath/pull/8)，[适用 CI 39937454919](https://github.com/He-RT/om-openmath/actions/runs/39937454919) 通过；普通合并提交 `343bf846c7e7797fe0768c59e11cf84c8055f84c`。
- 最终[发行构建 39937450343](https://github.com/He-RT/om-openmath/actions/runs/39937450343)，workflow 为 `.github/workflows/release-ios.yml`；`source_sha` 和 workflow SHA 均为上述 `c38d678d`，`run_simulators=false`、`publish=false`。
- GitHub 托管 ARM64 macOS 27 / `xcode-27` 镜像 `20261022.0305.3`，Xcode 27.0 / SDK 27.0、Rust/Cargo 1.94.0。真机 Release 与模拟器 build-for-testing、资源一致性、归档和来源校验均通过，没有再次启动模拟器。

下载 `ios-pre-alpha-5-candidate` 后核对六文件集合、ZIP 完整性、两个应用的版本和完整 revision、`org.openmath.OpenMath.pre-alpha5`、iPhone/iPad 设备声明 `[1,2]`、最低系统 27.0 及四项助手资源。Apple 数字版本为 `CFBundleShortVersionString=0.1.0`、`CFBundleVersion=5`，`OpenMathReleaseVersion=0.1.0-pre-alpha.5`。来源校验使用：

```sh
python3 scripts/release.py sources verify --repo . \
  --assets-root target/ios-pre-alpha.5/release-39937450343 \
  --index target/ios-pre-alpha.5/release-39937450343/ios-release-sources-index.json
```

结果为 `Verified 3 assets and all recorded Git inputs`。三个包、manifest、source sidecar 和来源索引一起公开，完整 Git 输入、工具链、锁文件与资产散列可从持久 Release 取回。

## 公开资产

| 文件 | 字节 | SHA256 |
|---|---:|---|
| [OpenMathKernel_0.1.0-pre-alpha.5.xcframework.zip](https://github.com/He-RT/om-openmath/releases/download/v0.1.0-pre-alpha.5/OpenMathKernel_0.1.0-pre-alpha.5.xcframework.zip) | 90720114 | `ebbef74924d5f7f9dc5b882e16205396fd3d22d5e207f115317035c1b2555aa8` |
| [OpenMath_0.1.0-pre-alpha.5_ios_device_arm64_unsigned.app.zip](https://github.com/He-RT/om-openmath/releases/download/v0.1.0-pre-alpha.5/OpenMath_0.1.0-pre-alpha.5_ios_device_arm64_unsigned.app.zip) | 18001215 | `52c9a985200f5150d5d3503502b6e9b2ca1ea42d2de6a56be7c25612d71255cb` |
| [OpenMath_0.1.0-pre-alpha.5_ios_simulator_arm64.app.zip](https://github.com/He-RT/om-openmath/releases/download/v0.1.0-pre-alpha.5/OpenMath_0.1.0-pre-alpha.5_ios_simulator_arm64.app.zip) | 23270030 | `46bebca3b227a233d5dbb2b7772b770f3c38ee3216ceb577d097898b0e6b00df` |
| [ios-build-sources.json](https://github.com/He-RT/om-openmath/releases/download/v0.1.0-pre-alpha.5/ios-build-sources.json) | 3981 | `5b93c34092f9988f625d80e9d26e5f0688b26c169cb7eb7538d370633b8b1e76` |
| [ios-release-manifest.json](https://github.com/He-RT/om-openmath/releases/download/v0.1.0-pre-alpha.5/ios-release-manifest.json) | 723 | `1b1e2f4a1c7d8d4de7082810d1f01afe7e881c013988016632e40b780c380f46` |
| [ios-release-sources-index.json](https://github.com/He-RT/om-openmath/releases/download/v0.1.0-pre-alpha.5/ios-release-sources-index.json) | 6207 | `8b44178f785239592603e92cfbd12bb8dd1bd6cc5183d431189ccdafa1b18614` |

使用 `gh release upload` 追加，未使用 `--clobber`。公开后只核对 GitHub API 的 `uploaded` 状态、大小和 digest，六项均与 CI 下载文件吻合；原十个资产的 ID、大小、digest 和原标签目标保持不变。没有重复匿名下载或安装回读。

## 运行覆盖与签名边界

- 完整本地运行提交为 `405a1f05d838bbd95d43e403871537a5888905c4`：iPhone 18 Pro / iOS 27.0 与 iPad Pro 11-inch M5 / iPadOS 27.0 各 34 单元 + 6 UI，40 通过、0 失败、0 跳过。原 53 条数学最大耗时分别 92.771 / 622.252 ms，原每条 1000 ms 门槛未改。
- 文件保存回读、Keychain、二维、参数探索、真实三维球体/相机、旋转与助手草稿已覆盖。助手用本地合成 URLProtocol，走真实 Pi → JavaScriptCore → URLSession → Rust codec → 原生工具六轮，完成读取、预览、写入、运行结果 4、回读及撤销；没有真实模型请求。
- 最终源码身份补丁另在临时 iPhone 上通过 5 项针对性测试，准确输入与 `c38d678d` 一致；[输入文件散列和测试摘要](ios/unicode-guard-results.json)单独记录。UI/渲染路径未改变，没有重复两端完整 UI。
- 以上运行证据来自本地构建候选。最终公开文件来自另一次 CI 构建，`ios-build-sources.json` 如实记为 build-only；不能声称公开 ZIP 完成同一字节的安装运行验收。
- 模拟器应用为 ARM64 ad hoc 签名；真机应用未签名，需要用户自己的账户、签名和描述文件，不能直接安装。没有 Apple Distribution/TestFlight/App Store 或物理设备验收。
- Duo 按真实容器宽度自适应，保持文稿、选择、滚动与检查器状态；没有 Duo 模拟设备/折叠硬件或 27.1 专用 API 验证，不把普通旋转测试等同于折叠验收。

[完整运行与失败记录](ios/README.md)保留首轮签名、性能、界面操作及 CI 失败原因。原始构建日志、失败/成功 `.xcresult`、全部候选与公开包保留在 `target/ios-pre-alpha.5/`，未将失败覆盖成成功。

## 模拟器与缓存收尾

Device Hub 顺序为 `0 → iPhone → 0 → iPad → 0 → 临时 iPhone → 0`，关机设备同样计入，始终最多一台。最后一次移除一度无响应，仅关闭这台已授权设备解除阻塞后，原 Device Hub 移除完成；最终 UI 为 No Simulators、CLI 清单也为 0。本轮真实模型新增请求 0，原调用账本仍 69/80。

只移除本轮 device/simulator 的 `Intermediates.noindex`、`ModuleCache.noindex` 和 `SDKExplicitPrecompiledModules` 六个已停止使用的缓存目录。Data 卷实际可用空间从 69,195,882,496 增至 69,673,799,680 字节，回收 477,917,184 字节（约 456 MiB）。保留 Products、xctestrun、XCFramework、SourcePackages、全部 ZIP 与日志/结果包，没有扩大清理范围。
