# 开发指南

在 `dev` 分支接续，使用 Cargo workspace、npm 和锁文件。[CODEX_HANDOFF.md](../CODEX_HANDOFF.md) 给出交接约定。

## 环境与构建

使用 Rust 1.94.0（`rust-toolchain.toml`）、Node 26.0.0（`app/.nvmrc`）、Python 3.11+。`wasm-bindgen-cli 0.2.129` 必须与锁定的 Rust 绑定库一致。macOS 桌面需要 Xcode Command Line Tools；Windows 需要 Visual Studio C++ 工具、Windows SDK 和 WebView2。MSI 构建需启用 VBScript 可选组件。参见 [Tauri 前置条件](https://v2.tauri.app/start/prerequisites/)。

```sh
cargo install wasm-bindgen-cli --version 0.2.129 --locked
npm ci --prefix app
npm run dev --prefix app
# 或 npm run tauri --prefix app -- dev
```

`predev`、`pretest`、`prebuild` 编译实际 WASM，生成被 Git 忽略的绑定；不提交预编译内核。`npm run build --prefix app` 输出 `app/dist`，`postbuild` 复制上游许可。

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

接口新增或与计划不同的选择记入 `docs/plan/DEVIATIONS.md`，任务及验证记入 `PROGRESS.md`。保留数学期望，库代码禁止 `unsafe`，保持 WASM 依赖方向与资源上限。TypeScript 协议以 Rust 生成，不手工修改。发行见[发布流程](releasing.md)。

## 原生移动端

使用 Xcode 27.0 / SDK 27、Swift 6，以及 Rust 1.94.0 的两种 iOS ARM64 target。先运行 `bash ios/Scripts/build-kernel.sh`，再生成/打开 `ios/OpenMath.xcodeproj`。Swift 包和传递依赖精确锁定；[移动端指南](ios.md)描述 ABI、生命周期、Keychain 与测试。

## 新增常用函数

科研扩展按[研发账本](plan/NEXT_RELEASE.md)维护；[现代设计](design/modern-language.md)与[功能目录](reference/README.md)锁定接口。目录已接入运行时，统一规范别名、命名参数、管道位置、补全/Hover、帮助、AI 与能力查询；未实现条目不进入可执行推荐。

普通数学函数在共享 Rust 内核实现一次，桌面、Web、CLI 和原生移动端复用。按所属类别更新 `crates/om-eval/src/*_registry.rs` 的求值入口、参数约束和中英文 `DocEntry`；解析名称和显示语法分别检查 `om-core` / `om-parse` / `om-format`。本地补全与 Hover 读取同一函数文档，不另写 Swift 函数分发表。

为新函数加入独立数学期望，以及参数错误、定义域、精确/数值结果和中断的必要用例。保留现有 53 条权威语料的期望。涉及绘图时同时接入 `om-eval` 的数值编译器；涉及新的公式命令时检查 SwiftMath 的真实排版，未知命令仍显示可复制源码。

协议和输出类型不变时，无需改动 C ABI 或重做移动端导航；重建 XCFramework 后运行现有 Swift/界面门禁即可。本机模拟器会占用较多内存，当前用户约定由 GitHub CI 执行手机/平板模拟器测试；本地以真机或构建检查为主：

```sh
bash ios/Scripts/build-kernel.sh
# 本地验证通过后推送 dev，CI 自动执行两类模拟器门禁。
# 用户明确需要本地模拟器时，才手动运行 test-simulator.sh phone / pad。
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

Rust审计穷尽真实DocEntry注册名并检查当前示例；Python检查目录/证据/稳定身份/生成一致性和未实现条目不冒充支持。副作用和Agent事务的运行时仍在后续阶段；不引入Pi/Rig类型到业务协议，不把Agent会话/日志/提示词/权限或凭据塞进.omnb。

`.3` dev 的共享图形导出在om-kernel/artifact：仅绘制已采样几何，PNG标签用固定OFL字体/Swash纯Rust，支持WASM和两个iOS ARM64切片，不需要系统字体路径/GPU/Python运行时。新纯导出请求的独立尺寸/字节/遍历预算及SVG/PNG规范、实际文件回读在[导出设计](design/artifact-export.md)；维护字体时用锁定生成脚本和licenses/plot-font/manifest.json验证，不更新库内任意精度数学算法。
