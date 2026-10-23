# 开发指南

从 `dev` 创建短期任务分支接续，使用 Cargo workspace、npm 和锁文件。[CODEX_HANDOFF.md](../CODEX_HANDOFF.md) 给出交接约定；文件入库、忽略规则与验收档案的边界见 [Git 管理约定](git-management.md)。

当前 `.5` 包括原生 Mac、Web，以及 Tauri 2 完整封装 Web 的 Windows / Android，实施与验收记录见 [PRE_ALPHA_5.md](plan/PRE_ALPHA_5.md)；原生 iOS/iPadOS 开发附件已追加公开，[移动补包记录](acceptance/pre-alpha.5/ios-release.md)单独记录实际来源。原生 Mac 使用 SwiftUI/AppKit，构建入口为 `bash macos/Scripts/build-native.sh`，自包含包入口为 `bash macos/Scripts/package-native.sh release-assets`；需要 Xcode 27 / macOS 27 SDK，详见 [macos/README.md](../macos/README.md)。Tauri 不生成本版原生 Mac 发行包。

原生 iOS/iPadOS `.5` 追加范围见[移动计划](plan/PRE_ALPHA_5_IOS.md)及[构建指南](ios.md)。`check-native.sh` 检查宿主逻辑，`build-candidate.sh` 只编译真机/模拟器目标；后者不会创建或启动模拟器。移动端采用 SwiftUI/UIKit、SceneKit Metal 和 JavaScriptCore Pi，新增源码事务、附件和助手持久化，不依赖 Node 进程或 WebView。实际模拟器操作需遵守本轮 Device Hub 单台限制，不默认重跑旧移动工作流。

## 环境与构建

使用 Rust 1.94.0（`rust-toolchain.toml`）、Node 26.0.0（`app/.nvmrc`）、Python 3.11+。`wasm-bindgen-cli 0.2.129` 必须与锁定的 Rust 绑定库一致。macOS 桌面需要 Xcode Command Line Tools；Windows 需要 Visual Studio C++ 工具、Windows SDK 和 WebView2。MSI 构建需启用 VBScript 可选组件。参见 [Tauri 前置条件](https://v2.tauri.app/start/prerequisites/)。

```sh
cargo install wasm-bindgen-cli --version 0.2.129 --locked
npm ci --prefix app
npm run dev --prefix app
# 或 npm run tauri --prefix app -- dev
```

`predev`、`pretest`、`prebuild` 编译实际 WASM，生成被 Git 忽略的绑定；不提交预编译内核。`npm run build --prefix app` 输出 `app/dist`，`postbuild` 复制上游许可。

### Tauri 2 Windows / Android 封装

设置 `VITE_OPENMATH_WEB_SHELL=1`，使用 `web-shell` Cargo feature。共享前端继续使用 WASM 计算和现有 Web 助手，Tauri 文件 API 独立提供系统打开、保存和导出。Windows 的构建命令（PowerShell）为：

```powershell
$env:VITE_OPENMATH_WEB_SHELL = '1'
npm run tauri --prefix app -- build --features web-shell --bundles msi -- --no-default-features
```

Android 需要 JDK 17、Android SDK Platform 35、build-tools 35.0.0、NDK 28.2.13676358，以及 `aarch64-linux-android` / `x86_64-linux-android` Rust target。生成工程后由脚本统一 app 与库的 compileSdk=35，并固定 targetSdk=35、minSdk=24：

```sh
export VITE_OPENMATH_WEB_SHELL=1
npm run tauri --prefix app -- android init --ci --skip-targets-install
python3 scripts/prepare_android.py
npm run tauri --prefix app -- android build --ci --apk --features web-shell --target aarch64 x86_64
```

发布 APK 需再 zipalign 和使用持久发行密钥签名；私钥和口令不入库。可复现构建入口、独立源码 SHA 和最小实际包检查见[发布流程](releasing.md)及[Tauri 补包记录](acceptance/pre-alpha.5/tauri-release.md)。

## 验证

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
CARGO_PROFILE_TEST_OPT_LEVEL=2 cargo test --workspace --locked
python3 -m unittest discover -s tests -v
cargo build -p om-kernel --no-default-features --locked --target wasm32-unknown-unknown
cargo deny --locked check
cargo test -p om-kernel export_bindings --locked
git diff --exit-code app/src/kernel/generated
cd app
npm run lint
npm test
npm run build
npx playwright install chromium
npm run test:e2e
OPENMATH_E2E_PREVIEW=1 npm run test:e2e
```

PowerShell 使用 `$env:OPENMATH_E2E_PREVIEW = '1'`，再运行相同 npm 命令。性能及原始语料命令见[验收记录](pre-alpha.md)。自动测试只用合成提供商和本地 HTTP；真实服务测试需明确授权。

## 代码结构

| 位置 | 职责 |
|---|---|
| `om-num`、`om-core` | 数值、表达式及中断预算 |
| `om-parse`、`om-format` | 双方言解析和展示 |
| `om-poly`、`om-simplify`、`om-solve` | 多项式、化简、求解及证明 |
| `om-eval`、`om-kernel` | 求值器、会话、依赖与 JSON 协议 |
| `om-llm` | 请求构造、流解码和模型任务 |
| `om-cli`、`om-wasm` | 终端及浏览器绑定 |
| `app/src`、`app/src-tauri` | 共享前端及原生宿主 |
| `macos/OpenMathNative`、`om-apple-ffi` | 原生 Mac 工作台、存储、助手与 Rust C ABI |
| `agent`、`agent/prompts` | Pi 运行时适配及共用提示词源 |

本版任务、验证与设计差异记入 [PRE_ALPHA_5.md](plan/PRE_ALPHA_5.md)；旧总账为 `PROGRESS.md` / `DEVIATIONS.md`。保留数学期望，库代码禁止 `unsafe`，保持 WASM 依赖方向与资源上限。TypeScript 协议以 Rust 生成，不手工修改。发行见[发布流程](releasing.md)。

## 原生移动端

使用 Xcode 27.0 / SDK 27、Swift 6，以及 Rust 1.94.0 的两种 iOS ARM64 target。先运行 `bash ios/Scripts/build-kernel.sh`，再生成/打开 `ios/OpenMath.xcodeproj`。Swift 包和传递依赖精确锁定；[移动端指南](ios.md)描述 ABI、生命周期、Keychain 与测试。

## 新增常用函数

科研扩展按[研发账本](plan/NEXT_RELEASE.md)维护；[现代设计](design/modern-language.md)与[功能目录](reference/README.md)锁定接口。目录已接入运行时，统一规范别名、命名参数、管道位置、补全/Hover、帮助、AI 与能力查询；未实现条目不进入可执行推荐。

普通数学函数在共享 Rust 内核实现一次，桌面、Web、CLI 和原生移动端复用。按所属类别更新 `crates/om-eval/src/*_registry.rs` 的求值入口、参数约束和中英文 `DocEntry`；解析名称和显示语法分别检查 `om-core` / `om-parse` / `om-format`。本地补全与 Hover 读取同一函数文档，不另写 Swift 函数分发表。

为新函数加入独立数学期望，以及参数错误、定义域、精确/数值结果和中断的必要用例。保留现有 53 条权威语料的期望。涉及绘图时同时接入 `om-eval` 的数值编译器；涉及新的公式命令时检查 SwiftMath 的真实排版，未知命令仍显示可复制源码。

协议和输出类型不变时，无需改动 C ABI 或重做移动端导航；重建 XCFramework 后按变化运行相关 Swift/界面检查。常规 CI 不自动启动 iOS 模拟器；运行测试需显式许可，本机须使用 Device Hub 且同时最多存在一台，关机也计入。当前模拟器验收已完成，数量为 0，不重复启动：

```sh
bash ios/Scripts/build-kernel.sh
# 从任务分支提交 PR，适用 CI 通过后合入 dev。
# 只有明确允许运行模拟器后，才派发 run_ios/run_simulators 或运行测试脚本。
```

每轮使用独立模拟器并保留独立 `.xcresult`，结束后移除本轮临时模拟器。文档恢复测试使用独立 UserDefaults，HTTP fixture 使用独立端点，避免前一用例的自动保存或取消请求影响下一用例。仓库文本显式使用 UTF-8；Xcode 环境检查完整读取版本输出。上述基础设施不应因添加数学函数而反复出现同一错误。新协议字段、结果类型或算法行为仍需相应契约测试及各平台适配。

### 目录维护

编辑 `docs/reference/functions.toml`，规范名/别名可以演进，稳定 `id` 及已有回调归属不能随之变化；新增身份显式追加到身份账本。参数 `default` 当前是说明文本，不是Agent工具schema的默认值。副作用分类是预留描述，不能替代只读执行、权限和传递性检查。

```sh
python3 scripts/function_docs.py
python3 scripts/function_docs.py --check
python3 -X warn_default_encoding -W error::EncodingWarning -m unittest discover -s tests -v
CARGO_PROFILE_TEST_OPT_LEVEL=2 cargo test -p om-eval --test function_catalog --locked
```

Rust审计穷尽真实DocEntry注册名并检查当前示例；Python检查目录/证据/稳定身份/生成一致性和未实现条目不冒充支持。Mac/Web助手已通过宿主工具接入实际源码事务、运行与撤销；函数目录中的副作用分类不能替代权限和事务校验。不引入Pi/Rig类型到业务协议，不把Agent会话/日志/提示词/权限或凭据塞进.omnb。

`.3` dev 的共享图形导出在om-kernel/artifact：仅绘制已采样几何，PNG标签用固定OFL字体/Swash纯Rust，支持WASM和两个iOS ARM64切片，不需要系统字体路径/GPU/Python运行时。新纯导出请求的独立尺寸/字节/遍历预算及SVG/PNG规范、实际文件回读在[导出设计](design/artifact-export.md)；维护字体时用锁定生成脚本和licenses/plot-font/manifest.json验证，不更新库内任意精度数学算法。
