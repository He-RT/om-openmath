# pre-alpha.5 iOS / iPadOS 补包

2026-10-23。用户要求开始制作并发布 `.5` 的 iOS / iPadOS 版，先完成内容，只有获得其后续明确许可才能启动模拟器验证。沿用单代理、原生实现、不改系统权限/输入法/外观、按需清理自有构建缓存的约束。此追加范围覆盖旧 `.5` 计划中排除 iOS 的条款；Git 走短期分支 → PR → 适用 CI → dev。

用户已在内容候选完成后允许模拟器验证，追加约束：**通过 Device Hub 管理，同时最多存在一台模拟器（关机也计入），一个型号测完就删除，再创建下一个**。已通过 Xcode 官方组件入口安装 iOS 27.0 运行时，按 0 → iPhone → 0 → iPad → 0 顺序完成，Unicode 修复再按 0 → iPhone → 0 补测；所有测试设备均已从 Device Hub 删除。六个移动资产已于 2026-10-23 追加公开，见[补包记录](../acceptance/pre-alpha.5/ios-release.md)。

## 交付目标

- SwiftUI / UIKit 原生工作台，遵循 [原生玻璃草稿 02](../design/ios-ipados-workbench-02.md)：清晰纸面、浅灰源码槽、状态左轨、悬浮原生玻璃工具组。
- iPhone 单栏与底部检查器；iPad 宽窗口目录 / 文稿 / 检查器，窄窗口回到单栏。Duo 以真实容器尺寸布局，保留同一文稿、编辑、检查器和滚动状态；不把草稿硬编码的安全边缘 / 折叠区域当作硬件数据。
- 复用 `.5` Rust 数学内核，原生数学/Markdown、二维与三维、步骤、变量、文件读写/导出和模型设置；补齐移动助手的执行/讨论、操作记录、撤销、附件和持久草稿。不得将旧只读 CAS 对话称为 `.5` 助手完成。
- 在已有 GitHub `.5` pre-release 追加独立移动资产和来源清单，原标签和已有资产保持不变。默认交付可由用户自行签名的真机开发包、模拟器开发包、XCFramework；不是 App Store / TestFlight 或无需签名的通用安装包。

## 顺序与检查边界

1. 核对并补齐生产内容、工程生成、资源与包内版本。
2. 无模拟器阶段：针对新逻辑的宿主/契约检查、Rust FFI 回归、真机目标的 unsigned 构建，以及模拟器目标的 **build-only**（不得 boot / install / launch / test）。日志进入自有 `target/ios-pre-alpha.5/`；不重复 Mac/Web 冻结验收、不调用真实模型。
3. 内容和候选构建可评审后，请用户明确允许启动模拟器。许可前不派发原 `ios.yml`，该工作流会自动启动模拟器。
4. 获许可后，按实际改动验证 iPhone / iPad、文件保存回读、编辑/键盘、布局与状态、求根/二维/三维及助手回环；真实模型如需调用另核对既有预算与范围。记录实际覆盖，未测项不记为通过。
5. 通过后按 Git 管理规则集成并发布新增资产，更新安装文档、README 和独立补包记录。公开后不覆盖已有同名资产。

## 当前进度

- 初始基线 `41a49068f33e78e12bee002d35d2d190ccce853f`，实现分支 `codex/ios-pre-alpha-5`。Xcode / iPhoneOS SDK 27.0、Rust 1.94.0；两个 iOS ARM64 目标已安装。最终产品及打包提交为 `c38d678d461c9f12bf3e9ea686af192db51a033c`。
- [x] 原生工作台内容：纸面、源码槽、状态轨、原生玻璃工具组、宽度布局与控制器持有的编辑/阅读状态。
- [x] 原生三维实现：Rust 真实采样、SceneKit Metal 显示、相机交互与 OBJ 导出；几何边界检查及两设备三维/旋转 UI 验收通过。
- [x] 移动 `.5` 助手实现：Pi 1.0.4 + JavaScriptCore、共享模型 codec、执行/讨论、源码事务/撤销、附件和持久状态。停止后等待旧任务退场；保留同轮中已完成工具的消息配对，不丢弃整条提案。
- [x] 工程 / 资源 / 包内版本与开发包来源记录：独立 `.5` bundle ID 和存储、生成资源一致性、build-only/打包脚本及默认不开模拟器的补包工作流。最终公开包、manifest、source sidecar 和索引已生成并核对。
- [x] 内容候选 `5c5c473e0076cf622404054a54d44b10fa4e8b09` 的真机 Release / 模拟器 build-for-testing 与三包归档，均为 ARM64；该阶段只有构建证据，后续运行修复与最终公开来源分别记录。
- [x] 用户允许后的 iPhone/iPad 验证：`405a1f05d838bbd95d43e403871537a5888905c4`，各 40 项通过、零失败/跳过；两设备均删除。
- [x] PR #7 / #8 通过适用 CI 后普通合入 `dev`；最终固定输入构建成功，六个移动文件已追加到原 `.5` Release，README/安装/模型/发布文档同步。

## 内容候选阶段的检查记录（获许可前）

已完成的无模拟器检查：

- Rust FFI/能力/三维相关 26 项通过；`target/ios-pre-alpha.5/native-math-tests-02.log`。其后 Rust 仅格式调整，当前 `cargo fmt --all -- --check` 通过。
- 准确提交候选的真机 ARM64 unsigned Release 与模拟器 `build-for-testing` 均通过；`candidate-5c5c473e-build.log`。模拟器一项只编译应用与测试目标，不运行 XCTest/UI。
- `check-native.sh` 通过；`host-native-current.log`。使用真实 macOS JavaScriptCore/Pi 调度，覆盖两轮、真实工具调用与持久配对；纯 Swift 覆盖源码冲突拒绝、事务/草稿/附件持久化、部分停止轮次消息配对、运行依赖与非法几何。没有网络请求。
- 全部 31 项仓库契约检查通过（含发行 8 项、iOS 静态契约 2 项及来源清单关系）；`contracts-current.log`。生成资源一致性、工程生成、actionlint、脚本语法、Git 忽略边界和 diff 空白检查通过。

候选在 `target/ios-pre-alpha.5/candidate-5c5c473e-assets/`，独立 manifest 记录三个包的大小和 SHA256；真机应用约 18.1 MB、模拟器应用约 23.5 MB、XCFramework 约 90.8 MB。已核对两份 ZIP 内 `.5` 版本、完整 revision、独立 bundle ID、iPhone/iPad 声明及 Pi 资源；没有安装应用。`candidate-5c5c473e.json` 记录本地构建范围。后续进度文档的提交不改变此候选的输入。

该阶段推送三个实现提交并创建 [草稿 PR #7](https://github.com/He-RT/om-openmath/pull/7)，目标为 `dev`；当时常规 CI 正在运行，iOS 运行项按许可边界跳过，尚未合入或公开移动资产。

当时尚未执行 iPhone/iPad 的 XCTest/UI、安装回读、硬件键盘/触控、性能、真实模型和 Duo 折叠验证，未创建或运行模拟器；后续获许可的结果见下节。27.1 专用折叠区域/纵向工具栏 API 不在本机 27.0 SDK 内，不能记为 Duo 设备验收。没有 Apple Distribution 签名身份，真机资产为需自行签名的开发应用，不是通用 IPA / TestFlight / App Store 包。

## 获许可后的运行修复（以上为许可前记录）

- 首轮 iPhone 33 项单元测试有 3 项失败，6 项 UI 中三维/助手流程失败；失败根 `phone-5c5c473e.xcresult` 保留。设置失败来自模拟器候选仅有 linker 签名，补 Xcode ad hoc 应用签名后两项设置测试通过。53 数学条目中 row 17 首次 1264 ms（FFI 127 ms / 主线程恢复 1132 ms），复测在原 1000 ms 阈值内通过，不放宽阈值。
- 三维相机接管内部拖动，测试改为从文稿边缘滚动；补底部工具栏滚动留白、44pt 三维操作触点和宽屏检查器 ID。`phone-scene-gutter-02.xcresult` 实测球体 3968 三角形、信息、旋转和助手草稿保留通过。
- `phone-mobile-loopback-02.xcresult` 完成 6 轮真实 Pi/JavaScriptCore → URLSession 合成协议 → Rust codec → 原生工具回环，验证预览、源码保存、计算/结果 4、回读和撤销；没有外网/真实模型调用。修正快照编码为稳定字段顺序，避免分页引用随机改变。
- 常规 CI 暴露旧 Web 的 iOS 三维占位断言、FFI enum 大小 lint，以及 `ctutils 0.4.2` 新漏洞 RUSTSEC-2026-0385；更新真实几何断言、Box 请求，锁文件仅升级到修复版 0.4.4。相关 Clippy、4 项桥接测试、Web 类型和 advisories 检查通过，最终 SHA CI 待重跑。
- `405a1f05` 的双目标候选已构建并归档。iPhone/iPad 各 34 项单元 + 6 项 UI 全通过；原 53 条数学最大耗时分别 92.771 / 622.252 ms。iPad 安装程序与候选 ZIP 的 executable / Pi 资源一致。设备数量最终为 0。
- [精简验收记录、原始数值和选定截图](../acceptance/pre-alpha.5/ios/README.md)保留失败及覆盖边界。旧候选包和失败根保留，不发布；常规 CI、集成、公开补包及文档同步继续推进。
- 主实现 `bba0448e` 的完整 CI `39934092109` 已通过，PR #7 合入 `dev` 为 `e060c4ad`。随后 NFC/NFD 源码字节保护补丁在单台临时 iPhone 上通过 5 项针对性测试，设备再次删除，数量为 0；补丁另走 PR/适用 CI，不重复未变化的 UI/渲染验收。最终发行使用包含补丁的提交。

## 最终公开状态

- Unicode 修复 `c38d678d461c9f12bf3e9ea686af192db51a033c` 的适用 CI [39937454919](https://github.com/He-RT/om-openmath/actions/runs/39937454919) 通过；[PR #8](https://github.com/He-RT/om-openmath/pull/8) 普通合入 `dev` 为 `343bf846c7e7797fe0768c59e11cf84c8055f84c`。
- [发行构建 39937450343](https://github.com/He-RT/om-openmath/actions/runs/39937450343) 的产品和打包输入均固定为 `c38d678d`，两个 ARM64 目标编译、资源一致性、归档与来源校验通过；`run_simulators=false`、`publish=false`，没有再次启动模拟器。
- 下载该构建的六个文件，核对 ZIP 完整性、包内版本/完整 revision/bundle ID/设备声明/助手资源及来源索引后，手动追加公开。GitHub API 的大小、SHA256 与本地一致；原十资产 ID/大小/散列和原标签均未变化。没有把该 CI 包记为已完成相同字节安装验收。
- 仅清理自有且已停止使用的六个构建中间缓存目录，Data 卷实际回收约 456 MiB，约 65 GiB 可用；保留 Products、测试产物、XCFramework、依赖、全部包和失败/成功记录。真实模型请求新增 0，模拟器最终数量 0。
