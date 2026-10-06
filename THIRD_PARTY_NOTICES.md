# Third-party notices

No third-party algorithm source has been copied into the OpenMath. Rust and npm dependencies retain their upstream licenses.

## Tauri transitive MPL-2.0 dependencies

The desktop host brings these unmodified MPL-2.0 packages transitively; each has a version-specific exception in `deny.toml`, not a global MPL allowance:

- cssparser 0.37.0 and cssparser-macros 0.7.1 — https://github.com/servo/rust-cssparser
- dtoa-short 0.3.5 — https://github.com/upsuper/dtoa-short
- selectors 0.38.0 — https://github.com/servo/stylo
- option-ext 0.2.0 — https://github.com/soc/option-ext

Their source and license text are available in those upstream repositories and the crates.io source archives. No files from these packages have been modified. Release packaging must preserve applicable notices and source-availability obligations.

## Maintenance notice

`proc-macro-error 1.0.4` is an unmaintained build-time transitive dependency of Linux GTK/glib macros through Tauri. RUSTSEC-2024-0370 reports lack of maintenance, not a known vulnerability, and no safe upgrade is available within the current upstream chain. Only that advisory is excepted, with a reason, in `deny.toml`; all other advisories remain enforced.

Dependencies and their licensing are checked with `cargo deny check`. If algorithm source is adapted later, record its provenance and required license notice here before committing that adaptation.

## Native HTTPS certificate data

The prescribed reqwest 0.13.5 `rustls` transport includes unmodified
`webpki-root-certs 1.0.9` through rustls-platform-verifier on supported targets.
It contains Mozilla X.509 root certificate data under CDLA-Permissive-2.0.
Only that package/version has a certificate-data exception in `deny.toml`.
The upstream license text is preserved in `licenses/CDLA-Permissive-2.0.txt` and
must accompany distributed certificate data in release packages.
Upstream: https://github.com/rustls/webpki-roots

reqwest's specified `rustls` feature also selects its native AWS-LC cryptographic
provider. These native transport dependencies do not enter the default pure CAS
or WebAssembly runtime graph; their upstream permissive licenses remain enforced.

Distributed Web and desktop builds retain upstream license/notice texts in `licenses/rust/` and `licenses/npm/`. `licenses/INDEX.txt` records locked names, versions, license declarations and source URLs; it may also include build/test-only and other-platform dependencies from the lockfiles. The originals are copied without modification.

## iOS 原生组件与字体资源

SwiftPM 版本及传递依赖固定于 `ios/OpenMath.xcodeproj/project.xcworkspace/xcshareddata/swiftpm/Package.resolved`：

- SwiftMath **1.7.3**，MIT；未修改其代码。[上游](https://github.com/mgriebling/SwiftMath)。
- swift-markdown **0.9.0**，Apache-2.0；未修改其代码。[上游](https://github.com/swiftlang/swift-markdown)。
- swift-cmark **0.9.0**，保留 COPYING 中的 BSD 风格条款及相关贡献者声明；未修改其代码。[上游](https://github.com/swiftlang/swift-cmark)。

SwiftMath 随附 `mathFonts.bundle` 作为未经修改的字体资源分发。默认排版使用 Latin Modern Math；完整包的字体资源仍由 SwiftPM 保留。上游 bundle 随附的 MIT、GUST Font License（含对 LPPL 的引用）与 SIL Open Font License 必须保留，不能将其字体许可视为 OpenMath Rust 代码的统一许可。副本保存在 `ios/OpenMath/Resources/SwiftMath-*.txt`，同其他组件许可一起编入应用并随模拟器应用分发。禁止移除版权及字体保留名称条款。

原生 UI 参考库仅用于阅读可移植行为契约；没有复制 Telegram 类体系或源码，没有引入 Telegram 构建依赖。OpenMath 的 C ABI 桥接为本仓库原创实现。

## 二维导出标签字体与光栅化

`.3` dev 的共享 PNG 导出使用固定 `Swash 0.2.10`（Apache-2.0 OR MIT，仅std/scale/render）及其真实锁文件传递依赖，许可由现有包装脚本保留。它只解析/光栅化字体，不执行数学或提供三维框架。

字体 `OpenMathPlotLabels-Regular.otf` 是固定 Noto Sans SC Regular 的子集与重命名版本，OFL-1.1；版权/作者字段保留，文本许可、上游/产物 SHA256 与可复现生成方式见 [字体清单](licenses/plot-font/README.md)。正常运行使用内嵌资源，不下载字体或读取宿主系统字体路径。
