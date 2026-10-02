# Third-party notices

No third-party algorithm source has been copied into the current scaffold. Rust and npm dependencies retain their upstream licenses.

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
