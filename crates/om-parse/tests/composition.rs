//! Modern composition extends syntax without changing old mathematical expectations.
use om_core::{BUILTIN as B, Expr, Symbol};
use om_parse::{Dialect, Severity, parse, parse_expr};
fn raw(source: &str) -> Expr {
    parse_expr(source, Dialect::Modern).unwrap_or_else(|d| panic!("{source}: {d:?}"))
}

#[test]
fn pipe_uses_metadata_position_and_associates_left() {
    let e = raw("[1,2,3] |> map(fn(x)=>x^2) |> first()");
    assert!(e.is_head(Symbol::intern("First")));
    let map = &e.args()[0];
    assert!(map.is_head(B::MAP));
    assert!(map.args()[0].is_head(B::FUNCTION));
    assert_eq!(map.args()[1], raw("[1,2,3]"));
    assert_eq!(
        raw("x+1 |> expand() |> factor()"),
        raw("factor(expand(x+1))")
    );
    assert_eq!(
        raw("x^2=1 |> solve(x,domain:reals)"),
        raw("solve(x^2=1,x,domain:reals)")
    );
    for s in ["x |> clear()", "x |> unknown()", "x |> 2"] {
        assert!(
            parse(s, Dialect::Modern)
                .diagnostics
                .iter()
                .any(|d| d.severity == Severity::Error)
        );
    }
}
#[test]
fn lambda_is_contextual_and_keeps_lexical_parameter_names() {
    assert_eq!(
        raw("fn(sin,x)=>sin+x"),
        Expr::call(
            B::FUNCTION,
            [
                Expr::call(B::LIST, [Expr::symbol("sin"), Expr::symbol("x")]),
                raw("sin+x")
            ]
        )
    );
    assert!(raw("fn(x)=>fn(y)=>x+y").args()[1].is_head(B::FUNCTION));
    assert_eq!(raw("fn(1)").head_symbol(), Some(Symbol::intern("fn")));
    assert!(raw("(fn(x)=>x+1)(2)").head().is_head(B::FUNCTION));
    assert!(
        parse("fn(x,x)=>x", Dialect::Modern)
            .diagnostics
            .iter()
            .any(|d| d.severity == Severity::Error)
    );
}
#[test]
fn ranges_decimal_boundaries_and_slices_are_unambiguous() {
    assert_eq!(
        raw("1..3"),
        Expr::call(B::SPAN, [Expr::int(1), Expr::int(3)])
    );
    assert_eq!(
        raw("1+2..5-1"),
        Expr::call(B::SPAN, [raw("1+2"), raw("5-1")])
    );
    assert_eq!(
        raw("v[2..4]"),
        Expr::call(
            B::PART,
            [
                Expr::symbol("v"),
                Expr::call(B::SPAN, [Expr::int(2), Expr::int(4)])
            ]
        )
    );
    assert_eq!(raw("1.23..2.34").args()[0], raw("1.23"));
    assert!(
        parse("1..2..3", Dialect::Modern)
            .diagnostics
            .iter()
            .any(|d| d.severity == Severity::Error)
    );
    assert_eq!(
        raw("A @ b"),
        Expr::call(
            Symbol::intern("Dot"),
            [Expr::symbol("A"), Expr::symbol("b")]
        )
    );
    // Wolfram @ keeps prefix application.
    assert_eq!(
        parse_expr("f @ x", Dialect::Wolfram).unwrap(),
        Expr::call(Symbol::intern("f"), [Expr::symbol("x")])
    );
}
#[test]
fn records_use_literal_keys_keep_order_and_preserve_brace_lists() {
    let e = raw("{color:\"green\", α:2}");
    assert!(e.is_head(Symbol::intern("Record")));
    assert_eq!(
        e.args()[0],
        Expr::call(B::RULE, [Expr::string("color"), Expr::string("green")])
    );
    assert_eq!(e.args()[1].args()[0], Expr::string("α"));
    assert_eq!(
        raw("config.color"),
        Expr::call(B::PART, [Expr::symbol("config"), Expr::string("color")])
    );
    assert!(raw("{}").is_head(B::LIST));
    assert!(raw("{a,b}").is_head(B::LIST));
    assert!(raw("record()").is_head(Symbol::intern("Record")));
    for s in ["{a:1,a:2}", "{\"a\":1,a:2}", "{a:1,2}"] {
        assert!(
            parse(s, Dialect::Modern)
                .diagnostics
                .iter()
                .any(|d| d.severity == Severity::Error)
        );
    }
}

#[test]
fn actual_modes_and_named_axes_lower_to_existing_held_protocol() {
    assert_eq!(
        raw("solve(x=1,x,precision:20,mode:\"numeric\",output:\"values\")"),
        raw("nsolve_values(x=1,x,precision:20)")
    );
    assert_eq!(
        raw("plot(sin(x),x:-pi..pi)"),
        raw("plot(sin(x),[x,-pi,pi])")
    );
    assert_eq!(
        raw("plot(x^2+y^2=1,x:-2..2,y:-2..2,view:\"contour\")"),
        raw("implicit_plot(x^2+y^2=1,[x,-2,2],[y,-2,2])")
    );
    for source in [
        "solve(x=1,x,mode:\"unknown\")",
        "solve(x=1,x,mode:\"exact\",precision:20)",
        "find_root(x=1,x,initial:0,bracket:0..2)",
        "plot(x,x:0..1,view:\"surface\")",
        "diff(x,x,x,order:2)",
    ] {
        assert!(
            parse(source, Dialect::Modern)
                .diagnostics
                .iter()
                .any(|d| d.severity == Severity::Error),
            "{source}"
        );
    }
}
