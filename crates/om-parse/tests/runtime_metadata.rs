//! Executable aliases and strict named-option schema preserve user functions and source spans.
use om_core::{BUILTIN as B, Symbol};
use om_parse::{Dialect, ParseEnv, Severity, parse, parse_expr, parse_with};

#[test]
fn modern_aliases_resolve_actual_callbacks() {
    for (source, head) in [
        ("polynomial_gcd(x,x^2)", "PolynomialGCD"),
        ("algebraic_root(function(x),1)", "Root"),
        ("implicit_plot(x^2+y^2=1,[x,-2,2],[y,-2,2])", "ContourPlot"),
        ("solve_values(x=1,x)", "SolveValues"),
    ] {
        assert_eq!(
            parse_expr(source, Dialect::Modern).unwrap().head_symbol(),
            Some(Symbol::intern(head)),
            "{source}"
        );
    }
    assert!(
        parse_expr("root(8,3)", Dialect::Modern)
            .unwrap()
            .is_head(B::POWER)
    );
}
#[test]
fn named_options_are_function_specific_and_repairable() {
    for source in [
        "solve(x=1,x, precison: 20)",
        "sin(x, method: \"Newton\")",
        "find_root(x=1,[x,0],max_iterations: 0)",
        "solve(x=1,x,cubics: 17)",
        "find_root(x=1,[x,0],method: \"magic\")",
        "solve(x=1,x,cubics:true,cubics:false)",
    ] {
        assert!(
            parse(source, Dialect::Modern)
                .diagnostics
                .iter()
                .any(|d| d.severity == Severity::Error),
            "{source}"
        );
    }
    let parsed = parse("nsolve(x=1,x,precison:20)", Dialect::Modern);
    assert!(
        parsed
            .diagnostics
            .iter()
            .any(|d| d.fix.as_ref().is_some_and(|f| f.replacement == "precision"))
    );
    assert!(
        parse_expr(
            "find_root(x^2=2,[x,1],method: \"Newton\",max_iterations: 25)",
            Dialect::Modern
        )
        .is_ok()
    );
    assert!(parse_expr("custom(x, anything: 1)", Dialect::Modern).is_ok());
    // Wolfram parsing remains source compatible, with callback diagnostics after evaluation.
    assert!(parse_expr("Sin[x,Method->\"magic\"]", Dialect::Wolfram).is_ok());
}
#[test]
fn user_binding_beats_new_alias() {
    let mut env = ParseEnv::default();
    env.known_functions.insert(Symbol::intern("polynomial_gcd"));
    let parsed = parse_with("polynomial_gcd(3)", Dialect::Modern, &env);
    assert!(
        parsed.statements[0]
            .expr
            .is_head(Symbol::intern("polynomial_gcd"))
    );
}

#[test]
fn redeclaration_preserves_existing_alias_binding() {
    let mut env = ParseEnv::default();
    env.known_functions.insert(Symbol::intern("polynomial_gcd"));
    let parsed = parse_with("let polynomial_gcd(x)=x+2", Dialect::Modern, &env);
    assert!(parsed.statements[0].expr.args()[0].is_head(Symbol::intern("polynomial_gcd")));
}
