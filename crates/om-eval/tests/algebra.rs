//! Working algebra builtins execute through the shared evaluation engine.
use om_core::{BUILTIN as B, Expr, Interrupt, Symbol, canonicalize};
use om_eval::Evaluator;
use om_parse::{Dialect, parse_expr};
fn raw(s: &str) -> Expr {
    parse_expr(s, Dialect::Wolfram).unwrap()
}
fn e(s: &str) -> Expr {
    canonicalize(&raw(s))
}
fn eval(s: &str) -> Expr {
    Evaluator::new()
        .evaluate(&raw(s), &Interrupt::default())
        .unwrap()
}
fn same(src: &str, want: &str) {
    assert_eq!(eval(src), e(want), "{src}");
}
fn equivalent(src: &str, want: &str) {
    let got = eval(src);
    assert!(
        !got.is_head(raw(src).head_symbol().unwrap()),
        "not evaluated {src}"
    );
    assert_eq!(
        om_simplify::zero::is_zero(&om_core::sub(got, e(want))),
        om_simplify::zero::Tri::Zero,
        "{src}"
    );
}
#[test]
fn checked_arithmetic_transforms_and_safe_simplification() {
    same("Expand[(x+1)^3]", "x^3+3x^2+3x+1");
    same("Factor[x^2-y^2]", "(x-y)(x+y)");
    same("Together[1/x+1/y]", "(x+y)/(x y)");
    same("Cancel[(x^2-1)/(x-1)]", "x+1");
    same("Simplify[Sin[x]^2+Cos[x]^2]", "1");
    same("FullSimplify[(Sqrt[2]+Sqrt[3])*(Sqrt[3]-Sqrt[2])]", "1");
    same("Simplify[Sqrt[x^2],x>0]", "x");
    assert_ne!(eval("Simplify[Sqrt[x^2]]"), e("x"));
    same("Expand[{(x+1)^2,(y+1)^2}]", "{x^2+2x+1,y^2+2y+1}");
}
#[test]
fn polynomial_queries_keep_parameters_and_axes() {
    same("Coefficient[(a+b)x^2+3x+1,x,2]", "a+b");
    same("Coefficient[(x+y)^3,x]", "3y^2");
    same("Coefficient[7,x,0]", "7");
    same("CoefficientList[a x^2+b x+c,x]", "{c,b,a}");
    same("CoefficientList[x^2+y,{x,y}]", "{{0,1},{0,0},{1,0}}");
    same("CoefficientList[0,x]", "{}");
    same("Exponent[(x+1)^5,x]", "5");
    same("Exponent[0,x]", "DirectedInfinity[-1]");
    same("PolynomialQ[x^2+a/x,x]", "False");
    same("PolynomialQ[Sin[x]+x,x]", "False");
    same("PolynomialQ[Sqrt[x],x]", "False");
    same("PolynomialQ[a x^2+Sin[a] x+1,x]", "True");
    same("PolynomialQ[x y,{x,y}]", "True");
    same("Variables[x^2+2x y+y^2]", "{x,y}");
    same("Variables[7]", "{}");
    same("Collect[a x^2+b x^2+x+y,x]", "(a+b)x^2+x+y");
}
#[test]
fn polynomial_arithmetic_uses_exact_content_and_parameter_coefficients() {
    same("PolynomialGCD[2x^2-2,4x^2+8x+4]", "2x+2");
    same("PolynomialGCD[x^2-y^2,x^2+2x y+y^2]", "x+y");
    equivalent("PolynomialLCM[x^2-1,(x+1)^2]", "(x-1)(x+1)^2");
    same("PolynomialGCD[x/2,x/3]", "x/6");
    same("PolynomialQuotient[x^3-2x^2-4,x-3,x]", "x^2+x+3");
    same("PolynomialRemainder[x^3-2x^2-4,x-3,x]", "5");
    equivalent("PolynomialQuotient[a x^2+b x+c,x+d,x]", "a x+b-a d");
    equivalent("PolynomialRemainder[a x^2+b x+c,x+d,x]", "c-b d+a d^2");
    same("Resultant[x^2-a,x-b,x]", "b^2-a");
    same("Resultant[x^2/2-1,x-3,x]", "7/2");
    same("Discriminant[a x^2+b x+c,x]", "b^2-4a c");
    same("Discriminant[x^2/2-1,x]", "2");
}
#[test]
fn apart_restores_proper_and_improper_repeated_factor_fractions() {
    same("Apart[1/(x(x+1)),x]", "1/x-1/(x+1)");
    same("Apart[(x+2)/(x+1)^2,x]", "1/(x+1)+1/(x+1)^2");
    same("Apart[(x^3+1)/(x^2-1),x]", "x+1/(x-1)");
    equivalent("Apart[1/((x+1)*(x^2+1)),x]", "1/(2(x+1))+(1-x)/(2(x^2+1))");
    same("Apart[1/(x(x+1))]", "1/x-1/(x+1)");
    assert!(eval("Apart[1/(x+y),x]").is_head(Symbol::intern("Apart")));
}
#[test]
fn differentiation_covers_chain_mixed_orders_lists_and_formal_functions() {
    equivalent("D[(x+1)^3,x]", "3(x+1)^2");
    equivalent("D[Sin[x^2] Exp[x],x]", "Exp[x]*(2x Cos[x^2]+Sin[x^2])");
    equivalent("D[x^x,x]", "x^x*(1+Log[x])");
    same("D[x^4,{x,2}]", "12x^2");
    same("D[x^2 y^3,x,y]", "6x y^2");
    same("D[{x^2,Sin[x]},x]", "{2x,Cos[x]}");
    same("D[f[x^2],x]", "2x Derivative[1][f][x^2]");
    same("D[g[x,y],x]", "Derivative[1,0][g][x,y]");
    equivalent("D[ProductLog[x],x]", "ProductLog[x]/(x*(1+ProductLog[x]))");
    assert!(eval("D[x,1]").is_head(Symbol::intern("D")));
}
#[test]
fn root_reduce_uses_the_correct_minimal_polynomial_and_root_index() {
    same("RootReduce[Sqrt[2]+Sqrt[3]]", "Root[#^4-10#^2+1&,4]");
    same("RootReduce[Sqrt[2]*Sqrt[2]]", "2");
    same("RootReduce[-Sqrt[2]]", "Root[#^2-2&,1]");
    same(
        "RootReduce[{Sqrt[2],-Sqrt[2]}]",
        "{Root[#^2-2&,2],Root[#^2-2&,1]}",
    );
    same("ToRadicals[Root[#^2-2&,1]]", "-Sqrt[2]");
    same(
        "ToRadicals[{Root[#^3-2&,1],Root[#^2-2&,2]}]",
        "{2^(1/3),Sqrt[2]}",
    );
    let want = e("Root[1-# +#^5&,1]");
    assert_eq!(eval("ToRadicals[Root[1-# +#^5&,1]]"), want);
}
#[test]
fn every_algebra_entry_has_docs_protection_and_real_arity_validation() {
    for name in [
        "Expand",
        "Factor",
        "Together",
        "Cancel",
        "Apart",
        "Simplify",
        "FullSimplify",
        "Collect",
        "Coefficient",
        "CoefficientList",
        "Exponent",
        "PolynomialQ",
        "PolynomialGCD",
        "PolynomialLCM",
        "PolynomialQuotient",
        "PolynomialRemainder",
        "Resultant",
        "Discriminant",
        "Variables",
        "D",
        "RootReduce",
        "ToRadicals",
    ] {
        let sym = Symbol::intern(name);
        let doc = Evaluator::doc(sym).expect(name);
        assert!(
            !doc.summary_zh.is_empty() && !doc.summary_en.is_empty() && !doc.examples.is_empty(),
            "{name}"
        );
        let mut ev = Evaluator::new();
        ev.evaluate(&raw(&format!("{name}=7")), &Interrupt::default())
            .unwrap();
        assert_eq!(
            ev.evaluate(&Expr::sym(sym), &Interrupt::default()).unwrap(),
            Expr::sym(sym)
        );
        let call = Expr::call(sym, []);
        assert_eq!(ev.evaluate(&call, &Interrupt::default()).unwrap(), call);
        assert!(
            ev.messages
                .take()
                .iter()
                .any(|m| m.tag == "argx" || m.tag == "argrx")
        );
    }
}
#[test]
fn algebra_calls_compose_with_definitions_modern_syntax_and_readonly_state() {
    let mut ev = Evaluator::new();
    ev.evaluate(&raw("a=3"), &Interrupt::default()).unwrap();
    assert_eq!(
        ev.evaluate(&raw("Coefficient[a x^2,x,2]"), &Interrupt::default())
            .unwrap(),
        Expr::int(3)
    );
    let mut fork = ev.fork_readonly();
    assert_eq!(
        fork.evaluate(&raw("Expand[(x+1)^2]"), &Interrupt::default())
            .unwrap(),
        e("x^2+2x+1")
    );
    let modern = parse_expr("Expand((x+1)^2)", Dialect::Modern).unwrap();
    assert_eq!(
        ev.evaluate(&modern, &Interrupt::default()).unwrap(),
        e("x^2+2x+1")
    );
    let ctx = Interrupt::default();
    ctx.steps_left.set(10);
    assert!(ev.evaluate(&raw("Expand[(x+1)^100]"), &ctx).is_err());
    assert!(ev.last_steps.is_none());
    assert_eq!(eval("PolynomialQ[x+y,{x,x}]"), e("False"));
    assert!(eval("PolynomialQuotient[x,0,x]").is_head(Symbol::intern("PolynomialQuotient")));
    assert!(eval("Simplify[x,Assumptions->x<0]").is_head(Symbol::intern("Simplify")));
    assert_ne!(eval("Cancel[1/(x+1)]"), Expr::sym(B::INDETERMINATE));
}

#[test]
fn zero_constant_parameter_denominator_and_higher_derivative_edges() {
    for (src, want) in [
        ("PolynomialGCD[0,0]", "0"),
        ("PolynomialGCD[0,-2x]", "2x"),
        ("PolynomialLCM[0,x]", "0"),
        ("PolynomialLCM[x/2,x/3]", "x"),
        ("PolynomialLCM[x/2,x/3,x/5]", "x"),
        ("PolynomialGCD[-x]", "x"),
        ("PolynomialQuotient[1,x,x]", "0"),
        ("PolynomialRemainder[0,x,x]", "0"),
        ("PolynomialQuotient[x^2,2,x]", "x^2/2"),
        ("Resultant[2,x^2+1,x]", "4"),
        ("Discriminant[5,x]", "0"),
        ("Discriminant[a x+b,x]", "1"),
        ("Coefficient[x/y,x]", "1/y"),
        ("Collect[(a x+b x)/c,x]", "(a+b)x/c"),
        ("Apart[2/(2x+2),x]", "1/(x+1)"),
        ("Apart[1/((2x+1)(x+1)),x]", "2/(2x+1)-1/(x+1)"),
        ("D[f[x],{x,2}]", "Derivative[2][f][x]"),
        ("D[f[x,y],x,y]", "Derivative[1,1][f][x,y]"),
        ("D[{1,2},x]", "{0,0}"),
        ("D[x,{x,0}]", "x"),
        ("Collect[(x^2-1)/(x-1)*y+x*y,y,Cancel]", "(2x+1)y"),
    ] {
        same(src, want);
    }
    equivalent("Resultant[(x^2-a)/b,x-c,x]", "(c^2-a)/b");
    equivalent("Discriminant[(a x^2+b x+c)/d,x]", "(b^2-4a c)/d^2");
    let positive = eval("RootReduce[Sqrt[2]]");
    assert_eq!(eval("RootReduce[RootReduce[Sqrt[2]]]"), positive);
    let complex = eval("ToRadicals[Root[#^2+1&,2]]");
    assert_eq!(complex, e("I"));
    for src in [
        "Coefficient[Sin[x],x]",
        "CoefficientList[x^-1,x]",
        "D[x,{x,-1}]",
        "D[x,{x,10000}]",
        "Resultant[Sin[x],x,x]",
        "PolynomialGCD[x/2^x,x]",
        "Apart[Sin[x],x]",
    ] {
        assert!(eval(src).is_head(raw(src).head_symbol().unwrap()), "{src}");
    }
}

#[test]
fn fixed_seed_polynomial_identities_check_every_builtin_against_known_constructions() {
    let mut rng = om_num::rng::SplitMix64::new(0x414c4745425241);
    for _ in 0..24 {
        let r = rng.next_range(1, 9) as i64;
        let a = rng.next_range(1, 6) as i64;
        let b = rng.next_range(1, 7) as i64;
        let s = rng.next_range(1, 5) as i64;
        let h = format!("{a}x^2+{b}x+1");
        let f = format!("(x-{r})({h})+{s}");
        equivalent(&format!("PolynomialQuotient[{f},x-{r},x]"), &h);
        same(&format!("PolynomialRemainder[{f},x-{r},x]"), &s.to_string());
        same(&format!("Resultant[x-{r},{f},x]"), &s.to_string());
        same(
            &format!("Discriminant[{a}x^2+{b}x+{s},x]"),
            &(b * b - 4 * a * s).to_string(),
        );
        equivalent(
            &format!("PolynomialGCD[2(x-{r})(x+1),4(x-{r})(x+2)]"),
            &format!("2(x-{r})"),
        );
        equivalent(
            &format!("PolynomialLCM[(x-{r})(x+1),(x-{r})(x+2)]"),
            &format!("(x-{r})(x+1)(x+2)"),
        );
        equivalent(
            &format!("Apart[1/((x-{r})(x+1)),x]"),
            &format!("(1/(x-{r})-1/(x+1))/({r}+1)"),
        );
        equivalent(&format!("D[{h},x]"), &format!("2*{a}x+{b}"));
    }
}

#[test]
fn elementary_derivatives_match_independent_centered_numeric_differences() {
    for (name, x) in [
        ("Sin", 0.4),
        ("Cos", 0.4),
        ("Tan", 0.4),
        ("Cot", 0.4),
        ("Sec", 0.4),
        ("Csc", 0.4),
        ("ArcSin", 0.4),
        ("ArcCos", 0.4),
        ("ArcTan", 0.4),
        ("ArcCot", 0.4),
        ("ArcSec", 2.0),
        ("ArcCsc", 2.0),
        ("Sinh", 0.4),
        ("Cosh", 0.4),
        ("Tanh", 0.4),
        ("Coth", 0.4),
        ("Sech", 0.4),
        ("Csch", 0.4),
        ("ArcSinh", 0.4),
        ("ArcCosh", 2.0),
        ("ArcTanh", 0.4),
        ("Log", 0.4),
        ("Exp", 0.4),
        ("Sqrt", 0.4),
        ("ProductLog", 0.4),
    ] {
        let value = |at: f64| {
            om_simplify::numeval::approximate(
                &e(&format!("{name}[{at}]")),
                om_num::Precision::Machine,
                &Interrupt::default(),
            )
            .unwrap()
            .unwrap()
            .to_f64()
            .unwrap()
        };
        let h = 1e-5;
        let difference = (value(x + h) - value(x - h)) / (2.0 * h);
        let derivative = eval(&format!("D[{name}[x],x]")).replace_all(&[(e("x"), Expr::real(x))]);
        let derivative = om_simplify::numeval::approximate(
            &derivative,
            om_num::Precision::Machine,
            &Interrupt::default(),
        )
        .unwrap()
        .expect(name)
        .to_f64()
        .unwrap();
        assert!(
            (derivative - difference).abs() < 1e-7,
            "{name}: {derivative} vs {difference}"
        );
    }
}

#[test]
fn full_simplify_adds_real_radical_work_and_root_conversion_preserves_branches() {
    same("ToRadicals[Root[#^2-2&,2]]", "Sqrt[2]");
    same("FullSimplify[Root[#^2+1&,2]]", "I");
    same("RootReduce[3+4I]", "Root[#^2-6#+25&,2]");
    for src in ["Root[#^3+# +1&,1]", "Root[#^4+# +1&,1]"] {
        let original = e(src);
        let radical = eval(&format!("ToRadicals[{src}]"));
        assert!(!radical.is_head(B::ROOT), "{src}");
        let difference = om_core::sub(radical, original);
        let z = om_simplify::numeval::enclose(&difference, 700, &Interrupt::default())
            .unwrap()
            .unwrap();
        let bound = om_num::BigFloat::from_parts(1.into(), -400);
        assert!(
            z.re.contains_zero() && z.im.contains_zero() && z.re.rad < bound && z.im.rad < bound,
            "{src}"
        );
    }
}

#[test]
fn root_conversions_visit_compound_heads_and_keep_symbolic_axes() {
    same("ToRadicals[f[Root[#^2-2&,2]][x]]", "f[Sqrt[2]][x]");
    same("RootReduce[f[Sqrt[2]][x]]", "f[Root[#^2-2&,2]][x]");
    same("RootReduce[x+Sqrt[2]]", "x+Root[#^2-2&,2]");
}
