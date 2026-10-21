# OpenMath

[English](README.en.md) · [下载安装包](https://github.com/He-RT/om-openmath/releases) · [使用文档](docs/README.md)

OpenMath 是以方程求解为核心的开源计算机代数系统。终端、响应式浏览器笔记本、桌面应用和原生 iOS/iPadOS 客户端共享 Rust 内核，支持现代数学写法与 Wolfram 语言子集、精确解卡片、推导步骤、交互绘图和可配置 AI。

已发布 [**0.1.0-pre-alpha.5**](https://github.com/He-RT/om-openmath/releases/tag/v0.1.0-pre-alpha.5)：原生 Mac 与 Web 工作台 v2，提供目录、纸面结果、助手／步骤／变量检查器，以及支持执行、讨论、操作记录和撤销的笔记本助手。本版发行 Mac DMG、app ZIP 和 Web ZIP；旧版公开资产继续保留。支持有资源边界的求解子集；无法支持的问题保留原表达式并给出诊断。兼容范围见[求解指南](docs/solve.md)。

```sh
om -e 'solve(x^2 - 5x + 6 = 0, x)'
# x = 2  │  x = 3
om -e 'Solve[x^2-5x+6==0,x]'
om --json -e 'solve(x^2=2,x)'
```

AI 提议源码，由 CAS 解析、求解和验证。执行模式可通过工具修改和运行笔记本，讨论模式只读并可临时计算；操作记录和本轮结果反映宿主实际回执。密钥和对话不进入笔记本文件。

![Mac 工作台 v2：原型与实机验收对照](docs/acceptance/pre-alpha.5/g1-mac-light.png)

Mac 工作台 v2 的原型与实机验收对照；[Web 桌面](docs/acceptance/pre-alpha.5/g1-web-desktop.png)、[Web 手机](docs/acceptance/pre-alpha.5/g1-web-phone.png)及[逐项验收记录](docs/plan/PRE_ALPHA_5.md#6-进度表)可查。

## 下载与安装

在 [pre-alpha.5 发行页](https://github.com/He-RT/om-openmath/releases/tag/v0.1.0-pre-alpha.5) 下载本版 Mac/Web；Windows、独立 CLI 与 iOS 附件仍使用 [pre-alpha.3](https://github.com/He-RT/om-openmath/releases/tag/v0.1.0-pre-alpha.3)：

| 平台 | 文件 | 使用方式 |
|---|---|---|
| Windows 10/11 x64 | `OpenMath_0.1.0-pre-alpha.3_x64-setup.exe` | 推荐；中文向导，安装到当前用户目录 |
| Windows 10/11 x64 | `OpenMath_0.1.0-pre-alpha.3_x64_zh-CN.msi` | MSI 安装包，适合系统管理员部署 |
| macOS 27+ Apple Silicon | `OpenMath_0.1.0-pre-alpha.5_aarch64.dmg` | 打开后将 OpenMath Preview.app 拖入“应用程序” |
| macOS 27+ Apple Silicon | `OpenMath_0.1.0-pre-alpha.5_macos_arm64.app.zip` | 解压得到 OpenMath Preview.app |
| Windows / macOS | `om-cli_…_windows_x64.zip` / `om-cli_…_macos_arm64.zip` | 解压后运行 `om.exe` / `om`，可加入 PATH |
| iOS/iPadOS 27 | 原生 Xcode 工程、ARM64 模拟器应用及 XCFramework | 真机通过本机自动签名安装，见[移动端指南](docs/ios.md) |
| 浏览器 | `OpenMath-web_0.1.0-pre-alpha.5.zip` | 解压到 HTTP(S) 站点根目录 |

本版 Mac 为自包含原生应用，无需另装 Node、Rust 或 Xcode；使用 ad hoc 签名，尚无 Developer ID 签名或 Apple 公证。旧版 Windows 包未签名，携带 WebView2 引导程序；系统没有 WebView2 时，首次安装需要联网下载运行时。当前不提供 Intel Mac、Windows ARM64 或 Linux 安装包。详细步骤、卸载及校验见[安装指南](docs/install.md)。

每个发行版提供 `release-manifest.json`，记录同一提交构建的文件大小和 SHA256。全部分发包保留相关第三方许可文本。

## 开始使用

打开[示例笔记本](docs/examples/README.md)，或添加数学单元格输入 `solve(x^2 - 5*x + 6 = 0, x)`，运行后得到 2、3。分别运行 `let a=2` 与 `a+1`，再将定义改为 `let a=5`；依赖结果标为过期，重新运行后得到 6。普通 `.omnb` 只保存源码，打开后需手动运行。

原生菜单支持新建、打开、保存 `.omnb`，以及 Markdown/LaTeX 导出。共享内核支持 [二维 SVG/PNG、纯数据 CSV/JSON 与 CLI 导出](docs/design/artifact-export.md)、[三维曲面/场景与 OBJ](docs/design/scene3d.md)。本版 Mac 和 Web 已完成本地安装、真实西瓜图形及求根验收；公开下载后的重复安装回读按用户要求省略，详见[安装记录](docs/acceptance/pre-alpha.5/local-install.md)。系统文件面板间歇禁用“打开”时，可在 Finder 用“打开方式 → 其他”指定本版应用。

AI 在设置中接入自己的模型服务，配置地址、模型、密钥和能力。本版 Mac 使用独立 Preview 数据目录，不自动迁移 `.4` 的配置或会话；Mac 密钥保存在系统钥匙串，Web 需服务允许 CORS。细节见[模型接入指南](docs/llm.md)。

## 科研计算与现代语法

`.3` 加入管道、匿名函数、范围、记录/字段、矩阵 `@` 与统一命名参数；旧现代语法、Wolfram 子集和 `.omnb` v1 继续使用。完整支持范围、参数、精度和失败语义见 [功能目录](docs/reference/README.md)。

```text
[1,2,3] |> map(fn(x)=>x^2)
integrate(exp(-x^2),x:-inf..inf,mode:"numeric")
let motion = ode(fn(t,y)=>[y[2],-y[1]],initial:[1,0],t:0..6)
sample(motion.solution,t:0..6)
plot(sin(x)*cos(y),x:-pi..pi,y:-pi..pi,view:"surface")
```

支持有边界的符号积分、极限/Taylor、机器数值积分、稠密矩阵分解、非刚性 ODE、局部优化/可认证凸二次优化、QR/LM 拟合、统计概率、可重复随机、纯数据解析和 SI 单位。新数值分析不冒充任意精度；未收敛、不可用精度、奇点、错误维度和取消明确报告。

二维曲线、参数图、隐式/区域/场/流线、密度、数据和频数图共同覆盖桌面/Web/iOS；三维网格、图元、样式、变换与旋转/光照先覆盖桌面/Web，CLI 导出 OBJ。iOS 三维明确保留源式并提示未适配。[地月 L2 与完整西瓜笔记本](docs/examples/README.md) 随仓库交付；[真实验收截图](docs/acceptance/r36b/README.md) 可查。

## 从源码运行与构建

安装 Git、仓库锁定的 Rust 1.94.0 和 Node 26.0.0。原生 Mac 构建需要 Xcode 27 / macOS 27 SDK，见[原生 Mac 指南](macos/README.md)；其他开发环境见[开发指南](docs/development.md)。

```sh
git clone https://github.com/He-RT/om-openmath.git
cd om-openmath
cargo install --path crates/om-cli --locked
cargo install wasm-bindgen-cli --version 0.2.129 --locked
npm ci --prefix app
npm run dev --prefix app             # 浏览器笔记本
# 原生 Mac：
bash macos/Scripts/build-native.sh
```

```sh
npm run build --prefix app           # 静态网站：app/dist
bash macos/Scripts/package-native.sh release-assets # 原生 Mac app ZIP 与 DMG
cargo build -p om-cli --release --locked
```

`om` 进入 REPL；`om run FILE.om` 或 `om run FILE.omnb` 顺序执行源码。使用 `:help`、`:steps`、`:latex`、`:ask`、`:explain`、`:dialect`、`:clear`，或 `om config path|show|edit` 管理原生配置。

## 文档与贡献

- [安装及常见问题](docs/install.md)、[输入语言与交互](docs/language.md)
- [求解语义与边界](docs/solve.md)、[模型接入与隐私](docs/llm.md)
- [开发与验证](docs/development.md)、[内核协议](docs/protocol.md)、[发布流程](docs/releasing.md)
- [全景功能目录](docs/reference/README.md)、[现代语言设计](docs/design/modern-language.md)、[下一版计划](docs/plan/NEXT_RELEASE.md)（规划接口尚未实现）
- [原生 iOS/iPadOS 使用与构建](docs/ios.md)、[移动端验收记录](docs/ios-acceptance.md)
- [本版发行说明](docs/release-0.1.0-pre-alpha.5.md)、[安装验收与发布记录](docs/acceptance/pre-alpha.5/local-install.md)、[本版实施与验收进度](docs/plan/PRE_ALPHA_5.md)

欢迎提交带有输入、方言、平台、期望和实际结果的 [Issue](https://github.com/He-RT/om-openmath/issues)。请不要附带 API 密钥。

以 [MIT](LICENSE-MIT) 或 [Apache-2.0](LICENSE-APACHE) 双许可证发布。[第三方声明](THIRD_PARTY_NOTICES.md)保留原许可证文本。本项目独立于 Wolfram Mathematica，也不是 OpenMath 交换标准的实现。
