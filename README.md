# OpenMath

A Rust computer algebra system centered on equation solving, with planned desktop, browser and terminal interfaces.

> **Early development:** the repository is being implemented task by task. It is not yet a usable solver. See [implementation progress](docs/plan/PROGRESS.md) for verified capabilities.

## Development

Rust 1.94.0 is pinned in `rust-toolchain.toml`. Rustup installs the WASM target, rustfmt and clippy automatically.

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

## Design

- Exact symbolic mathematics and structured derivation steps.
- Modern input syntax plus a Wolfram Language subset.
- A shared Rust kernel for a CLI, WASM browser worker and Tauri desktop host.
- Configurable LLM providers: proposals are parsed and verified by the CAS, not executed as host code.

The [implementation plan](docs/plan/PLAN.md) is authoritative. Decisions are tracked in [DEVIATIONS.md](docs/plan/DEVIATIONS.md).

This project is independent of Wolfram Mathematica and the OpenMath interchange standard; `om-` is the crate prefix.

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option.

[中文说明](README.zh-CN.md)
