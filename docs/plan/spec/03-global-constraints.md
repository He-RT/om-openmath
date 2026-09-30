<!-- Extracted from docs/plan/PLAN.md sections [3]. PLAN.md is authoritative; keep in sync. -->

## 3. 全局约束（Global Constraints）

每个任务都隐式包含本节全部要求。

- **Rust：** stable 1.94，`edition = "2024"`，`rust-version = "1.94"`。workspace 共享 `[workspace.package]` 与 `[workspace.dependencies]`。
- **WASM：** `om-num`、`om-core`、`om-parse`、`om-format`、`om-poly`、`om-simplify`、`om-solve`、`om-eval`、`om-kernel`（关闭 `native` feature 时）、`om-llm`（关闭 `http` feature 时）必须能编译到 `wasm32-unknown-unknown`。所以：这些 crate 不得依赖 `std::time::Instant`（在 wasm32 上会 panic）、线程、文件系统、`getrandom`、`tokio`。时钟与取消通过 `om_num::ctx::Interrupt` 注入（`om_core::ctx` 重导出相同类型，见 6.4），使 om-poly 不依赖表达式层。CI 里有专门任务：`cargo build -p om-kernel --no-default-features --target wasm32-unknown-unknown`。
- **许可证：** 每个 crate 的 `license = "MIT OR Apache-2.0"`；根目录放 `LICENSE-MIT`、`LICENSE-APACHE`；`deny.toml`（cargo-deny）只允许 `MIT, Apache-2.0, Apache-2.0 WITH LLVM-exception, BSD-2-Clause, BSD-3-Clause, ISC, Zlib, Unicode-3.0, CC0-1.0, MPL-2.0`（MPL 不作通用许可，只通过 deny.toml 中逐包例外允许必要的传递依赖）。**禁止：malachite（LGPL）、rug/gmp-mpfr-sys（LGPL）、algebraics（LGPL）、symbolica（非 OSI）、flint 绑定。**
- **依赖白名单**（版本为 2026-09-28 在 crates.io 查到的最新稳定版，Cargo.toml 中写 `"0.6"` 这种兼容版本即可）：

  | 用途 | crate | 版本 | 许可证 |
  |---|---|---|---|
  | 大整数/有理数/任意精度浮点 | `dashu`（`dashu-int`, `dashu-ratio`, `dashu-float`） | 0.6.1 | MIT OR Apache-2.0 |
  | 数值 trait | `num-traits` | 0.2.19 | MIT OR Apache-2.0 |
  | 小向量 | `smallvec` | 1.16 | MIT OR Apache-2.0 |
  | 快速哈希 | `rustc-hash` | 2.1 | MIT OR Apache-2.0 |
  | 有序 map | `indexmap` | 2.14 | MIT OR Apache-2.0 |
  | 错误类型 | `thiserror` | 2.0 | MIT OR Apache-2.0 |
  | 序列化 | `serde`（derive）、`serde_json` | 1.0.229 / 1.0.151 | MIT OR Apache-2.0 |
  | TS 类型生成 | `ts-rs` | 12.0 | MIT |
  | 配置 | `toml` 1.1、`directories` 6.0 | | MIT OR Apache-2.0 |
  | 密钥存储（仅 native） | `keyring` | 4.2 | MIT OR Apache-2.0 |
  | HTTP（仅 native） | `reqwest`（features: `json`, `stream`, `rustls`），版本 0.13 | 0.13.5 | MIT OR Apache-2.0 |
  | 异步（仅 native） | `tokio` 1.53、`futures` 0.3、`tokio-util` 0.7 | | MIT |
  | 终端 REPL | `reedline` =0.49.0（0.50+ 要求 Rust 1.95）、`nu-ansi-term` 0.50 | | MIT |
  | 诊断美化（CLI） | `ariadne` | 0.6 | MIT |
  | 日志 | `tracing` 0.1 | | MIT |
  | WASM 绑定 | `wasm-bindgen` 0.2.129、`js-sys` 0.3.106、`console_error_panic_hook` 0.1.7 | | MIT OR Apache-2.0 |
  | 桌面 | `tauri` 2.12、`tauri-build` 2.7、`tauri-plugin-dialog` 2.8、`tauri-plugin-fs` 2.6、`tauri-plugin-opener` 2.6 | | Apache-2.0 OR MIT |
  | 测试 | `proptest` 1.11、`insta` 1.48（features: `json`, `yaml`）、`pretty_assertions` 1.4、`criterion` 0.8、`wiremock` 0.6、`wasm-bindgen-test` 0.3 | | 宽松 |

  **有意不用：** 解析器生成器（chumsky/pest/lalrpop/logos），因为手写 Pratt 解析器在错误恢复、源码位置和两种方言切换上更可控；`egg`/`egglog`（v1 化简用启发式，e-graph 列入未来工作）；`async-openai`/`genai`/`rig-core`（LLM 层需要 sans-IO，同时支持 wasm 和 FIM 端点，所以自己写，大约 800 行）；`nalgebra`/`faer`（v1 数值线代规模很小，自研 LU/QR 即可）。
- **前端（npm，禁止 pnpm/yarn/bun）：** `react`/`react-dom` 19.3、`vite` 8.3、`typescript` 7.0、`@vitejs/plugin-react` 6.1、`vite-plugin-wasm` 3.6、`@codemirror/{state 6.7, view 6.43, commands 6.11, autocomplete 6.20, lint 6.9, language 6.12, search 6.7}`、`@lezer/highlight` 1.2、`katex` 0.18、`zustand` 5.0、`cmdk` 1.1（命令面板）、`@tauri-apps/api` 2.12、`@tauri-apps/plugin-dialog` 2.8、`@tauri-apps/plugin-fs` 2.6、`@tauri-apps/cli` 2.12（devDependency）、测试 `vitest` 5.0、`@testing-library/react` 16.3、`jsdom` 30、`@playwright/test` 1.63、lint `eslint` 10 + `typescript-eslint` 8.70。绘图**不用第三方库**，自写 SVG 组件（见 12.6），以便精确控制解点高亮、区间着色和主题。
  > 如果执行时发现某个版本号与上表不一致（例如大版本升级导致 API 变化），**锁定到上表中的版本**，不要追新。
- **工具安装（一次性，M0 完成）：** `rustup target add wasm32-unknown-unknown`；`cargo install wasm-bindgen-cli --version 0.2.129 --locked`（版本**必须**与 `wasm-bindgen` crate 完全一致）；`cargo install cargo-deny --locked`；`cargo install cargo-insta --locked`。
- **代码风格：** 所有公共项写 `///` 文档注释；模块内部注释只解释“为什么”。错误类型用 `thiserror`；库 crate 不用 `anyhow`（`om-cli`、`om-desktop` 可以用）。
- **国际化：** UI 与 CLI 文案提供 `zh-CN`（默认跟随系统）与 `en`；步骤记录（Steps）里的文案**不写死自然语言**，只存结构化数据，渲染时按语言模板生成（见 8.6）。
- **性能底线（验收时测）：** 第 14 节语料里除标注 `slow` 的用例外，单条 `Solve` 在 M 系列 Mac 上的 release 构建里 < 200 ms；WASM 版本 < 1 s。
- **安全：** LLM 输出**只能**作为源码字符串交给 `om-parse`，从不以任何形式执行（无 `eval`、无 shell）；API 密钥不写入笔记本文件、不写日志、不发送到配置之外的主机。
