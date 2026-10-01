//! Ferrari via a certified positive resolvent root and two exact quadratic factors.
use super::super::{Candidate, extract, numeric, reductions};
use super::cubic::qexpr;
use crate::{Formula, Level, SolveError, SolveOptions, Step, StepKind, StepSink};
use om_core::{Expr, add, div, mul, neg, pow, sqrt, sub};
use om_num::{Rational, ctx::Interrupt};
use om_poly::UPoly;

pub(in crate::univariate) fn ferrari(
    c: &[Expr],
    original: &Expr,
    x: &Expr,
    opts: &SolveOptions,
    ctx: &Interrupt,
    sink: &mut impl StepSink,
) -> Result<Vec<Candidate>, SolveError> {
    ctx.tick()?;
    let c = c
        .iter()
        .map(extract::exact)
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| SolveError::Unsupported("Ferrari requires rational coefficients".into()))?;
    let a = &c[4];
    let (b, d, e, f) = (&c[3] / a, &c[2] / a, &c[1] / a, &c[0] / a);
    let shift = &b / Rational::from(4);
    let p = &d - Rational::from(3) * &b * &b / Rational::from(8);
    let q = &e - &b * &d / Rational::from(2) + &b * &b * &b / Rational::from(8);
    let r = &f - &b * &e / Rational::from(4) + &b * &b * &d / Rational::from(16)
        - Rational::from(3) * &b * &b * &b * &b / Rational::from(256);
    let y = reductions::fresh(original, x, ctx)?;
    let coefficients = [
        qexpr(r.clone()),
        qexpr(q.clone()),
        qexpr(p.clone()),
        Expr::int(0),
        Expr::int(1),
    ];
    let depressed = reductions::polynomial(&coefficients, &y, ctx)?;
    sink.record(|| {
        Step::new(
            StepKind::Substitute {
                new_var: y.clone(),
                def: add([x.clone(), qexpr(shift.clone())]),
            },
            vec![original.clone()],
            vec![depressed.clone()],
            Level::Major,
        )
    });
    let mut roots = if q == Rational::ZERO {
        reductions::roots(&coefficients, &depressed, &y, opts, ctx, sink)?.ok_or_else(|| {
            SolveError::Unsupported("depressed biquadratic reduction failed".into())
        })?
    } else {
        let z = reductions::fresh(&add([original.clone(), y.clone()]), &y, ctx)?;
        let resolvent = UPoly::new(vec![
            -&q * &q,
            &p * &p - Rational::from(4) * &r,
            Rational::from(2) * &p,
            Rational::ONE,
        ]);
        let expr = reductions::polynomial(
            &resolvent
                .coeffs
                .iter()
                .cloned()
                .map(qexpr)
                .collect::<Vec<_>>(),
            &z,
            ctx,
        )?;
        sink.record(|| {
            Step::new(
                StepKind::Eliminant {
                    var: z.clone(),
                    poly: expr.clone(),
                },
                vec![depressed.clone()],
                vec![expr.clone()],
                Level::Major,
            )
        });
        let resolvent_opts = SolveOptions {
            cubics: true,
            ..opts.clone()
        };
        let candidates = numeric(&resolvent, &expr, &z, &resolvent_opts, ctx, sink)?
            .ok_or_else(|| SolveError::Unsupported("Ferrari resolvent failed".into()))?;
        let mut chosen = None;
        for candidate in candidates {
            ctx.tick()?;
            if let Some(key) = &candidate.key
                && !key.nonreal
                && key.re.real_sign(ctx)? == Some(1)
            {
                om_simplify::numeval::remember_real(&candidate.value, &key.re);
                chosen = Some(candidate.value);
                break;
            }
        }
        let z = chosen.ok_or_else(|| {
            SolveError::Unsupported("Ferrari has no certified positive resolvent root".into())
        })?;
        if contains_root(&z) {
            return Err(SolveError::Unsupported(
                "Ferrari resolvent is not radical".into(),
            ));
        }
        let s = sqrt(z.clone());
        let sum = add([qexpr(p.clone()), z]);
        let quotient = div(qexpr(q.clone()), s.clone());
        let t = div(sub(sum.clone(), quotient.clone()), Expr::int(2));
        let u = div(add([sum, quotient]), Expr::int(2));
        let first = [t, s.clone(), Expr::int(1)];
        let second = [u, neg(s), Expr::int(1)];
        let first_p = reductions::polynomial(&first, &y, ctx)?;
        let second_p = reductions::polynomial(&second, &y, ctx)?;
        sink.record(|| {
            Step::new(
                StepKind::Factor {
                    factors: vec![(first_p.clone(), 1), (second_p.clone(), 1)],
                },
                vec![depressed],
                vec![mul([first_p.clone(), second_p.clone()])],
                Level::Major,
            )
        });
        let mut all = simple_quadratic(&first, &first_p, ctx, sink)?;
        all.extend(simple_quadratic(&second, &second_p, ctx, sink)?);
        all
    };
    for root in &mut roots {
        ctx.tick()?;
        root.value = sub(root.value.clone(), qexpr(shift.clone()));
        root.key = None;
    }
    sink.record(|| {
        Step::new(
            StepKind::ApplyFormula {
                formula: Formula::Ferrari,
                bindings: vec![
                    ("p".into(), qexpr(p)),
                    ("q".into(), qexpr(q)),
                    ("r".into(), qexpr(r)),
                    ("shift".into(), qexpr(shift)),
                ],
                results: roots.iter().map(|r| r.value.clone()).collect(),
            },
            vec![original.clone()],
            roots.iter().map(|r| r.value.clone()).collect(),
            Level::Major,
        )
    });
    Ok(roots)
}
fn contains_root(e: &Expr) -> bool {
    e.is_head(om_core::BUILTIN::ROOT) || e.args().iter().any(contains_root)
}
fn simple_quadratic(
    c: &[Expr],
    p: &Expr,
    ctx: &Interrupt,
    sink: &mut impl StepSink,
) -> Result<Vec<Candidate>, SolveError> {
    ctx.tick()?;
    let d = sub(
        pow(c[1].clone(), Expr::int(2)),
        mul([Expr::int(4), c[0].clone()]),
    );
    sink.record(|| {
        Step::new(
            StepKind::Discriminant {
                value: d.clone(),
                sign: None,
            },
            vec![p.clone()],
            vec![d.clone()],
            Level::Minor,
        )
    });
    // The caller factors an irreducible square-free quartic, so these quadratics
    // cannot have a repeated root. No costly dependent-radical zero oracle is needed.
    let radical = sqrt(d);
    let roots = [
        sub(neg(c[1].clone()), radical.clone()),
        add([neg(c[1].clone()), radical]),
    ]
    .into_iter()
    .map(|e| Candidate {
        value: div(e, Expr::int(2)),
        multiplicity: 1,
        key: None,
    })
    .collect::<Vec<_>>();
    sink.record(|| {
        Step::new(
            StepKind::ApplyFormula {
                formula: Formula::Quadratic,
                bindings: vec![
                    ("a".into(), Expr::int(1)),
                    ("b".into(), c[1].clone()),
                    ("c".into(), c[0].clone()),
                ],
                results: roots.iter().map(|r| r.value.clone()).collect(),
            },
            vec![p.clone()],
            roots.iter().map(|r| r.value.clone()).collect(),
            Level::Major,
        )
    });
    Ok(roots)
}
