//! Generator lattice normalization and exact expression-polynomial reconstruction.
use om_core::{BUILTIN as B, Expr, canonicalize, div, pow};
use om_num::{
    Integer, Rational,
    ctx::{Abort, Interrupt},
};
use om_parse::{Dialect, parse_expr};
use om_poly::{MPoly, MonoOrder, Monomial};
use om_simplify::convert::{
    from_mpoly, from_mpoly_with, to_rational_function, to_rational_function_with,
};
use proptest::prelude::*;
fn e(s: &str) -> Expr {
    canonicalize(&parse_expr(s, Dialect::Wolfram).unwrap())
}
fn q(n: usize, terms: &[(Vec<u32>, i64, i64)]) -> MPoly<Rational> {
    MPoly::new(
        n,
        terms
            .iter()
            .map(|(m, a, b)| {
                (
                    Monomial::new(m.clone()).unwrap(),
                    Rational::from(*a) / Rational::from(*b),
                )
            })
            .collect(),
        MonoOrder::Lex,
        &Interrupt::default(),
    )
    .unwrap()
}
#[test]
fn authority_fractional_powers_share_one_generator_and_base_relation() {
    let x = e("x");
    let v = to_rational_function(&e("Sqrt[x]+x^(1/3)"), std::slice::from_ref(&x));
    assert_eq!(v.gens, vec![e("x^(1/6)")]);
    assert_eq!(v.num, q(1, &[(vec![3], 1, 1), (vec![2], 1, 1)]));
    assert!(v.den.is_one());
    assert_eq!(pow(v.gens[0].clone(), Expr::int(6)), x);
    let v = to_rational_function(&e("x+Sqrt[x]+x^(1/3)+x^(-5/6)"), &[e("x")]);
    assert_eq!(v.gens, vec![e("x^(1/6)")]);
    assert_eq!(
        v.num,
        q(
            1,
            &[
                (vec![11], 1, 1),
                (vec![8], 1, 1),
                (vec![7], 1, 1),
                (vec![0], 1, 1)
            ]
        )
    );
    assert_eq!(v.den, q(1, &[(vec![5], 1, 1)]));
}
#[test]
fn authority_unreduced_fraction_and_common_denominator_views() {
    let v = to_rational_function(&e("(x^2-1)/(x-1)"), &[e("x")]);
    assert_eq!(v.num, q(1, &[(vec![2], 1, 1), (vec![0], -1, 1)]));
    assert_eq!(v.den, q(1, &[(vec![1], 1, 1), (vec![0], -1, 1)]));
    let v = to_rational_function(&e("1/x+1/y"), &[e("x"), e("y")]);
    assert_eq!(v.num, q(2, &[(vec![1, 0], 1, 1), (vec![0, 1], 1, 1)]));
    assert_eq!(v.den, q(2, &[(vec![1, 1], 1, 1)]));
    assert_eq!(
        div(from_mpoly(&v.num, &v.gens), from_mpoly(&v.den, &v.gens)),
        e("(x+y)/(x*y)")
    );
}
#[test]
fn transcendental_and_parameter_atoms_are_independent_with_deterministic_requested_axes() {
    let input = e("a*x+Sin[x]+E^x+E^(2*x)+Sqrt[2]");
    let v = to_rational_function(&input, &[e("x")]);
    assert_eq!(v.gens[0], e("x"));
    for generator in [e("a"), e("Sin[x]"), e("E^x"), e("E^(2*x)"), e("Sqrt[2]")] {
        assert!(v.gens.contains(&generator));
    }
    assert_eq!(v.gens.len(), 6);
    assert_eq!(
        div(from_mpoly(&v.num, &v.gens), from_mpoly(&v.den, &v.gens)),
        input
    );
    let ordered = to_rational_function(&e("x+y+a"), &[e("y"), e("x"), e("y")]);
    assert_eq!(ordered.gens, vec![e("y"), e("x"), e("a")]);
}
#[test]
fn raw_syntax_nested_rational_powers_and_arbitrary_function_bases_normalize() {
    let ctx = Interrupt::default();
    let raw = parse_expr("Sqrt[x]+x^(1/3)", Dialect::Wolfram).unwrap();
    let v = to_rational_function_with(&raw, &[e("x")], &ctx)
        .unwrap()
        .unwrap();
    assert_eq!(v.gens, vec![e("x^(1/6)")]);
    let v = to_rational_function(&e("Sin[x]^(2/3)+Sin[x]^(4/5)"), &[]);
    assert_eq!(v.gens, vec![e("Sin[x]^(1/15)")]);
    assert_eq!(v.num, q(1, &[(vec![12], 1, 1), (vec![10], 1, 1)]));
    let v = to_rational_function(&e("(x+y)^2+(x+y)^(1/2)"), &[]);
    assert_eq!(v.gens, vec![e("(x+y)^(1/2)")]);
    assert_eq!(v.num, q(1, &[(vec![4], 1, 1), (vec![1], 1, 1)]));
}
#[test]
fn exact_constants_machine_and_complex_atoms_and_empty_polynomials_roundtrip() {
    for source in ["0", "-7/3", "1.25", "I", "3+4*I"] {
        let input = e(source);
        let v = to_rational_function(&input, &[]);
        assert_eq!(
            div(from_mpoly(&v.num, &v.gens), from_mpoly(&v.den, &v.gens)),
            input
        );
    }
    let v = to_rational_function(&e("-7/3"), &[]);
    assert!(v.gens.is_empty());
    assert_eq!(v.num, q(0, &[(vec![], -7, 3)]));
    assert!(v.den.is_one());
    assert_eq!(from_mpoly(&q(2, &[]), &[e("x"), e("y")]), Expr::int(0));
}
#[test]
fn explicit_compound_generators_degree_bound_and_invalid_output_context_are_checked() {
    let ctx = Interrupt::default();
    let v = to_rational_function(&e("(x+y)^2"), &[e("x+y")]);
    assert_eq!(v.gens, vec![e("x+y")]);
    assert_eq!(v.num, q(1, &[(vec![2], 1, 1)]));
    assert!(
        from_mpoly_with(&q(2, &[(vec![1, 0], 1, 1)]), &[e("x")], &ctx)
            .unwrap()
            .is_none()
    );
    let input = Expr::call(B::POWER, [e("x"), Expr::integer(Integer::ONE << 40)]);
    assert!(
        to_rational_function_with(&input, &[e("x")], &ctx)
            .unwrap()
            .is_none()
    );
    let total = to_rational_function(&input, &[e("x")]);
    assert_eq!(from_mpoly(&total.num, &total.gens), canonicalize(&input));
}
#[test]
fn budgets_and_external_cancel_stop_conversion_and_reconstruction() {
    let ctx = Interrupt::default();
    let input = e("(x+y+z)^30/(x-y)^3");
    for budget in [0, 10, 100] {
        ctx.steps_left.set(budget);
        assert!(matches!(
            to_rational_function_with(&input, &[e("x"), e("y")], &ctx),
            Err(Abort::Budget)
        ));
    }
    ctx.steps_left.set(0);
    assert!(matches!(
        from_mpoly_with(&q(2, &[(vec![1, 0], 1, 1)]), &[e("x"), e("y")], &ctx),
        Err(Abort::Budget)
    ));
    ctx.flag.store(true, std::sync::atomic::Ordering::Relaxed);
    assert!(matches!(
        to_rational_function_with(&Expr::int(0), &[], &ctx),
        Err(Abort::Interrupted)
    ));
}
proptest! {
 #![proptest_config(ProptestConfig{cases:128,rng_seed:proptest::test_runner::RngSeed::Fixed(0x4d382e31),..ProptestConfig::default()})]
 #[test]
 fn rational_polynomial_views_roundtrip_terms_and_evaluate_exactly(c in prop::collection::vec(-5i64..=5,6),x in 1i64..=5,y in 1i64..=5){let ctx=Interrupt::default();let terms:Vec<_>=[vec![2,0],vec![1,1],vec![0,2],vec![1,0],vec![0,1],vec![0,0]].into_iter().zip(&c).map(|(m,c)|(m,*c,3)).collect();let p=q(2,&terms);let gens=[e("x"),e("y")];let expr=from_mpoly(&p,&gens);let v=to_rational_function_with(&expr,&gens,&ctx).unwrap().unwrap();prop_assert_eq!(&v.num,&p);prop_assert!(v.den.is_one());
 let numeric=expr.replace_all(&[(gens[0].clone(),Expr::int(x)),(gens[1].clone(),Expr::int(y))]);let expected=Rational::from(c[0]*x*x+c[1]*x*y+c[2]*y*y+c[3]*x+c[4]*y+c[5])/Rational::from(3);prop_assert_eq!(numeric,Expr::number(om_num::Number::Rational(expected)));
 }
}
