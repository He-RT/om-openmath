# OpenMath 0.1.0 pre-alpha acceptance

The first pre-alpha implements the planned M0–M13 path: shared CAS/kernel,
terminal REPL, native notebook, production WASM notebook, configurable AI,
source notebook files, exports and local distribution packages.
This record describes the final local checks. [CI](https://github.com/He-RT/om-openmath/actions/workflows/ci.yml) enforces the repository gates on dev.

## Verified gates

| Requirement | Evidence |
|---|---|
| Rust quality and correctness | fmt/Clippy clean;878 workspace tests pass;3 workspace policy tests pass |
| Pure target | kernel builds for wasm32-unknown-unknown;14 pure protocol tests pass |
| Dependency policy | locked cargo-deny succeeds; no new third-party runtime dependency |
| Protocol |56 generated TypeScript files have no drift |
| Frontend | Node26 lint/typecheck/build and59 unit tests pass |
| Development browser |29 tests pass; explicit production performance test is skipped here |
| Actual static dist |27 UI tests plus the53-row performance test pass |
| Native authority/performance |53 cold om processes match original exact/700-bit numerical authority; each kernel calculation is below200ms |
| WASM authority/performance |53 cold production workers match the same authority through independent native validation; each calculation is below1s |
| Distribution | macOS arm64 .app/DMG built; final DMG checksum validates; Web zip includes deployable dist |
| Credentials | no credential-shaped literals in tracked files; tests use loopback/intercepted providers |

Two tests are ignored in the normal Rust suite: the opt-in performance test
inherited from earlier milestones and the production WASM authority runner
which requires a generated artifact. The native release gate is compiled only
for release and run explicitly. The actual release acceptance executes both
new authority/performance runners successfully.

## Measured performance and corrections

The final53-row release run reports a maximum native kernel time of
119.914ms, below the200ms gate. The maximum production WASM time is
145ms, below the1s gate. These are local measurements on the development
M-series Mac, not a hardware-independent latency guarantee. Each native case
starts a fresh process and each WASM case a fresh worker; automatic plotting is
disabled for the comparison, with default verification/steps retained.
Per-row data and the unchanged corpus digest are in [pre-alpha-results.json](pre-alpha-results.json).

The initial quintic/cyclic-3 measurements were1256/5581ms. Profiling identified
repeated root isolation, algebraic coordinate conversion/projection and an unused
complex argument in integer-power overflow checks. Bounded caches retain only
successful immutable proofs, with fresh interruption/deadline checks. Private
exact expansion reduces recovered coordinate expressions without altering stored
forms or steps. Exact zero/one and identical-complex-value identities avoid
reconstruction. CBall.log_magnitude preserves the real logarithm enclosure and
origin/range rejection while avoiding an unused argument computation.

A separate numeric projection regression proves that a recovered coordinate
which is exactly1 is presented as real1 rather than a tiny spurious imaginary
component. Original53 mathematical expectations and all source restrictions
remain unchanged. Inline review checked degree/precision/cancellation bounds,
constructor proofs, root ranks, principal branches, source poles, steps,
readonly state and new runtime unwrap/unsafe usage.

## Packaged native and production UI acceptance

The actual OpenMath.app window was inspected through CUA. It shows exact−3/1
solution cards for x²+2x=3, both highlighted intersections, live formula preview
and recorded factorization/zero-product steps. A definition changed from a=2 to
a=5 updates a+1 from3 to6. The final app also opens the saved source notebook
through its native picker and recomputes these actual results.

An isolated native configuration and synthetic loopback server exercise a real
probe (pong and first-byte/total timings), streaming explanation, parsed Ask
proposal and explicit execution returning±2. Ghost text appears after consent;
Tab inserts its exact source without running it. The same UI behaviors, model
draft/save/privacy controls, plots, cancellation/recovery and light/dark/Chinese
layouts are covered against real production WASM assets. Test fixture setup uses
normal saved-browser configuration rather than development-module replacement.

Native menus actually save source-only .omnb, export current results to Markdown,
and write a standalone LaTeX document. The exported LaTeX source compiles with the
built-in desktop compiler after its initial format-cache download. Unit tests
verify TeX escaping, safe Markdown fences and omission of stale output. Notebook
files exclude configuration, credentials, output caches and conversations.

Previous user-authorized DeepSeek smoke testing verified the shared-kernel
probe/translation/completion/chat route (P103). Automated and packaged acceptance
uses synthetic providers; no live key is stored in this repository.

## Reproduce

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
CARGO_PROFILE_TEST_OPT_LEVEL=2 cargo test --workspace --locked
python3 -m unittest discover -s tests -v
cargo build -p om-kernel --no-default-features --locked --target wasm32-unknown-unknown
cargo deny --locked check
cargo test -p om-kernel --no-default-features --test protocol --locked
cargo test -p om-cli --release --test corpus all_authority_rows_pass_native_release --locked -- --ignored
cd app
npm run lint && npm test && npm run build
OPENMATH_E2E_PREVIEW=1 OPENMATH_PERFORMANCE=1 npm run test:e2e -- --workers=1
npm run test:e2e -- --workers=1
cd ..
cargo test -p om-cli --release --test corpus production_wasm_results_match --locked -- --ignored
```

The production performance test writes target/pre-alpha/wasm-corpus.json for
independent original-authority validation. Frontend commands require the pinned
Node26 and matching wasm-bindgen0.2.129. Performance uses Python3.11+ to read the
authoritative TOML fixture. Optimized Rust test builds keep debug/overflow checks.

Local packages are target/release/bundle/macos/OpenMath.app,
target/release/bundle/dmg/OpenMath_0.1.0_aarch64.dmg,
target/release/bundle/web/OpenMath-web-0.1.0.zip and target/release/om.
Each Web/native distribution retains license notices for829 locked packages and
the certificate-data license. The current macOS packages are unsigned;
Windows/Linux installers are not verified. Browser providers require CORS, and
Unicode source in a LaTeX export may require a suitable Unicode font setup.
The bounded solving subset and unsupported cases are documented in [solve.md](solve.md).
