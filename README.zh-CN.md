# OpenMath

以方程求解为核心的 Rust 计算机代数系统，计划提供桌面、浏览器和终端界面。

> **开发初期：** 正按任务逐步实现，目前还不是可用的求解器。已经验证的功能以[实施进度](docs/plan/PROGRESS.md)为准。

## 开发

`rust-toolchain.toml` 固定 Rust 1.94.0，rustup 会安装 WASM target、rustfmt 和 clippy。

```sh
cargo build --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
python3 -m unittest discover -s tests -v
cargo build -p om-num --target wasm32-unknown-unknown
cargo install cargo-deny --version 0.20.2 --locked
cargo deny check
```

## 设计方向

- 精确符号计算、结构化推导步骤与交互式可视化。
- 现代数学语法和 Wolfram Language 子集。
- CLI、浏览器 WASM Worker 与 Tauri 桌面共享同一 Rust 内核。
- 可配置 LLM 提供商；模型只提议表达式，CAS 负责解析和验证，不执行宿主代码。

[实施计划](docs/plan/PLAN.md)是权威规格，[DEVIATIONS.md](docs/plan/DEVIATIONS.md)记录工程裁决。

本项目独立于 Wolfram Mathematica 和 OpenMath 数学交换标准，crate 前缀为 `om-`。

## 许可证

可选择 [MIT](LICENSE-MIT) 或 [Apache-2.0](LICENSE-APACHE)。

[English](README.md)
