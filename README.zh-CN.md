# OpenMath

OpenMath 0.1.0 是以方程求解为核心的 pre-alpha 计算机代数系统。终端、响应式浏览器笔记本和 Tauri 桌面应用共享 Rust 内核，支持现代数学写法与 Wolfram 语言子集、精确解卡片、真实推导步骤、交互绘图和可配置 AI。

```sh
om -e 'solve(x^2 - 5x + 6 = 0, x)'
# x = 2  │  x = 3
om -e 'Solve[x^2-5x+6==0,x]'
om --json -e 'solve(x^2=2,x)'
```

AI 提议源码，由 CAS 解析、求解并验证数学结果。建议需要明确插入或运行；密钥和对话不进入笔记本文件。

## 从源码运行

使用仓库锁定的 Rust 1.94.0 与 Node 26.0.0（`app/.nvmrc`）。

```sh
cargo install --path crates/om-cli --locked
cargo install wasm-bindgen-cli --version 0.2.129 --locked
npm ci --prefix app
npm run dev --prefix app             # 浏览器笔记本
npm run tauri --prefix app -- dev    # 桌面笔记本
```

`om` 进入 REPL。`om run FILE.om`、`om run FILE.omnb` 顺序执行源码；`:help`、`:steps`、`:latex`、`:ask`、`:explain`、`:dialect`、`:clear` 提供交互操作。`om config path|show|edit` 查看或编辑原生配置，`om llm test PROFILE` 测试真实服务连接。

## 构建

```sh
npm run build --prefix app
npm run preview --prefix app
npm run tauri --prefix app -- build --bundles app,dmg
cargo build -p om-cli --release --locked
```

已验证的 macOS arm64 构建产物为 `target/release/bundle/macos/OpenMath.app` 和 `target/release/bundle/dmg/OpenMath_0.1.0_aarch64.dmg`，当前本地 pre-alpha 包未签名。`app/dist` 可作为静态网站部署，请通过 HTTP(S) 从站点根目录提供服务。Web 版模型服务需允许浏览器 CORS，桌面及终端使用原生 HTTP。此次发行尚未验证 Windows/Linux 安装包。

原生菜单支持笔记本文件、Markdown/LaTeX 导出、主题、面板和帮助。导出包含当前计算结果，排除过期输出。两种笔记本均提供常规及模型、功能路由设置；浏览器凭据仅在明确勾选后记住，原生密钥使用环境变量或系统密钥库。

## 文档与验证

- [输入语言和示例](docs/language.md)
- [求解语义与边界](docs/solve.md)
- [模型接入、隐私和终端操作](docs/llm.md)
- [pre-alpha 验收记录](docs/pre-alpha.md)
- [实施进度](docs/plan/PROGRESS.md)、[内核协议](docs/protocol.md)

开发验证包括 fmt、工作区 Clippy/tests、仓库契约、WASM build 和 cargo-deny。前端在 `app` 下运行 `npm run lint`、`npm test`、`npm run build`、`npm run test:e2e`；构建后设置 `OPENMATH_E2E_PREVIEW=1` 验证真实生产资源。两个直接导入开发模块的底层 worker 测试保留在开发套件，全部界面验收测试均运行于生产构建。测试不会访问真实模型服务。

当前提供有资源边界的求解子集，不等同于完整 Mathematica。无法支持的问题会保留原表达式并给出诊断；主值分支、原始分母限制及精确/数值验证见求解文档。本项目独立于 Wolfram Mathematica，也不是 OpenMath 交换标准的实现。

以 [MIT](LICENSE-MIT) 或 [Apache-2.0](LICENSE-APACHE) 双许可证发布；构建保留依赖许可文本与 `licenses/` 包索引。[English](README.md)
