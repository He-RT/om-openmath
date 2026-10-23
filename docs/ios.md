# iOS / iPadOS 原生客户端

`.5` 原生 iOS/iPadOS 开发附件已于 2026-10-23 追加到[公开发行页](https://github.com/He-RT/om-openmath/releases/tag/v0.1.0-pre-alpha.5)。产品及打包提交均为 `c38d678d461c9f12bf3e9ea686af192db51a033c`；[补包记录](acceptance/pre-alpha.5/ios-release.md)给出资产散列和构建来源。用户在内容完成后允许验证；已通过 Device Hub 逐台创建、测试、删除，当前没有保留模拟器。[移动补包计划](plan/PRE_ALPHA_5_IOS.md)与[实际验收记录](acceptance/pre-alpha.5/ios/README.md)列出覆盖和未测项。

最低 iOS/iPadOS 27，Apple Silicon 开发环境；一个原生应用同时支持 iPhone 与 iPad。使用 SwiftUI/UIKit/TextKit 2、SwiftMath、原生 Canvas，以及 SceneKit 的 Metal 三维渲染；数学计算复用 Rust 内核。不是 Tauri/WebView 封装。

## `.5` 内容

- 工作台遵循[原生玻璃草稿 02](design/ios-ipados-workbench-02.md)：纸面内容、源码槽、状态左轨、原生悬浮玻璃操作组。iPhone 使用单栏和底部检查器；宽窗口展开文稿与检查器，更宽窗口可打开目录。布局读取容器实际尺寸，不以设备名称或横竖屏重建文稿。
- iPhone Duo 使用同一尺寸自适应路径。源码、选择、检查器和阅读锚点由文稿控制器持有；真实折叠区域、iOS 27.1 纵向系统工具栏与触屏表现仍须在支持的系统上验证，不能把 Mac 草稿的评审画布数值当作设备安全区。
- 数学、文本、Ask 单元格可添加、删除、移动和切换方言。文字默认阅读态，双击编辑；数学编辑保留中文合成保护、本地补全、预览与硬件键盘快捷键。
- 解、成立条件、步骤、变量、结构化值、二维交互图、三维网格/曲线/点/标注及独立参数探索来自内核数据。三维可旋转、平移、缩放、复位与导出真实 OBJ；旧版的“未适配”占位已移除。
- 助手使用固定 Pi 1.0.4 调度，在系统 JavaScriptCore 内解释执行随包资源；Swift 负责网络、文件和工具，Rust 负责共享模型协议。没有 Node 子进程、系统 shell 或远程脚本下载。
- 助手分执行/讨论模式。讨论不能改写或运行主文稿；临时计算复制当前数学定义，不重放笔记本源码。执行修改先预览并核对源码散列，源码写入前记录 pending 事务，再保存与记录结果；支持冲突拒绝、操作查询及撤销。
- 模型仅来自已配置并可用的供应商配置；选择、讨论/执行、图片能力、草稿、对话、操作和事务按文稿持久保存。可新建或回看历史对话，查看本轮完整模型输入和供应商实际报告的用量；未报告的用量不伪造。
- 附件保留原始文件和散列。图片归一化，PDF 提取带页码的文字，UTF-8 文本分页读取。图片能力由用户按模型启用；音视频识别、OCR 和无可提取文字的扫描 PDF 未加入本轮。API 密钥和额外请求参数保存在 Keychain，不写入笔记本/对话/发行包。
- 文件菜单提供新建、打开、保存、另存为、分享、Markdown/LaTeX 导出。普通 `.omnb` v1 只保存源码；重新打开不会自动运行全部单元格。自动保存失败会保留修改状态。

`.5` 使用独立 bundle ID `org.openmath.OpenMath.pre-alpha5`、独立设置和 Keychain 服务，不自动导入旧移动版配置。公开资产不包含个人签名材料或测试凭据。

## 不启动模拟器的开发入口

需要 Xcode **27.0** / SDK 27.0、Rust **1.94.0**、Node 与 `app/package-lock.json` 的依赖。所有生成输出按锁文件和固定源稿构建。

```sh
npm ci --prefix app
bash ios/Scripts/verify-environment.sh
bash ios/Scripts/check-native.sh
bash ios/Scripts/build-candidate.sh
```

`check-native.sh` 在 macOS 原生宿主检查 Pi 调度、源码事务、持久化和几何边界。`build-candidate.sh` 构建 ARM64 真机和 ARM64 模拟器目标，但不创建、boot、install、launch 或运行模拟器测试。应用、模块和中间缓存都在 `target/ios-pre-alpha.5/`；Rust XCFramework 在忽略的 `ios/Frameworks/`。

对测试代码做编译检查，可使用 `xcodebuild ... -destination 'generic/platform=iOS Simulator' ... build-for-testing`；它不等于运行测试或验收 UI。

## 获明确许可后的模拟器验证

本轮已完成验收，不重复启动模拟器。后续本机获得相应许可后，先确认现存数量为 0，再通过 Device Hub 创建并启动一个型号，以其实际 UDID 运行已有编译产物：

```sh
DEVICE_HUB_UDID='填入 Device Hub 中本次唯一设备的 UDID'
xcodebuild -project ios/OpenMath.xcodeproj -scheme OpenMath -configuration Release \
  -destination "platform=iOS Simulator,id=$DEVICE_HUB_UDID" \
  -derivedDataPath target/ios-pre-alpha.5/simulator \
  -resultBundlePath target/ios-pre-alpha.5/device-hub-check.xcresult \
  -parallel-testing-enabled NO -maximum-concurrent-test-simulator-destinations 1 \
  -collect-test-diagnostics never ARCHS=arm64 ONLY_ACTIVE_ARCH=YES \
  CODE_SIGNING_ALLOWED=YES CODE_SIGN_IDENTITY=- test-without-building
```

结果目录每轮使用新名字；测完在 Device Hub 删除设备，再创建下一型号。以下脚本入口供已获许可的 CI 使用，会通过 `simctl` 自行创建/删除设备，不用于本轮要求 Device Hub 管理的本机流程：

```sh
bash ios/Scripts/test-simulator.sh phone
bash ios/Scripts/test-simulator.sh pad
```

脚本拒绝已有任何模拟器（包含关机设备）的环境，每轮创建独立模拟器，保存独立 `.xcresult`，结束后只清理该轮模拟器。测试包括真实 Rust 计算与公式排版、文件、设置、二维、三维、旋转及助手草稿流程；真实外部模型请求不包含在默认用例中。Duo 特定折叠区域和 27.1 行为另记实际覆盖。

## 补包与发布

[release-ios.yml](../.github/workflows/release-ios.yml) 默认只构建；`run_simulators` 和 `publish` 都默认关闭，自动上传分支要求同次模拟器检查成功。本轮已完成本地运行验收，最终 CI 使用固定 `source_sha` 编译并记录来源后手动追加资产，没有再次启动模拟器；不能将 CI 包写成已安装验收的同一字节包。未得到用户许可前，不派发会启动模拟器的旧 `ios.yml` 或打开新工作流的验证选项。

已公开的移动资产为：

- `OpenMath_0.1.0-pre-alpha.5_ios_simulator_arm64.app.zip`：ad hoc 签名的 ARM64 模拟器开发应用。
- `OpenMath_0.1.0-pre-alpha.5_ios_device_arm64_unsigned.app.zip`：未签名真机开发应用，需要使用自己的签名和描述文件；不能直接安装。
- `OpenMathKernel_0.1.0-pre-alpha.5.xcframework.zip`：真机 / 模拟器 ARM64 Rust 库与许可。
- 独立 `ios-release-manifest.json`、`ios-build-sources.json` 与 `ios-release-sources-index.json`，记录完整 Git 输入、工具链、检查范围、大小与 SHA256。

候选源码提交且干净后，用 `package-candidate.sh` 在一个新目录生成资产。公开时只追加新名字，不覆盖原 `.5` 标签、Mac/Web/CLI/Tauri 资产或清单。本次六个新增文件均已公开；没有 TestFlight / App Store 发行。完整构建与本地运行覆盖分别见[补包记录](acceptance/pre-alpha.5/ios-release.md)。

## 本机真机安装

已有源工程可以在 Xcode 以自己的开发者账户编译安装。个人 `DEVELOPMENT_TEAM` 放在未跟踪的 `ios/Local.xcconfig`；连接可信设备后，可运行 `bash ios/Scripts/install-device.sh UDID`。脚本会进行真机签名、安装和启动，应在准备进行该实际操作时使用；本轮尚未执行。

源码工程和包锁文件随仓库提交。C ABI 的返回缓冲只允许释放一次，串行队列与文档代次隔离旧回复；HTTP 原始字节交给 Rust 解码，不把部分工具参数当成完整可执行调用。能力和来源记录均不替代实际设备运行证据。
