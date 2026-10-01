# Kernel JSON protocol

`om_kernel::protocol` defines the shared request, response and event schema. M11.1
provides types; runtime dispatch and notebook execution follow in the remaining
M11 tasks. The CLI, WASM worker and desktop host will use this same contract.

Requests and responses use `Envelope<T> { id, body }`. Their numeric IDs match;
events use ID zero. Browser clients must keep IDs and millisecond timeouts within
`Number.MAX_SAFE_INTEGER`. LLM jobs have separate string `request_id` values.

```json
{"id":42,"body":{"type":"evaluate","cell_id":"a","source":"solve(x^2=2,x)","dialect":"Modern"}}
```

Operation tags use snake_case. Parser dialects are `Modern`, `Wolfram`, `Auto`;
cell kinds/statuses, token classes and message levels retain their capitalized
Rust spellings. Configuration dialects use `modern`, `wolfram`, `auto` as in the
TOML configuration example. Solution kinds are `finite`, `all`, `none`, `region`.
Verification retains the solver representation: `"Exact"`, `"ByConstruction"`,
`{"Numeric":{"digits":200}}` or `"Unverified"`.

`Response::Preview` is flattened into the tagged object:

```json
{"type":"preview","latex":"x^2","diagnostics":[],"tokens":[],"dialect":"Modern","actions":[]}
```

Optional fields serialize as null. Tuples (token/span pairs, coordinates and
ranges) serialize as arrays; parameter maps have string keys. Most DTO fields
are required on input. Option fields may be omitted, and configuration sections
and profile options accept documented defaults. Configuration defaults follow
PLAN §10.8; an omitted capability is false, so a custom provider must explicitly
advertise tool and JSON support.

`StepsView.root` contains `StepView` values with stable IDs, rule IDs, levels,
title keys, LaTeX parameter maps, before/after LaTeX arrays and children. No
expression tree or computational `StepKind` crosses the wire. Owned diagnostic
adapters preserve the parser JSON without leaking incoming codes to static memory.

Notebook files contain only `version` (currently 1), `title` and ordered `cells`.
Each cell contains `id`, `kind`, `source`, `dialect`. Configuration, API keys,
runtime definitions and cached outputs are excluded from the file schema.

A present `ProfileConfig.api_key` serializes and formats for Debug as `"***"`.
Before installing a submitted configuration, call `merge_redacted_keys` with
the current configuration: masks retain the existing key by profile name, while
null/missing keys clear it. A mask for a new profile resolves to no key. Real
credentials in a browser `HttpRequest` belong to the explicit transport request;
configuration masking does not alter HTTP headers. Native persistence and key
resolution are implemented in M11.8.

Generate bindings with `cargo test -p om-kernel export_bindings --locked`. All
reachable types are committed under `app/src/kernel/generated/`. The derive
export paths are fixed from each crate's manifest directory, because ts-rs 12's
default base is `bindings/`. Do not edit generated files. CI regenerates them and
checks both tracked differences and unexpected files; frontend typecheck and
tests validate imports, discriminant narrowing and representative JSON shapes.
The single export test also removes trailing line whitespace from ts-rs output;
per-type automatic exporters are disabled to keep regeneration consistent.
