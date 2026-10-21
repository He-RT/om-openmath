# OpenMath 0.1.0-pre-alpha.5

2026-10-21 已[公开发布](https://github.com/He-RT/om-openmath/releases/tag/v0.1.0-pre-alpha.5)。源码提交：`86da06206c1ce547921ff2fb6ada892c5b090b48`；原 Mac/Web 三包和补充的两平台 CLI ZIP 均对应此提交，后续文档与打包流程更新不改变产品源码。

本版提供原生 Mac 和 Web 的工作台 v2：目录、数学源码槽与纸面结果、状态左轨，以及助手／步骤／变量检查器。精确解、有条件成立、过期结果和运行错误均有明确文字状态；文字单元格默认阅读，代码块支持横向滚动。

助手支持执行和讨论模式、按消息排列的操作记录、定位、源码差异和撤销。本轮结果来自宿主的修改、运行和检查回执；结束回合不代表任务必然达成。模型管理在设置中，输入框支持附件、单元格引用、模型选择和停止。

## 下载与运行

- **Mac：** 需要 Apple Silicon、macOS 27 或更新版本。打开 DMG，将 **OpenMath Preview.app** 拖到应用程序目录；也可解压 app ZIP。Node、Pi、公式字体和运行资源随包提供，无需安装 Rust、Xcode、Node 或 npm。关于窗口显示应用、内核版本及源码提交。
- **Web：** 解压 Web ZIP，通过 HTTP(S) 静态服务器访问；直接用 `file://` 打开不支持模块和 WASM。设置底部显示应用版本、实际内核版本和源码提交。浏览器直连模型服务需要供应商允许 CORS。
- 三个包及 `release-manifest.json` 同版，清单记录源码提交、文件大小和 SHA256。普通 `.omnb` v1 文件保持源码格式，打开不自动运行。
- **CLI 补包：** `om-cli_0.1.0-pre-alpha.5_macos_arm64.zip`、`om-cli_0.1.0-pre-alpha.5_windows_x64.zip`，解压后在终端运行 `om`／`om.exe`，无需 Node 或图形界面。保留现有终端功能，不新增笔记本助手；大小与 SHA256 单独记录在 `cli-release-manifest.json`，原清单及资产保留。

## 签名与数据

这是实验性 pre-alpha。Mac 包使用本地 ad hoc 签名，尚无 Developer ID 签名或 Apple 公证；首次下载可能被 Gatekeeper 阻止。签名完整性检查不等于通过公证。

原生应用使用本版独立的 Preview 数据目录，保留 `.4` 目录，不自动读取或迁移旧版配置、密钥和会话。模型需要显式配置；Mac 密钥保存在系统钥匙串，Web 密钥按用户选择保存在浏览器本地。密钥和助手会话不写入 `.omnb`。更新 ad hoc 签名候选后，系统可能再次询问钥匙串访问。

## 附件与已知限制

- Mac 支持图片输入，以及 PDFKit 提取的带页码文字；扫描 PDF 没有可提取文字时会明确提示。Web 本版发送图片。其他文件可保留为附件，但不发送给模型。
- 模型的图片理解可能出错。本次真实图片验收中，供应商识别出颜色和棋盘纹，但空间位置描述不完全准确；图片成功传输不证明模型描述正确。
- 图形工具返回实际几何数据；未注册渲染图片时，助手不能声称已经目视检查图形。数值采样与误差估计不等于严格认证。
- 三维透明物体仍使用近似排序。部分系统文件选择面板中曾出现有效 `.omnb` 文件的“打开”按钮不可用，原因尚未确定。
- 若文件面板的“打开”不可用，可在 Finder 右键文件，选择“打开方式 → 其他”，指定本次安装的 OpenMath Preview.app；此路径已在本版 DMG 和 app ZIP 冷启动、保存读回时实际验证。
- 本版不提供新的 Windows 图形安装包或 iOS 包；不含音视频处理、长对话记忆压缩、提示词管理、多窗口、多预设及思考强度完整编辑器、语法高亮。CLI 为独立 ZIP 补包，未增加 Developer ID／Authenticode 发行签名或 Apple 公证。旧版公开 tag 和资产保持原样。

## 验收与发布记录

同一源码提交的 [CI](https://github.com/He-RT/om-openmath/actions/runs/39533915333) 已通过 Rust、frontend、dependencies、native 检查，iOS 按本版范围跳过。本地三个包已安装或解压，实际生成三维西瓜并求得 2、3，Mac 关于窗口与 Web 设置均显示 `.5` 及完整提交。

CLI 补包的[专用构建](https://github.com/He-RT/om-openmath/actions/runs/39539290622)已在 macOS ARM64 和 Windows x64 分别通过实际 ZIP 解压后的版本与求根 2/3 检查。仅补现有终端功能，不加助手；详见[CLI补包记录](acceptance/pre-alpha.5/cli-release.md)。

公开后的匿名下载、哈希复核和重复安装回读按用户 2026-10-21“不用验证了，同步更新readme等文档”的要求省略，未记为实测通过。资产大小、SHA256 与本地验收详见[安装与发布记录](acceptance/pre-alpha.5/local-install.md)；设计、真实助手场景、数学、性能及数据回归见[本版计划](plan/PRE_ALPHA_5.md)。
