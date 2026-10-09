# 原生 Mac 安装、首次启动与发行契约

[原生客户端](macos-native-ui.md) · [宿主](macos-host-state.md) · [存储](macos-storage-recovery.md) · [模型与媒体](model-media.md) · [机器 Schema](macos-distribution.schema.json) · [启动/更新草案](prototypes/mac-installation.html)

2026-10-09。下一版 Mac 设计，**尚未实现**。用户明确不需要旧版数据迁移：采用全新原生数据目录和重新配置供应商，不实现 `.1/.2/.3` 的配置、凭据、聊天、草稿或缓存自动导入。原生首版定位 Apple Silicon、macOS 27.0+；包内自带 Rust 内核和 Pi/Node，安装与基本数学计算无需开发工具、npm 或模型密钥。

2026-10-09目标版号已收敛为[`.4`版本与门禁](../plan/PRE_ALPHA_4.md)，运行版本/数字build在实施冻结；本文件package/release正例仍仅为契约演示，不是实际包。继续`dev`，不移动公开`.1/.2/.3`标签、不合并`main`；当前公开包仍Tauri`.3`。其他平台保持既有UI/数学门禁，不重写其安装器，也不启动本机iOS模拟器。

## 范围裁决与环境依据

- 新原生客户端首次启动不扫描旧数据路径、不读取旧 service `openmath` 的 Keychain 项、不复制旧 TOML/环境密钥、不自动清理旧数据。供应商由用户重新添加；界面默认无已配置账户或已授权 AI 路由。
- `.omnb` v1 源码读取兼容保留：用户主动打开文件是普通文件功能，不需要一个数据迁移向导。解析/打开不自动执行，输出和 Agent 会话不随源码文件加载。
- **新原生版本以后的格式升级、备份和回退仍需保留**。这与导入旧 Tauri 数据是两件事，不能因为本次全新安装就以后把不认识的数据库清空。
- 本轮只读复核：本机 `arm64`、macOS `27.0`、Xcode `27.0`；有效本地签名身份统计 Developer ID Application 为0，Apple Development 为1。未导出证书/私钥，未尝试公证或替换应用。这不证明安装门禁已经通过。
- 首版最低 macOS 27.0 是本设计裁决，必须在实际 Info.plist/deployment target/包说明与最低系统机器验收一致；SwiftMath 或 TextKit 的较低 API 可用版本不自动成为应用兼容承诺。首版不发布 Intel/Universal 包，不把 Rosetta 当 ARM64原生运行通过。

## 应用身份、通道和数据目录

| 通道 | 应用/Bundle ID | 数据/钥匙串 | 分发行为 |
|---|---|---|---|
| 原生正式/预发行 | `OpenMath.app` / `org.openmath.OpenMath` | 系统 Application Support 下 `OpenMath/NativeMac`；Keychain service `org.openmath.native.release.provider` | 新原生公开包沿产品安装位置替换旧 app；不读取旧 Tauri 数据 |
| 原生开发预览 | `OpenMath Preview.app` / `org.openmath.OpenMath.Preview` | `OpenMath/NativeMacPreview`；service `org.openmath.native.preview.provider` | 可与公开 `.3` 并存；不覆盖用户安装，不成为 `.omnb` 默认处理器 |

通道由构建常量决定，启动参数/模型文本不能切到另一通道数据。store root、Caches/Logs/备份/恢复与凭据均保持通道隔离；运行时可读系统 `Bundle` URL，不能从 cwd 推导包资源。文件外部 `.omnb` 是用户自选 URL，不写进 `.app` 或作为 preview/public 共享业务数据库。

新Mac Session由外部宿主注入空/已保存新Registry和URLSession凭据端口，不能默认走旧NativeConfig路径加载TOML、环境Key或桌面keyring；现有底层KernelConfig的默认DeepSeek profile不能自动变成新UI的已配置账户。模板可以有非秘密连接建议，但纳入/测试/授权仍由真实新配置入口完成。

公开 app 延续产品 Bundle ID 便于 Finder/文档关联，**不因此复用旧 Keychain service**；新的 provider_id/revision 是独立身份。开发/Preview 同 Mac 用户仍不是操作系统上的不同安全主体，不宣称仅换数据目录就形成 sandbox。

公开 `.omnb` UTI沿用 `org.openmath.notebook`、声明 v1 source 文档与系统 NSDocument/打开面板。Preview 不抢默认关联，用户明确“打开方式”选择仍可使用；打开到错误通道时 title/about显示实际 Preview标识。重复实例前置已有窗口，锁按已选通道处理，不开第二个Pi改同库。

新原生正式包可以替换 `/Applications/OpenMath.app`；开发预览用不同文件名。普通安装支持 `/Applications` 或 `~/Applications`，Finder按实际目录权限处理；没有自装的管理员 daemon、LaunchAgent、后台自动开机进程或特权安装脚本。移除app保留用户文档与新业务数据，清缓存/删除会话/删除钥匙串由设置中的明确作用域操作，不因卸载app自动清盘。

## 自包含包结构

```text
OpenMath.app/
  Contents/
    Info.plist
    MacOS/OpenMath                         Swift原生宿主，静态链接Rust服务/FFI
    Helpers/OpenMathAgentRuntime.app/
      Contents/
        Info.plist                        隐藏辅助进程身份，非独立用户应用
        MacOS/OpenMathAgentRuntime          随包Node ARM64可执行文件
        Resources/agent-runtime/
          main.mjs                        固定OpenMath Pi适配器
          runtime modules/resources        被实际依赖引用的生产闭包
          dependency-lock.json            来自实施时锁文件的内容/许可证据
    Resources/
      RuntimeManifest.json                签名前生成的内层组件验证信息
      fonts/                              固定数学字体与原标签字体
      locales/                            中文/英文消息与真实步骤标题
      examples/                           源码示例，打开后不自动执行
      licenses/                           Rust/Pi/Node/Swift/字体完整许可
      PrivacyInfo.xcprivacy               实际适用的隐私声明
```

静态链接为首选，若实际需要 SQLite/Swift runtime 等动态组件，放在标准 Frameworks/嵌套位置并逐项核验 rpath/签名/架构，不能依赖开发机 Homebrew 路径或偶然已安装的 dylib。Mach-O `LC_BUILD_VERSION`、动态链接、字体/本地化缺失均属于包检查门禁。

Pi helper采用私有 `Process`+stdin/stdout IPC，按 Bundle绝对路径启动，不使用系统PATH的node/npm，不开公开监听端口。helper的代码身份/协议/版本/握手校验通过才能给任务上下文；模型HTTP和Keychain仍由Swift宿主处理。cwd为专用空临时/运行目录，环境是受控集合，去掉NODE_OPTIONS/NODE_PATH和供应商密钥等不需要的继承变量；不得将代理凭据/Keychain值塞进命令行。

首版为直接下载的原生应用，不启用 App Sandbox，不使用macOS私有sandbox配置；Hardened Runtime是运行时签名保护，**不是限制Node读取所有用户文件的OS隔离**。不安装Pi Coding Agent默认shell/任意文件工具/动态插件；工具权限通过可信宿主契约校验。本层不给“全磁盘访问”开关，不宣传 helper有超出实际机制的隔离保证。

## Node/Pi 与依赖供应

既定候选 Pi Core/AI `1.0.4`（源码 `503c605528f9af993c0e37ede468cf884fb0ff5b`，MIT，Node≥22.19.0）保持；实施必须创建专用agent lockfile，固定传递依赖而非只固定top-level package。仓库当前开发`.nvmrc`为26.0.0，本次不修改它。

2026-10-09核对的运行时候选为官方Node **26.11.1 darwin-arm64**，官方tar.xz SHA256：

```text
0ef9b443000681bffc062ec126d53f97933ea9b086ea73dcba0480d87e639b39
node-v26.11.1-darwin-arm64.tar.xz
```

这是候选锁定，不是已经随包交付。依据[官方发行索引](https://nodejs.org/download/release/index.json)、[官方校验清单](https://nodejs.org/download/release/v26.11.1/SHASUMS256.txt)、[Pi固定包声明](https://github.com/badlogic/pi-mono/blob/503c605528f9af993c0e37ede468cf884fb0ff5b/packages/agent/package.json)。实施时验证上游发布校验/签名与精确版本、记录Node和其内置OpenSSL等第三方许可；若换版本必须更新记录和同套功能/签名门禁，不能在发行CI下载“latest”。

AgentAdapter编译为Node实际可执行的ESM生产闭包，不在用户机转译TS、运行npm install或下载依赖。保留动态 import/模板文件/nativeaddon真实需要的资源，未审计传递native Mach-O拒绝直接发布；所有实际native组件签名同Team。npm/开发编译器/测试fixture/源码map非必要不随用户包分发，不用“bundle一个main.mjs”假装其动态资源已经齐全。

helper冷启动失败：给准确“助手不可用/查看诊断/重启助手”，手工笔记本与CAS继续可用；不得偷偷回退系统node或网上下载安装。只有用户启动Agent或显式配置中需要模型服务时再加载helper，打开笔记本不启动收费请求；配置的供应商仍不是helper直接网络调用。

## 签名、公证和包完整性

公开原生分发目标为 **Developer ID Application签名 + Hardened Runtime + secure timestamp + Apple公证/stapled ticket**。App Store/TestFlight、自动App Store更新和PKG安装器不在此轮。Apple Development/本地ad-hoc只用于受控开发预览，不能当作公开Developer ID门禁通过。[Apple公证流程](https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution)、[Hardened Runtime](https://developer.apple.com/documentation/security/hardened-runtime)

签名顺序由实际嵌套结构自内向外：nativeaddons/dylibs → Node executable/helper → main app；不靠 `codesign --deep`盲签所有内容，deep/strict只用于校验。helper若V8的实际运行路径需要，单独配置 `com.apple.security.cs.allow-jit`，主宿主不继承该豁免；默认不关闭library validation、不开放unsigned executable memory/get-task-allow，任何额外需求须有固定runtime实测证据。[Apple JIT entitlement](https://developer.apple.com/documentation/bundleresources/entitlements/com.apple.security.cs.allow-jit)

需要的权限/entitlements以实际实现确定；网络来自URLSession、文件来自用户选择/普通权限，OCR/媒体预览不要求屏幕录制或辅助功能控制。使用本地Speech时在明确功能入口触发系统授权，不在首次启动批量请求麦克风/相册/通知/全磁盘权限。provider secret在Swift Keychain以实际Team/代码身份和service访问；换签名Team的后果不能用“Bundle ID相同”掩盖。

RuntimeManifest记录内层已签名helper/native组件及agent/font/resource的版本、原上游archiveHash、打包后componentHash、source SHA、ABI/IPC/描述版本/许可证。生成它时内层签名已经固定；**不把最终外层main executable/.app签名的hash写进它自己封存的资源**，避免自引用和签名后改文件。最终整个ZIP/DMG散列放在包外release-manifest中，package manifest与外部release manifest职责不同。

本机没有Developer ID Application材料；本轮只记录这一真实前置，未创建签名身份或触发Apple服务。缺材料时开发可继续，产物明确developer_preview，不称已公证公开原生包；公开门禁不得自动降为unsigned通过。证书/私钥/notary凭据在受控CI密钥管理，包/仓库/日志不含个人签名材料。

## 构建与发行顺序

1. 锁版本、完整提交SHA、递增build number、通道与架构。Xcode27.0及macOS27 SDK缺失失败，Rust1.94.0/锁文件与SwiftPM/agent/Node闭包均核对；不是继续跑现有Tauri macOS job就算新原生包。
2. ARM64 Release构建Rust host/FFI、Swift原生app和PiAdapter；Swift/Rust/字体许可与所有实际运行资源到位。`CFBundleShortVersionString`为数字基础版本，`CFBundleVersion`为独立单调数字build，完整pre-alpha版本在`OpenMathReleaseVersion`/About与发行资产；不把完整SemVer硬塞进数字bundle version。[Apple build version](https://developer.apple.com/documentation/bundleresources/information-property-list/cfbundleversion)、[最低系统字段](https://developer.apple.com/documentation/bundleresources/information-property-list/lsminimumsystemversion)
3. 组装app、检查无开发机绝对路径/密钥/未跟踪Local配置/调试端口；核对架构、动态库、resources、层内许可与元数据。
4. 在拥有发行材料的流程签名所有内层，再生成RuntimeManifest，最后签名main app。验证签名/Team/entitlements和真实helper启动/IPC/取消/退出，主进程不阻塞等待。
5. 将已签app的临时ZIP提交 `notarytool`，Accepted才staple app并验证ticket、strict signature/Gatekeeper；签名/公证失败保留诊断，不改变release状态为成功。
6. 从最终stapled app生成 `.app.zip`与DMG；DMG含OpenMath.app、Applications快捷链接与简短中文安装说明。DMG签名/提交公证/staple/校验完成后才计算最终字节/hash。ZIP里的app与DMG中的app必须对应相同已封存版本；不能在哈希后staple、换icon或重写plist。
7. 新用户/干净机实际下载带quarantine资产，Finder挂载/复制/启动、首启/离线本地求解、Pi打包闭包、配置持久化与模型fixture、Unicode/渲染/2D/Metal/导出/停止/重启按下文门禁验证。spctl成功不是AppKitUI/Agent成功。
8. 其他平台与原53数学/时限/Swift/WASM门禁继续同SHA通过；九资产类型不缩水，Mac两个资产内部变为新原生客户端。版本/提交统一，CLI仍独立交付；没单独改CLI就不能让app随包的内部Node充当CLI。
9. release-manifest记录最终bytes/SHA256、架构/minOS、native implementation标识、signing/notary evidence引用；公开下载再逐资产核对。完整draft再pre-release，失败不移动已公开tag或合并main。

拟新增`macos/native-package/verify`脚本、macos/工程/Xcode scheme与native CI job均是planned，当前没有修改`.3`的workflow或附件规则。原九类资产是否在最终版本继续全部由同版产出沿已接受发行范围执行，资产名称/新版本号在实施锁定；不发布一份“原生”标签却仍装旧Tauri主窗口。

构建要求从固定源码/工具链/依赖可重建；secure timestamp/签名/公证ticket可能使另一次构建字节不同，不承诺重签名后bit-for-bit相同。发行清单核对的是该次最终公开包的真实字节，不能把后来的重建散列替换为公开附件散列。

## 安装与首次启动 UX

首次安装流程为：下载对应ARM64 DMG → Finder打开 → 拖到应用程序 → 启动OpenMath。ZIP作为相同app的备用分发；不要求用户跑`xattr -dr`、关Gatekeeper或安装开发者工具。开发预览的未公证说明单独列，不让公开安装指南要求关系统保护。[当前已发行安装指南](../install.md)仍如实说明`.3`未签名，不能用本设计提前改成已公证。

若从只读DMG启动，先显示“请将OpenMath复制到应用程序”，提供实际Finder/磁盘位置入口，不打开业务数据库/Pi任务或静默修改运行中app。从用户可写固定目录/`~/Applications`也可正常启动；判断依据实际卷/Bundle位置与签名状态，不根据路径里有“Downloads”就拒绝用户。预览/非标准位置显示实际通道，about可定位包。

首次启动由原生Host/StorageBootstrap产生阶段：检查包/ABI → 建立新通道根/锁/首代库 → KernelReady → 窗口就绪。既存新NativeMac数据不按“首次启动”重置，schema未知/损坏走恢复；旧Tauri目录完全不参与判定。无可靠总量用ProgressView和文字，UI动画结束不标bootstrap成功。

```text
OpenMath
本地数学笔记本

新建、求解与绘图可直接使用。
助手可在之后连接你的模型服务。

[ 开始本地计算 ]   [ 配置助手 ]
打开笔记本…       查看示例
```

这不是强制多步账号注册/迁移向导。基本UI可用即进入新笔记本/空态，欢迎面板可关闭，后续从帮助/设置再打开。样例只加载源码，不自动运行全部计算；配置助手定位供应商页（无旧账户），密钥和探测沿既定Test/Save分离。AI没配置时右栏解释“连接模型后使用助手”，手工笔记本不受影响。

数据根不可写/磁盘满/锁被占用分别显示原因与实际恢复动作；不能将空数据视为保存成功。临时草稿/隔离试算可在明确“未保存”状态使用，Agent写入在存储契约准备好前不开放。Kernel/ABI失败给启动错误/诊断与重新安装，不能用UI空窗口冒充已就绪；Pi单独失败只影响助手，窗口与数学继续可用。

权限按实际用户动作触发：打开/另存为→系统文件面板；粘贴/拖入→主动选择资源；Speech→所选处理服务权限。初次启动没有供应商网络、后台付费探测或远程图片下载。欢迎页不放“全权限”“浏览器附加”“导入旧版”假入口。

## 更新、回退与关于界面

首版采用 **原生检查更新 + 用户手动安装完整包**，不引入Sparkle、不在退出时自改签名bundle，也不让模型自行升级app/helper。默认手动检查；可显式开“启动后每天检查一次”，无变化静默，网络失败不影响数学或反复弹窗。更新请求仅版本/通道/架构与必要客户端标识，不附文档、媒体、会话或密钥。

UpdateService只读受控官方GitHub Releases/发行清单（预发行不可用GitHub的stable-only latest来判断），筛选draft=false、所选通道、可解析SemVer/build、ARM64/minOS和完整资产；超限/结构错/网络/权限失败显示未知，不称已是最新。按照明确选项包含pre-release，Preview默认不覆盖public选择。

关于窗口用标准Form/LabeledContent，显示完整app版本/build/source SHA、实际通道、系统/架构、Rust/ABI/IPC与Pi/Node版本、包完整性/签名状态、许可、诊断和打开下载页。不展示个人Team名称/Keychain值；Developer ID缺失显示实际developer preview身份，不能由打包json的`verified:true`独自证明签名。

更新可用时显示目标版本、兼容性/说明和“打开下载页”，必要时选择实际DMG/ZIP路径；下载由系统浏览器/Finder进行，应用不执行任意URL/新代码。没有自动安装，故不展示下载进度百分比或“已安装”状态。用户装完重启后通过实际Bundle版本/build、ABI与数据格式复核才提示“更新完成”；打开GitHub不改变当前版本。

手动替换app前结束/核对文档与Agent任务、保存源码/草稿并关闭应用；同版本正式包再次安装是app替换，不重置NativeMac数据。回退app到原`.3`时旧Tauri程序只用它自己的原路径，新NativeMac数据仍保留；不导回新聊天/模型配置。新原生后续版本回退到不支持的库格式只读拒写，按备份/new-document路径恢复，不删库“兼容”。

首次新原生发行Root/selector和Library格式为1，新文档库为store_version/minimum_reader_version/codec_version=2（R4.1.07的实际裁减记录需要独立读取规则）；旧Native文档1继续完整记录、不自动裁减或原地升级。实际契约见[源码历史与文本撤销](source-history-retention.md)。以后若必要schema upgrade，先一致备份/新代次验证/原子selector发布，保留逆向不支持提示。这是未来新数据安全升级，不是本轮旧版导入范围。开发Preview/正式通道之间的业务数据自动合并亦不交付。

## 诊断与安装故障

诊断导出默认只含版本/架构/minOS/包组件结果/ABI/操作错误类别与经过脱敏的进程退出状态；不复制源码、原始模型响应、provider头、数据库、附件或Keychain。完整诊断需用户明确选择其范围，导出预览给出文件列表，不能把随包RuntimeManifest当源码/性能成功证据。

| 现象 | 判定与恢复 |
|---|---|
| macOS低于27/Intel | 显示真实兼容性与旧版下载入口，安装器/launcher拒绝伪启动；不改系统版本或自动Rosetta通过 |
| Finder/Gatekeeper拒绝公开包 | 确认最终包签名/公证/隔离下载路径证据，重新从官方资产取得；不用“关闭保护”掩盖门禁失败 |
| 包缺Node/字体/JS资源或seal不符 | 组件错误/重新安装；不从系统PATH或网络补资源 |
| Pi启动/握手失败 | 助手不可用、查看脱敏诊断/重启助手，数学仍可用 |
| 当前库不可写/unknown version | 保留原件与草稿、存储恢复入口，不空库初始化或自动重放 |
| 模型未配置/超时 | 设置供应商/实际探测，安装成功不等于模型服务已成功 |
| 更新查询失败/目标不兼容 | 未知/不兼容与实际原因，保留当前app，不当“已是最新” |

## 实施顺序与门禁

| 阶段 | 交付 | 必须验证 |
|---|---|---|
| I0，N0前置 | 通道身份/新根/bootstrap、macOS27ARM64工程与版本规则 | 当前无旧数据读取，预览并存不抢文档关联，clean machine无开发环境启动 |
| I1，N2前置 | 固定Node/Pi生产闭包、helper身份与IPC | 不依赖PATH/npm、无包外绝对依赖、资源/协议/取消/关闭真实可用 |
| I2，N4 | 签名、公证、DMG/ZIP、RuntimeManifest与外部release manifest | inside-out、JIT仅实际需要helper、staple后最终散列、隔离下载/离线ticket/真实启动 |
| I3，N3/N4 | 欢迎/空态/About/更新与诊断 | 无强制AI设置或迁移，查询真实失败/通道/兼容/手工安装不假成功 |
| I4，发行 | 同SHA九资产/原数学和跨端门禁、公开下载回读 | 真实native安装/恢复/许可与全部已有门禁，不发布未通过/未签名假原生包 |

全部运行时验收 **planned**：IN01 首次安装/只读DMG/~/Applications/路径有空格/新用户与无网络数学；IN02 Preview/public并存/BundleID/新根/Keychain service与零旧目录/旧凭据访问；IN03 无系统node/npm/rust/xcode依赖，helper生产闭包/nativeaddon缺资源/错误架构/IPC/单任务/退出；IN04 数字build/完整SemVer/当前源码/最小系统与真实arm64/Mach-O/rpath；IN05 nested签名/Team/JIT与notary失败/Accepted/stapled ticket/最终ZIPDMGbyteshash；IN06 文件关联/主动`.omnb`打开/不自动执行/Unicode/原53/2D/完整西瓜Metal/导出；IN07 Keychain新配置Test不保存/真实凭据读取/锁屏/权限拒绝；IN08 存储满/锁/首次初始化中断/新库恢复/未知格式拒写；IN09 optional欢迎/无账号local-first/助手未配置/失败/重新启动；IN10 update网络失败/预发行/通道/版本/资产/不兼容且打开下载页不标已安装；IN11 手动替换/更晚草稿/任务停止/新数据备份/旧`.3`回退互不覆盖；IN12 脱敏诊断/包许可/新版本同SHA全部平台及公开资产回读。

## UI依据与本轮边界

欢迎/空态/更新/About用[telegram-ui-reference SKILL.md](/Users/hert/.agents/skills/telegram-ui-reference/SKILL.md)的[空状态/权限](/Users/hert/Documents/ChatGPT/ui-learning/06-state-and-feedback/empty-states-and-permission-prompts.md)、[进度与回执](/Users/hert/Documents/ChatGPT/ui-learning/06-state-and-feedback/progress-and-result-feedback.md)、[设置页](/Users/hert/Documents/ChatGPT/ui-learning/04-pages-and-flows/settings-pages.md)基础契约：来源决定阶段、无回执不成功、失败与未配置不同、只有明确动作触发权限。使用NSWindow/standardButton/Form/ProgressView/Menu，复用原40组件族，Apple标准设计比例仍75%、原生技术目标100%；HTML只作评审，不计原生实现。

本轮只形成安装/发行设计、机器契约和交互草案，修正旧数据迁移待办；没有导入/删除任何旧数据，没有下载/安装候选Node/Pi、签名/公证、改当前安装或创建Release。关于现有证书和系统的检查是只读事实，IN01–IN12仍待真实原生包实施。

实际设计检查：9个Schema合成正例/26反例、18项HTML启动/空态/更新/失败/回调/主题/窄窗检查通过，已有5份Schema及组件计数保持有效；详情见[评审记录](prototypes/installation-review.json)和[截图说明](prototypes/README.md#原生安装首次启动与更新草案)。这些检查不能作为IN01–IN12真实包/签名/运行时通过证据。
