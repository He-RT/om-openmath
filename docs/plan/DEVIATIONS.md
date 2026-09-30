# Implementation rulings

The approved product scope is unchanged. Entries below resolve engineering conflicts in PLAN.md.

## 2026-09-30 — foundational preflight

- **P1 — cancellation types:** M2.8 produces `Interrupt/Clock/Abort`, consumed by M5–M9. Define these expression-independent types in `om-num::ctx` and re-export from `om-core::ctx`; messages remain in om-core. This preserves om-poly's no-om-core dependency rule and every consumer's concrete type. Cost if wrong: moving a small module, without changing signatures.
- **P2 — Rust/reedline:** keep Rust 1.94.0, pin reedline to `=0.49.0`. The registry records Rust 1.95.0 for reedline 0.50–0.52 and 1.63.0 for 0.49.0. Cost if wrong: update CLI adapters when a newer toolchain is adopted.
- **P3 — progress hashes:** mark a task complete with verification commands in its commit; add that commit's hash in the next task commit. A commit cannot contain its own final hash. The ignored execution ledger records hashes immediately after committing.
- **P4 — contract names:** use `Abort::Interrupted` and `Verification`; fix obsolete milestone references and the plan header's section numbers. `recip(0.)` returns `NumError::DivByZero`, not an IEEE infinity, preserving the numeric layer's finite-value policy.
- **P5 — tool setup:** install cargo-deny now; defer wasm-bindgen-cli and cargo-insta until their first consumer. The WASM target is already installed. Avoid large unused tool builds and pin wasm-bindgen-cli to the eventual locked crate version. Cost if wrong: a deferred one-time tool installation.
- **P6 — bootstrap CI:** M0.1 validates WASM with om-num, the only crate that exists; switch to om-kernel in M0.2. Reserve frontend/type-generation checks for their introducing tasks so CI never invokes nonexistent packages.
- **P7 — license policy:** MPL is not globally allowed; any required MPL-only transitive package needs a named exception. This enforces the plan's transitive-only allowance rather than permitting new direct MPL dependencies.
- **P8 — task tooling:** existing task-start extracts `Task N` headings, not this plan's `M<n>.<n>` checkboxes. Generate per-task briefs directly from the authoritative plan, retaining BASE and task-done test logs. Cost if wrong: only execution tooling, not application behavior.
- **P9 — execution:** the user prohibited further workflows/subagents. Implement and perform separate review passes inline, with fmt/clippy/test gates. No automatic merge/publication; the user authorized pushing verified commits to `https://github.com/He-RT/om-openmath.git` on 2026-09-30.

- **P10 — scaffold API:** omit meaningless `placeholder()` exports and tests. Cargo package-graph boundary tests and actual native/WASM builds verify scaffolding without creating fake runtime functionality. Source modules are otherwise empty until their implementation task.
- **P11 — dependencies:** reserve approved external versions in workspace.dependencies and activate each dependency at its first consumer. `ctrlc` is included because §12.12 already requires it, despite its omission from the main whitelist. HTTP and native config feature slots exist now; their transports are introduced and tested in M11/M12.

## Deferred contract checks (resolve before first consumer)

- M4.4/M4.5 use exact special values and numeric expression evaluation assigned to M8. Implement the shared special/numeval modules before their first evaluator consumer; do not add an upward dependency.
- Ball domains, outward rounding and full-range representation need a rigorous interface decision in M1.3, not a guessed `1 ulp` bound for every operation.
- Ring static zero/one cannot select a runtime finite-field modulus; settle coefficient-context construction in M5.1.
- Kernel view/envelope/span and LLM transport ownership gaps will be resolved and fixture-tested before M11/M12, retaining the public fields where possible.

No preflight agent returned a completed report; neither math nor protocol correctness has been certified by those failed runs.
