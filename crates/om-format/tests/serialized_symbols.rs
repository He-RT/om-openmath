//! Transport must preserve modern symbol identities; public Wolfram pattern grammar is unchanged.
use om_core::{Expr, canonicalize};
#[test]
fn input_form_reader_preserves_underscores_heads_patterns_precision_and_unicode() {
    for expr in [
        Expr::symbol("earth_x_km"),
        Expr::symbol("_private_变量"),
        Expr::normal(Expr::symbol("my_function"), [Expr::symbol("some_value")]),
        om_parse::parse_expr("f[x_Real]:=x^2", om_parse::Dialect::Wolfram).unwrap(),
        om_parse::parse_expr("0.125`50", om_parse::Dialect::Wolfram).unwrap(),
    ] {
        let source = om_format::input_form(&expr);
        assert_eq!(
            canonicalize(&om_parse::parse_input_form(&source).unwrap()),
            canonicalize(&expr),
            "{source}"
        );
    }
    let pattern = om_parse::parse_expr("x_", om_parse::Dialect::Wolfram).unwrap();
    assert!(pattern.is_head(om_core::BUILTIN::PATTERN));
    assert_eq!(
        om_parse::parse_input_form("x_")
            .unwrap()
            .as_symbol()
            .unwrap()
            .name(),
        "x_"
    );
    assert!(om_parse::parse_input_form("1\n2").is_err());
}
