//! Groebner authority vectors and independent exhaustive-pair/rational reduction oracle.
use om_num::{
    Rational,
    ctx::{Abort, Interrupt},
};
use om_poly::{MPoly, MonoOrder, Monomial, groebner, normal_form, s_polynomial};
use proptest::prelude::*;
type Q = MPoly<Rational>;
fn q(n: usize, terms: &[(Vec<u32>, i64, i64)], order: MonoOrder) -> Q {
    MPoly::new(
        n,
        terms
            .iter()
            .map(|(e, a, b)| {
                (
                    Monomial::new(e.clone()).unwrap(),
                    Rational::from(*a) / Rational::from(*b),
                )
            })
            .collect(),
        order,
        &Interrupt::default(),
    )
    .unwrap()
}
fn div(a: &Monomial, b: &Monomial) -> Option<Monomial> {
    Monomial::new(
        a.exps
            .iter()
            .zip(&b.exps)
            .map(|(x, y)| x.checked_sub(*y))
            .collect::<Option<Vec<_>>>()?,
    )
}
fn shift(a: &Q, m: &Monomial, c: &Rational, ctx: &Interrupt) -> Q {
    MPoly::new(
        a.nvars,
        a.terms
            .iter()
            .map(|(n, a)| {
                (
                    Monomial::new(n.exps.iter().zip(&m.exps).map(|(x, y)| x + y)).unwrap(),
                    a * c,
                )
            })
            .collect(),
        a.order,
        ctx,
    )
    .unwrap()
}
fn oracle_nf(f: &Q, basis: &[Q], ctx: &Interrupt) -> Q {
    let mut p = f.clone();
    let mut out = Q::zero_in(f.nvars, f.order);
    while let Some((m, c)) = p.terms.first().cloned() {
        let reducer = basis.iter().find_map(|g| {
            g.terms
                .first()
                .and_then(|(n, b)| div(&m, n).map(|d| (g, d, &c / b)))
        });
        if let Some((g, m, c)) = reducer {
            p = p.sub(&shift(g, &m, &c, ctx), ctx).unwrap();
        } else {
            let t = Q::new(f.nvars, vec![(m, c)], f.order, ctx).unwrap();
            out = out.add(&t, ctx).unwrap();
            p = p.sub(&t, ctx).unwrap();
        }
    }
    out
}
fn monic(f: &Q, ctx: &Interrupt) -> Q {
    let Some((_, lc)) = f.terms.first() else {
        return f.clone();
    };
    Q::new(
        f.nvars,
        f.terms.iter().map(|(m, c)| (m.clone(), c / lc)).collect(),
        f.order,
        ctx,
    )
    .unwrap()
}
fn oracle_spoly(f: &Q, g: &Q, ctx: &Interrupt) -> Q {
    let (m, a) = f.terms.first().unwrap();
    let (n, b) = g.terms.first().unwrap();
    let l = Monomial::new(m.exps.iter().zip(&n.exps).map(|(x, y)| (*x).max(*y))).unwrap();
    shift(f, &div(&l, m).unwrap(), &(Rational::ONE / a), ctx)
        .sub(
            &shift(g, &div(&l, n).unwrap(), &(Rational::ONE / b), ctx),
            ctx,
        )
        .unwrap()
}
fn reduced(mut basis: Vec<Q>, ctx: &Interrupt) -> Vec<Q> {
    basis.retain(|f| !f.is_zero());
    let mut minimal = vec![];
    for (i, f) in basis.iter().enumerate() {
        if !basis.iter().enumerate().any(|(j, g)| {
            j != i
                && div(&f.terms[0].0, &g.terms[0].0).is_some()
                && (g.terms[0].0 != f.terms[0].0 || j < i)
        }) {
            minimal.push(f.clone());
        }
    }
    let mut out = vec![];
    for (i, f) in minimal.iter().enumerate() {
        let other: Vec<_> = minimal
            .iter()
            .enumerate()
            .filter(|(j, _)| *j != i)
            .map(|(_, g)| g.clone())
            .collect();
        out.push(monic(&oracle_nf(f, &other, ctx), ctx));
    }
    out.sort_by(|a, b| a.terms[0].0.cmp(&b.terms[0].0, a.order));
    out
}
fn oracle_basis(inputs: &[Q], ctx: &Interrupt) -> Vec<Q> {
    let mut basis: Vec<_> = inputs
        .iter()
        .filter(|f| !f.is_zero())
        .map(|f| monic(f, ctx))
        .collect();
    let mut pairs = vec![];
    for i in 0..basis.len() {
        for j in 0..i {
            pairs.push((j, i));
        }
    }
    let mut at = 0;
    while at < pairs.len() {
        let (i, j) = pairs[at];
        at += 1;
        let h = monic(
            &oracle_nf(&oracle_spoly(&basis[i], &basis[j], ctx), &basis, ctx),
            ctx,
        );
        if !h.is_zero() {
            let n = basis.len();
            for j in 0..n {
                pairs.push((j, n));
            }
            basis.push(h);
        }
    }
    reduced(basis, ctx)
}
fn check(inputs: &[Q], basis: &[Q], ctx: &Interrupt) {
    for f in inputs {
        assert!(oracle_nf(f, basis, ctx).is_zero());
        assert!(normal_form(f, basis, ctx).unwrap().unwrap().is_zero());
    }
    for i in 0..basis.len() {
        assert_eq!(basis[i].terms[0].1, Rational::ONE);
        let others: Vec<_> = basis
            .iter()
            .enumerate()
            .filter(|(j, _)| *j != i)
            .map(|(_, g)| g.clone())
            .collect();
        assert_eq!(oracle_nf(&basis[i], &others, ctx), basis[i]);
        for j in 0..i {
            assert!(oracle_nf(&oracle_spoly(&basis[i], &basis[j], ctx), basis, ctx).is_zero());
        }
    }
}
#[test]
fn authority_lex_vectors_and_unit_ideal_match_exactly() {
    let ctx = Interrupt::default();
    let l = MonoOrder::Lex;
    let fixtures = vec![
        (
            vec![
                q(
                    2,
                    &[(vec![2, 0], 1, 1), (vec![0, 2], 1, 1), (vec![0, 0], -1, 1)],
                    l,
                ),
                q(2, &[(vec![1, 0], 1, 1), (vec![0, 1], -1, 1)], l),
            ],
            vec![
                q(2, &[(vec![0, 2], 1, 1), (vec![0, 0], -1, 2)], l),
                q(2, &[(vec![1, 0], 1, 1), (vec![0, 1], -1, 1)], l),
            ],
        ),
        (
            vec![
                q(2, &[(vec![1, 1], 1, 1), (vec![0, 0], -1, 1)], l),
                q(2, &[(vec![2, 0], 1, 1), (vec![0, 1], -1, 1)], l),
            ],
            vec![
                q(2, &[(vec![0, 3], 1, 1), (vec![0, 0], -1, 1)], l),
                q(2, &[(vec![1, 0], 1, 1), (vec![0, 2], -1, 1)], l),
            ],
        ),
        (
            vec![
                q(
                    3,
                    &[
                        (vec![1, 0, 0], 1, 1),
                        (vec![0, 1, 0], 1, 1),
                        (vec![0, 0, 1], 1, 1),
                    ],
                    l,
                ),
                q(
                    3,
                    &[
                        (vec![1, 1, 0], 1, 1),
                        (vec![0, 1, 1], 1, 1),
                        (vec![1, 0, 1], 1, 1),
                    ],
                    l,
                ),
                q(3, &[(vec![1, 1, 1], 1, 1), (vec![0, 0, 0], -1, 1)], l),
            ],
            vec![
                q(3, &[(vec![0, 0, 3], 1, 1), (vec![0, 0, 0], -1, 1)], l),
                q(
                    3,
                    &[
                        (vec![0, 2, 0], 1, 1),
                        (vec![0, 1, 1], 1, 1),
                        (vec![0, 0, 2], 1, 1),
                    ],
                    l,
                ),
                q(
                    3,
                    &[
                        (vec![1, 0, 0], 1, 1),
                        (vec![0, 1, 0], 1, 1),
                        (vec![0, 0, 1], 1, 1),
                    ],
                    l,
                ),
            ],
        ),
        (
            vec![
                q(2, &[(vec![1, 0], 1, 1), (vec![0, 1], 1, 1)], l),
                q(
                    2,
                    &[(vec![1, 0], 1, 1), (vec![0, 1], 1, 1), (vec![0, 0], 1, 1)],
                    l,
                ),
            ],
            vec![q(2, &[(vec![0, 0], 1, 1)], l)],
        ),
    ];
    for (input, expected) in fixtures {
        let got = groebner(&input, l, &ctx).unwrap().unwrap();
        assert_eq!(got, expected);
        check(&input, &got, &ctx);
        assert_eq!(got, oracle_basis(&input, &ctx));
    }
}
#[test]
fn fraction_free_coefficients_and_order_conversion_preserve_ideal() {
    let ctx = Interrupt::default();
    let input = vec![
        q(
            2,
            &[(vec![2, 0], 2, 3), (vec![0, 1], -7, 5)],
            MonoOrder::Lex,
        ),
        q(2, &[(vec![1, 1], 9, 4), (vec![0, 0], 1, 6)], MonoOrder::Lex),
        q(
            2,
            &[(vec![0, 2], -1, 2), (vec![1, 0], 3, 7)],
            MonoOrder::Lex,
        ),
    ];
    for order in [MonoOrder::Lex, MonoOrder::GrevLex] {
        let fixed: Vec<_> = input
            .iter()
            .map(|f| Q::new(2, f.terms.clone(), order, &ctx).unwrap())
            .collect();
        let got = groebner(&input, order, &ctx).unwrap().unwrap();
        assert_eq!(got, oracle_basis(&fixed, &ctx));
        check(&fixed, &got, &ctx);
        let mut reversed = input.clone();
        reversed.reverse();
        assert_eq!(groebner(&reversed, order, &ctx).unwrap().unwrap(), got);
    }
}
#[test]
fn normal_form_is_exact_unscaled_and_spoly_cancels_leading_monomial() {
    let ctx = Interrupt::default();
    let order = MonoOrder::Lex;
    let f = q(
        2,
        &[(vec![2, 0], 1, 3), (vec![0, 1], 2, 1), (vec![0, 0], 1, 1)],
        order,
    );
    let g = q(2, &[(vec![1, 0], 3, 1), (vec![0, 0], -1, 1)], order);
    let got = normal_form(&f, std::slice::from_ref(&g), &ctx)
        .unwrap()
        .unwrap();
    assert_eq!(got, oracle_nf(&f, std::slice::from_ref(&g), &ctx));
    assert_eq!(
        got,
        q(2, &[(vec![0, 1], 2, 1), (vec![0, 0], 28, 27)], order)
    );
    assert_eq!(
        s_polynomial(&f, &g, &ctx).unwrap().unwrap(),
        oracle_spoly(&f, &g, &ctx)
    );
}
#[test]
fn empty_zero_constants_contexts_overflow_and_budget_are_checked() {
    let ctx = Interrupt::default();
    let order = MonoOrder::GrevLex;
    let zero = Q::zero_in(2, order);
    assert_eq!(groebner(&[], order, &ctx).unwrap(), Some(vec![]));
    assert_eq!(
        groebner(std::slice::from_ref(&zero), order, &ctx).unwrap(),
        Some(vec![])
    );
    assert_eq!(normal_form(&zero, &[], &ctx).unwrap(), Some(zero.clone()));
    let f = q(2, &[(vec![1, 1], 1, 1)], order);
    assert_eq!(
        groebner(&[zero, f.clone(), f.clone()], order, &ctx).unwrap(),
        Some(vec![f.clone()])
    );
    assert!(
        groebner(
            &[f.clone(), q(3, &[(vec![1, 0, 0], 1, 1)], order)],
            order,
            &ctx
        )
        .unwrap()
        .is_none()
    );
    let a = q(2, &[(vec![u32::MAX, 0], 1, 1)], order);
    let b = q(2, &[(vec![0, u32::MAX], 1, 1)], order);
    assert!(s_polynomial(&a, &b, &ctx).unwrap().is_none());
    for budget in [0, 10, 100] {
        ctx.steps_left.set(budget);
        assert!(matches!(
            groebner(
                &[
                    q(2, &[(vec![3, 0], 1, 1), (vec![0, 1], -1, 1)], order),
                    q(2, &[(vec![0, 3], 1, 1), (vec![1, 0], -1, 1)], order)
                ],
                order,
                &ctx
            ),
            Err(Abort::Budget)
        ));
    }
    ctx.flag.store(true, std::sync::atomic::Ordering::Relaxed);
    assert!(matches!(
        groebner(&[], order, &ctx),
        Err(Abort::Interrupted)
    ));
}
proptest! {
 #![proptest_config(ProptestConfig{cases:64,rng_seed:proptest::test_runner::RngSeed::Fixed(0x4d372e31),..ProptestConfig::default()})]
 #[test]
    fn filtered_sugar_basis_equals_exhaustive_pairs_for_small_random_systems(coeffs in prop::collection::vec(-3i64..=3,18),grev in any::<bool>()) {
  let ctx=Interrupt::default();let order=if grev{MonoOrder::GrevLex}else{MonoOrder::Lex};let exps=[vec![2,0],vec![1,1],vec![0,2],vec![1,0],vec![0,1],vec![0,0]];
  let input:Vec<_>=coeffs.chunks(6).map(|c|q(2,&exps.iter().zip(c).map(|(e,c)|(e.clone(),*c,1)).collect::<Vec<_>>(),order)).collect();
  let got=groebner(&input,order,&ctx).unwrap().unwrap();let reference=oracle_basis(&input,&ctx);prop_assert_eq!(&got,&reference);check(&input,&got,&ctx);
    }
    #[test]
    fn nonunit_planted_systems_match_exhaustive_pair_oracle(c in prop::collection::vec(-2i64..=2,7),r in -2i64..=2,s in -2i64..=2,grev in any::<bool>()) {
        let ctx=Interrupt::default();let order=if grev{MonoOrder::GrevLex}else{MonoOrder::Lex};
        let f=q(2,&[(vec![2,0],1,1),(vec![1,1],c[0],1),(vec![0,2],c[1],1),(vec![1,0],c[2],1),(vec![0,1],c[3],1),(vec![0,0],-(r*r+c[0]*r*s+c[1]*s*s+c[2]*r+c[3]*s),1)],order);
        let g=q(2,&[(vec![0,2],1,1),(vec![1,1],c[4],1),(vec![1,0],c[5],1),(vec![0,1],c[6],1),(vec![0,0],-(s*s+c[4]*r*s+c[5]*r+c[6]*s),1)],order);
        let input=vec![f,g];let got=groebner(&input,order,&ctx).unwrap().unwrap();
        prop_assert_eq!(&got,&oracle_basis(&input,&ctx));check(&input,&got,&ctx);
        prop_assert!(!got.iter().any(|g|g.terms.len()==1 && g.terms[0].0.deg==0));
    }
}
