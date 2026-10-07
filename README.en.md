# OpenMath

[简体中文](README.md) · [Downloads](https://github.com/He-RT/om-openmath/releases) · [Installation](docs/install.md)

OpenMath [0.1.0-pre-alpha.3](https://github.com/He-RT/om-openmath/releases/tag/v0.1.0-pre-alpha.3) is a pre-alpha computer algebra system centered on equation solving. A shared Rust kernel powers the terminal, a reactive browser notebook, a Tauri desktop app and a native iOS/iPadOS client. Version .3 adds scientific computation and compositional syntax. It supports modern notation and a Wolfram Language subset, exact solution cards, recorded derivations, interactive plots and configurable AI assistance.

```sh
om -e 'solve(x^2 - 5x + 6 = 0, x)'
# x = 2  │  x = 3
om -e 'Solve[x^2-5x+6==0,x]'
om --json -e 'solve(x^2=2,x)'
```

AI proposes source; the CAS parses it and handles mathematical solving and verification. Suggestions wait for an explicit insert/run action. Credentials and conversations stay out of notebook files.

Scientific extensions include bounded symbolic calculus, machine quadrature, dense matrix decomposition, nonstiff ODE, local optimization/certified convex quadratics, QR/LM fitting, statistics, SI units and pure data formats. The [function catalog](docs/reference/README.md) records actual precision, platform and mathematical limits. 2D plots and isolated parameter exploration cover all clients; desktop/Web adds real WebGL2 scenes and OBJ. iOS preserves 3D source with an explicit unsupported-display message. [Earth–Moon L2 and full watermelon notebooks](docs/examples/README.md) include real acceptance evidence. Existing `.1/.2` releases remain available.

## Run from source

Use the pinned Rust 1.94.0 toolchain and Node 26.0.0 (`app/.nvmrc`).

```sh
cargo install --path crates/om-cli --locked
cargo install wasm-bindgen-cli --version 0.2.129 --locked
npm ci --prefix app
npm run dev --prefix app             # browser notebook
npm run tauri --prefix app -- dev    # desktop notebook
```

`om` opens the REPL. Use `om run FILE.om` or `om run FILE.omnb` for sequential scripts; `:help`, `:steps`, `:latex`, `:ask`, `:explain`, `:dialect` and `:clear` are available. `om config path|show|edit` manages native settings, and `om llm test PROFILE` probes an actual provider.

## Build

```sh
npm run build --prefix app
npm run preview --prefix app
npm run tauri --prefix app -- build --bundles app,dmg
cargo build -p om-cli --release --locked
```

The verified macOS arm64 build produces `target/release/bundle/macos/OpenMath.app` and `target/release/bundle/dmg/OpenMath_0.1.0-pre-alpha.3_aarch64.dmg`. These local pre-alpha packages are unsigned. `app/dist` is a deployable static site; serve it over HTTP(S) at the site root. Web providers must allow browser CORS; native HTTP has no browser CORS restriction. Windows x64 EXE/MSI installers are tested in the release workflow; Linux installers are outside this release.

Native menus support notebook files, Markdown/LaTeX export, themes, panels and help. Exports include current mathematical results and omit stale output. General preferences and AI model/routing settings are available in both notebooks. Browser credentials are remembered only by explicit opt-in; native keys use environment variables or the system vault.

## Documentation and verification

- [Input language and examples](docs/language.en.md)
- [Solving semantics and limits](docs/solve.en.md)
- [AI providers, privacy and CLI usage](docs/llm.en.md)
- [Pre-alpha acceptance record](docs/pre-alpha.en.md)
- [Implementation progress](docs/plan/PROGRESS.md), [protocol](docs/protocol.en.md)

Run formatting, workspace Clippy/tests, workspace policy, WASM build and cargo-deny checks. Frontend verification uses `npm run lint`, `npm test`, `npm run build` and `npm run test:e2e` in `app`. Set `OPENMATH_E2E_PREVIEW=1` after building to exercise production assets. Two direct development-module worker tests remain in the development suite; all UI acceptance tests run against production. No tests contact a real provider.

OpenMath implements a bounded solving subset, not all Mathematica semantics. Unsupported problems retain their source and diagnostics. See the solving guide for principal branches, original poles, exact/numeric verification and resource limits. This project is independent of Wolfram Mathematica and the OpenMath interchange standard.

Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE). Builds retain dependency notices and a locked-package index in `licenses/`. [中文说明](README.zh-CN.md)

## Windows downloads

Use the Chinese NSIS `OpenMath_0.1.0-pre-alpha.3_x64-setup.exe` or MSI `OpenMath_0.1.0-pre-alpha.3_x64_zh-CN.msi` on Windows 10/11 x64. Installers embed the WebView2 bootstrapper; missing runtimes require an internet connection. Desktop packages are unsigned and macOS packages are not notarized. Release assets include macOS arm64 DMG/app ZIP, both CLI ZIPs, a Web ZIP and SHA256 manifest.

The native iOS/iPadOS 27 client, ARM64 XCFramework and simulator application are documented in [the mobile guide](docs/ios.md). Physical devices use local Xcode automatic signing; no TestFlight/App Store release is included. The `.2` release remains under acceptance; `.1` stays available.
