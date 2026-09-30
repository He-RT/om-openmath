//! M4.4 scalar and Boolean builtin batch, with real docs and arity checks.
use om_core::{BUILTIN as B, Expr, Interrupt, canonicalize};
use om_eval::Evaluator;
use om_parse::{Dialect, parse_expr};
fn src(s: &str) -> Expr {
    parse_expr(s, Dialect::Wolfram).unwrap()
}
fn eval(s: &str) -> Expr {
    Evaluator::new()
        .evaluate(&src(s), &Interrupt::default())
        .unwrap()
}
fn cases() -> Vec<(&'static str, &'static str)> {
    vec![
        ("Plus[1,2]", "3"),
        ("Plus[]", "0"),
        ("Times[2,3]", "6"),
        ("Times[]", "1"),
        ("Power[2,3]", "8"),
        ("Power[x,2]", "x^2"),
        ("Subtract[5,2]", "3"),
        ("Subtract[x,1]", "x-1"),
        ("Divide[6,4]", "3/2"),
        ("Divide[x,2]", "x/2"),
        ("Minus[3]", "-3"),
        ("Minus[x]", "-x"),
        ("Sqrt[12]", "2*Sqrt[3]"),
        ("Sqrt[4]", "2"),
        ("Exp[0]", "1"),
        ("Exp[x]", "E^x"),
        ("Log[1]", "0"),
        ("Log[E]", "1"),
        ("Log[2,8]", "3"),
        ("Log[-2]", "Log[2]+I*Pi"),
        ("Abs[-3]", "3"),
        ("Abs[3+4*I]", "5"),
        ("Sign[-3]", "-1"),
        ("Sign[0]", "0"),
        ("Re[3+4*I]", "3"),
        ("Re[2]", "2"),
        ("Im[3+4*I]", "4"),
        ("Im[2]", "0"),
        ("Conjugate[3+4*I]", "3-4*I"),
        ("Conjugate[2]", "2"),
        ("Arg[3]", "0"),
        ("Arg[-3]", "Pi"),
        ("Arg[I]", "Pi/2"),
        ("Floor[-3/2]", "-2"),
        ("Floor[2.9]", "2"),
        ("Ceiling[-3/2]", "-1"),
        ("Ceiling[2.1]", "3"),
        ("Round[5/2]", "2"),
        ("Round[7/2]", "4"),
        ("Round[-5/2]", "-2"),
        ("Round[9,2]", "8"),
        ("Mod[-7,3]", "2"),
        ("Mod[7,-3]", "-2"),
        ("Quotient[-7,3]", "-3"),
        ("Quotient[7,3]", "2"),
        ("GCD[12,18]", "6"),
        ("GCD[-12,0]", "12"),
        ("GCD[]", "0"),
        ("GCD[1/3,2/5]", "1/15"),
        ("LCM[4,6]", "12"),
        ("LCM[0,6]", "0"),
        ("LCM[]", "1"),
        ("Factorial[5]", "120"),
        ("Factorial[0]", "1"),
        ("Binomial[5,2]", "10"),
        ("Binomial[-3,2]", "6"),
        ("FactorInteger[-12]", "{{-1,1},{2,2},{3,1}}"),
        ("FactorInteger[1]", "{}"),
        ("PrimeQ[97]", "True"),
        ("PrimeQ[91]", "False"),
        ("Numerator[2/3]", "2"),
        ("Numerator[4]", "4"),
        ("Denominator[2/3]", "3"),
        ("Denominator[4]", "1"),
        ("Equal[1,1.]", "True"),
        ("Equal[1,2]", "False"),
        ("Equal[x,x]", "True"),
        ("Unequal[1,2,3]", "True"),
        ("Unequal[1,2,1]", "False"),
        ("Less[1,2,3]", "True"),
        ("Less[3,2]", "False"),
        ("LessEqual[1,1]", "True"),
        ("LessEqual[2,1]", "False"),
        ("Greater[3,2,1]", "True"),
        ("Greater[1,2]", "False"),
        ("GreaterEqual[1,1]", "True"),
        ("GreaterEqual[1,2]", "False"),
        ("And[True,True]", "True"),
        ("And[True,x]", "x"),
        ("Or[False,False]", "False"),
        ("Or[False,x]", "x"),
        ("Not[True]", "False"),
        ("Not[False]", "True"),
        ("SameQ[1,1]", "True"),
        ("SameQ[1,1.]", "False"),
        ("Sin[Pi/6]", "1/2"),
        ("Sin[-Pi/6]", "-1/2"),
        ("Cos[Pi/3]", "1/2"),
        ("Cos[Pi]", "-1"),
        ("Tan[Pi/4]", "1"),
        ("Tan[Pi/3]", "Sqrt[3]"),
        ("ArcSin[1/2]", "Pi/6"),
        ("ArcCos[1/2]", "Pi/3"),
        ("ArcTan[1]", "Pi/4"),
        ("ProductLog[0]", "0"),
        ("ProductLog[E]", "1"),
        ("0<1<=2", "True"),
        ("1==2==3", "False"),
    ]
}
#[test]
fn scalar_and_boolean_examples_have_at_least_two_per_implemented_function() {
    for (input, output) in cases() {
        assert_eq!(eval(input), canonicalize(&src(output)), "{input}");
    }
    assert_eq!(eval("x<y"), src("x<y"));
    assert_eq!(eval("x==y"), src("x==y"));
    assert_eq!(eval("3>2>1"), Expr::sym(B::TRUE));
}
#[test]
fn scalar_docs_arity_and_domain_rejection_match_functionality() {
    let names = [
        "Plus",
        "Times",
        "Power",
        "Subtract",
        "Divide",
        "Minus",
        "Sqrt",
        "Exp",
        "Log",
        "Abs",
        "Sign",
        "Re",
        "Im",
        "Conjugate",
        "Arg",
        "Floor",
        "Ceiling",
        "Round",
        "Mod",
        "Quotient",
        "GCD",
        "LCM",
        "Factorial",
        "Binomial",
        "FactorInteger",
        "PrimeQ",
        "Numerator",
        "Denominator",
        "Equal",
        "Unequal",
        "Less",
        "LessEqual",
        "Greater",
        "GreaterEqual",
        "And",
        "Or",
        "Not",
        "SameQ",
        "Sin",
        "Cos",
        "Tan",
        "ArcSin",
        "ArcCos",
        "ArcTan",
        "ProductLog",
    ];
    for name in names {
        let doc = Evaluator::doc(om_core::Symbol::intern(name)).unwrap();
        assert!(
            !doc.summary_zh.is_empty() && !doc.summary_en.is_empty() && !doc.examples.is_empty(),
            "{name}"
        );
    }
    let mut ev = Evaluator::new();
    let raw = src("Power[2]");
    assert_eq!(ev.evaluate(&raw, &Interrupt::default()).unwrap(), raw);
    assert!(
        ev.messages
            .take()
            .iter()
            .any(|m| m.symbol == "Power" && m.tag == "argx")
    );
    for input in [
        "Factorial[-1]",
        "Binomial[3,1/2]",
        "Floor[x]",
        "Round[x]",
        "Mod[3,0]",
        "Quotient[3,0]",
    ] {
        assert_eq!(
            ev.evaluate(&src(input), &Interrupt::default()).unwrap(),
            canonicalize(&src(input)),
            "{input}"
        );
    }
}
#[test]
fn boolean_short_circuiting_preserves_side_effect_order() {
    let mut ev = Evaluator::new();
    for input in ["False && (x=1)", "True || (x=2)"] {
        ev.evaluate(&src(input), &Interrupt::default()).unwrap();
    }
    assert_eq!(
        ev.evaluate(&src("x"), &Interrupt::default()).unwrap(),
        Expr::symbol("x")
    );
    assert_eq!(eval("And[]"), Expr::sym(B::TRUE));
    assert_eq!(eval("Or[]"), Expr::sym(B::FALSE));
}
#[test]
fn threaded_scalar_functions_and_pattern_conditions_share_evaluation() {
    assert_eq!(eval("Floor[{1.5,-1.5}]"), src("{1,-2}"));
    assert_eq!(eval("Sin[{0,Pi/6}]"), canonicalize(&src("{0,1/2}")));
    assert_eq!(eval("Sign[{-2,0,3}]"), src("{-1,0,1}"));
    let mut ev = Evaluator::new();
    ev.evaluate(&src("f[x_]/;x>0:=x^2"), &Interrupt::default())
        .unwrap();
    assert_eq!(
        ev.evaluate(&src("f[3]"), &Interrupt::default()).unwrap(),
        Expr::int(9)
    );
    assert_eq!(
        ev.evaluate(&src("f[-3]"), &Interrupt::default()).unwrap(),
        src("f[-3]")
    );
}
