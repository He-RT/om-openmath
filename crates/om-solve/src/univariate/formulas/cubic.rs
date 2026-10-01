//! Depressed Cardano formula with a nonzero cube-root branch and dependent v=-p/(3u).
use super::super::{Candidate, extract, reductions};
use crate::{Formula, Level, Sign, SolveError, Step, StepKind, StepSink};
use om_core::{Expr, add, div, mul, neg, pow, sqrt, sub};
use om_num::{Number, Rational, ctx::Interrupt};
pub(super) fn qexpr(q: Rational) -> Expr {
    Expr::number(Number::Rational(q))
}
pub(in crate::univariate) fn cardano(
    c: &[Expr],
    original: &Expr,
    x: &Expr,
    ctx: &Interrupt,
    sink: &mut impl StepSink,
) -> Result<Vec<Candidate>, SolveError> {
    ctx.tick()?;
    let c = c
        .iter()
        .map(extract::exact)
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| SolveError::Unsupported("Cardano requires rational coefficients".into()))?;
    let (a, b, d, e) = (&c[3], &c[2], &c[1], &c[0]);
    let shift = b / (Rational::from(3) * a);
    let p = d / a - b * b / (Rational::from(3) * a * a);
    let q = Rational::from(2) * b * b * b / (Rational::from(27) * a * a * a)
        - b * d / (Rational::from(3) * a * a)
        + e / a;
    let disc = -Rational::from(4) * &p * &p * &p - Rational::from(27) * &q * &q;
    let y = reductions::fresh(original, x, ctx)?;
    let depressed = reductions::polynomial(
        &[
            qexpr(q.clone()),
            qexpr(p.clone()),
            Expr::int(0),
            Expr::int(1),
        ],
        &y,
        ctx,
    )?;
    sink.record(|| {
        Step::new(
            StepKind::Substitute {
                new_var: y,
                def: add([x.clone(), qexpr(shift.clone())]),
            },
            vec![original.clone()],
            vec![depressed.clone()],
            Level::Major,
        )
    });
    let sign = if disc < Rational::ZERO {
        Sign::Negative
    } else if disc == Rational::ZERO {
        Sign::Zero
    } else {
        Sign::Positive
    };
    sink.record(|| {
        Step::new(
            StepKind::Discriminant {
                value: qexpr(disc.clone()),
                sign: Some(sign),
            },
            vec![depressed],
            vec![qexpr(disc)],
            Level::Minor,
        )
    });
    let mut roots = vec![];
    let (u, v) = if p == Rational::ZERO && q == Rational::ZERO {
        roots.push(Candidate {
            value: neg(qexpr(shift.clone())),
            multiplicity: 3,
            key: None,
        });
        (Expr::int(0), Expr::int(0))
    } else {
        let radical = sqrt(qexpr(
            &q * &q / Rational::from(4) + &p * &p * &p / Rational::from(27),
        ));
        let half_q = qexpr(-&q / Rational::from(2));
        let mut base = add([half_q.clone(), radical.clone()]);
        if base.is_zero() {
            base = sub(half_q, radical);
        }
        if base.is_zero() {
            return Err(SolveError::Unsupported(
                "Cardano cube-root branch is zero".into(),
            ));
        }
        let u = pow(base, Expr::rational(1, 3));
        let v = div(qexpr(-p.clone()), mul([Expr::int(3), u.clone()]));
        let omega = pow(Expr::int(-1), Expr::rational(2, 3));
        let conjugate = pow(Expr::int(-1), Expr::rational(-2, 3));
        for (phase, inverse) in [
            (Expr::int(1), Expr::int(1)),
            (omega.clone(), conjugate.clone()),
            (conjugate, omega),
        ] {
            ctx.tick()?;
            roots.push(Candidate {
                value: sub(
                    add([mul([phase, u.clone()]), mul([inverse, v.clone()])]),
                    qexpr(shift.clone()),
                ),
                multiplicity: 1,
                key: None,
            });
        }
        (u, v)
    };
    sink.record(|| {
        Step::new(
            StepKind::ApplyFormula {
                formula: Formula::Cardano,
                bindings: vec![
                    ("p".into(), qexpr(p)),
                    ("q".into(), qexpr(q)),
                    ("u".into(), u),
                    ("v".into(), v),
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
