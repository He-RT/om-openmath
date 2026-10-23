# OpenMath

[简体中文](README.md) · [Downloads](https://github.com/He-RT/om-openmath/releases/tag/v0.1.0-pre-alpha.5) · [Installation](docs/install.md)

OpenMath is a pre-alpha computer algebra system centered on equation solving. A shared Rust kernel powers the terminal, browser notebook, native Mac app and native iOS/iPadOS client. It supports modern notation and a Wolfram Language subset, exact solutions, recorded derivations, interactive plots and configurable AI assistance.

[**0.1.0-pre-alpha.5**](https://github.com/He-RT/om-openmath/releases/tag/v0.1.0-pre-alpha.5) introduces workbench v2 for native Mac and Web: a notebook outline, source wells and paper results, explicit result states, and assistant/steps/variables inspectors. The assistant supports execution and discussion modes, per-message action records, source diffs, undo, attachments and stopping. Execution mode can edit and run the notebook through tools; discussion mode is read-only with temporary calculations. Credentials and conversations stay out of `.omnb` files.

```sh
om -e 'solve(x^2 - 5x + 6 = 0, x)'
# x = 2  │  x = 3
om -e 'Solve[x^2-5x+6==0,x]'
om --json -e 'solve(x^2=2,x)'
```

Scientific features include bounded symbolic calculus, machine quadrature, dense matrix decomposition, nonstiff ODE, local optimization/certified convex quadratics, QR/LM fitting, statistics, SI units and pure data formats. The [function catalog](docs/reference/README.md) records precision, platform and mathematical limits. [Earth–Moon L2 and watermelon notebooks](docs/examples/README.md) are included in the repository. Unsupported problems retain their source and diagnostics; support does not imply full Mathematica semantics.

## Download and use

The [`.5` release](https://github.com/He-RT/om-openmath/releases/tag/v0.1.0-pre-alpha.5) contains Mac/Web packages, a Windows MSI, an Android APK, two CLI ZIPs and native iOS/iPadOS development assets. `release-manifest.json` covers the original Mac/Web packages and `cli-release-manifest.json` covers the CLI supplements at source revision `86da0620…`. Separate Tauri and iOS manifests record each supplement's actual source, sizes and SHA256 hashes. The original tag and published files are preserved.

| Platform | Asset | Usage |
|---|---|---|
| macOS 27+, Apple Silicon | `OpenMath_0.1.0-pre-alpha.5_aarch64.dmg` | Drag OpenMath Preview.app to Applications |
| macOS 27+, Apple Silicon | `OpenMath_0.1.0-pre-alpha.5_macos_arm64.app.zip` | Extract OpenMath Preview.app |
| Browser | `OpenMath-web_0.1.0-pre-alpha.5.zip` | Extract and serve over HTTP(S) |
| Windows 10/11 x64 | `OpenMath_0.1.0-pre-alpha.5_x64_zh-CN.msi` | Install the complete Tauri 2 workbench |
| Android 7.0+, ARM64 / x86_64 | `OpenMath_0.1.0-pre-alpha.5_android.apk` | Install the release-signed APK; compileSdk / targetSdk 35 |
| macOS ARM64 CLI | `om-cli_0.1.0-pre-alpha.5_macos_arm64.zip` | Extract, run `./om`, optionally add to PATH |
| Windows x64 CLI | `om-cli_0.1.0-pre-alpha.5_windows_x64.zip` | Extract, run `om.exe`, optionally add to PATH |
| iOS/iPadOS 27 | ARM64 simulator app, unsigned device app and XCFramework | Native SwiftUI/UIKit; device installation requires your own signing |

The Mac application bundles Node, Pi and runtime resources. End users do not need Rust, Xcode, Node or npm. It is ad hoc signed and has no Developer ID signature or Apple notarization. This Preview uses its own `.5` data directory and does not automatically migrate `.4` settings, keys or sessions. Mac keys use the system keychain; browser keys are remembered only by explicit opt-in. Web providers must allow CORS.

Open a [sample notebook](docs/examples/README.md) or enter `solve(x^2 - 5*x + 6 = 0, x)` in a math cell and run it to obtain 2 and 3. Notebook files store source, so reopening does not automatically recalculate. If the native file panel disables Open for a valid `.omnb`, use Finder → Open With → Other and select the installed OpenMath Preview.app; this path passed local cold-start and save/reopen checks.

Windows and Android use Tauri 2 to bundle the complete Web workbench: local WASM computation, 2D/3D plots, the existing assistant, settings, image attachments and exports. Open/save operations use system file pickers. The assistant retains the Web provider/CORS requirements and opt-in WebView credential storage. The Windows MSI has no Authenticode signature and may download WebView2 when missing. Android uses a persistent release signing key and the system WebView. See the [Tauri package record](docs/acceptance/pre-alpha.5/tauri-release.md).

The CLI supplements retain the existing terminal features and do not add the notebook assistant. They need no Node or GUI runtime. Native iOS/iPadOS 27 development assets are included in `.5`: an ad hoc signed ARM64 simulator app, an unsigned ARM64 device app and the XCFramework. Device installation requires your own signing. See the [mobile guide](docs/ios.md) and [40-test-per-device acceptance record](docs/acceptance/pre-alpha.5/ios/README.md). No Intel Mac, Windows ARM64 or Linux installer is provided.

The original three `.5` packages passed local installation/extraction, actual watermelon rendering, solving for 2/3 and version checks; the source CI passed. At the user's request, anonymous public downloads and repeat installation/readback were omitted after publication. The CLI supplements separately passed extracted-ZIP version and 2/3 checks on each platform; see the [CLI record](docs/acceptance/pre-alpha.5/cli-release.md). See the [release notes](docs/release-0.1.0-pre-alpha.5.md) and [installation and publication record](docs/acceptance/pre-alpha.5/local-install.md) for the exact scope and known limitations.

## Run from source and build

Use Rust 1.94.0 and Node 26.0.0 as pinned by the repository. Native Mac builds require Xcode 27 and the macOS 27 SDK; see the [Mac guide](macos/README.md) and [development guide](docs/development.md).

```sh
cargo install --path crates/om-cli --locked
cargo install wasm-bindgen-cli --version 0.2.129 --locked
npm ci --prefix app
npm run dev --prefix app             # browser notebook
bash macos/Scripts/build-native.sh   # native Mac app
```

```sh
npm run build --prefix app           # static website in app/dist
bash macos/Scripts/package-native.sh release-assets
cargo build -p om-cli --release --locked
```

`om` opens the REPL. Use `om run FILE.om` or `om run FILE.omnb` for sequential scripts; `:help`, `:steps`, `:latex`, `:ask`, `:explain`, `:dialect` and `:clear` are available. `om config path|show|edit` manages CLI settings. The native `.5` Mac app has separate Preview settings.

## Documentation

- [Input language and examples](docs/language.en.md), [solving semantics and limits](docs/solve.en.md)
- [Current Mac/Web/mobile AI configuration and privacy](docs/llm.md), [CLI configuration in English](docs/llm.en.md)
- [`.5` implementation and acceptance](docs/plan/PRE_ALPHA_5.md), [release process](docs/releasing.md), [protocol](docs/protocol.en.md)
- [Native iOS/iPadOS development](docs/ios.md), [complete documentation index](docs/README.md)

This project is independent of Wolfram Mathematica and the OpenMath interchange standard. Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE). Builds retain [third-party notices](THIRD_PARTY_NOTICES.md) and dependency licenses.
