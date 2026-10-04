//! New source structures roundtrip in both dialects, including literal keys and local aliases.
use om_parse::{Dialect, parse_expr};
#[test]
fn composed_source_is_lossless_including_local_builtin_alias_heads() {
    for source in [
        "1.23..2.34",
        "{\"中文\": 2, color: 3}",
        "record()",
        "fn(sin)=>sin(x)",
        "(fn(x)=>fn(y)=>x+y)(2)(3)",
        "v[2..4]",
        "config.color",
        "A @ b",
    ] {
        let expr = parse_expr(source, Dialect::Modern).unwrap();
        for (rendered, dialect) in [
            (om_format::modern_form(&expr), Dialect::Modern),
            (om_format::input_form(&expr), Dialect::Wolfram),
        ] {
            let reparsed =
                parse_expr(&rendered, dialect).unwrap_or_else(|e| panic!("{rendered}: {e:?}"));
            assert_eq!(
                om_core::canonicalize(&expr),
                om_core::canonicalize(&reparsed),
                "{source} => {rendered}"
            );
        }
    }
}
