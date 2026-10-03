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
