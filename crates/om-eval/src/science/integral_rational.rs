//! Rational primitives use true partial fractions and the verified repeated-quadratic recurrence.
use super::*;
use om_core::{add, div, mul, neg, pow, sqrt};
pub(super) fn term(e: &Expr, x: &Expr, ctx: &Interrupt) -> Result<Option<Expr>, EvalError> {
    let factors = if e.is_head(B::TIMES) {
        e.args().to_vec()
    } else {
        vec![e.clone()]
    };
    for (index, factor) in factors.iter().enumerate() {
        ctx.tick()?;
        if !factor.is_head(B::POWER) || factor.args().len() != 2 {
            continue;
        }
        let Some(Number::Integer(n)) = factor.args()[1].as_number() else {
            continue;
        };
        let Ok(n) = i32::try_from(n) else {
            continue;
        };
        if !(-32..=-1).contains(&n) {
            continue;
        }
        let Some(coeff) = super::integral_poly::coefficients(&factor.args()[0], x, ctx)? else {
            continue;
        };
        if coeff.len() != 3 {
            continue;
        }
        let Some(q) = coeff
            .iter()
            .map(|c| {
                c.as_number()
                    .filter(|n| n.is_exact())
                    .and_then(crate::scalar::rational)
            })
            .collect::<Option<Vec<_>>>()
        else {
            continue;
        };
        let numerator = mul(factors
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != index)
            .map(|(_, e)| e.clone()));
        let Some(num) = super::integral_poly::coefficients(&numerator, x, ctx)? else {
            continue;
        };
        if num.len() > 2 {
            continue;
        }
        let (a, b, c) = (q[2].clone(), q[1].clone(), q[0].clone());
        if a == Rational::ZERO {
            continue;
        }
        let d = Rational::from(4) * &a * c - &b * &b;
        if d == Rational::ZERO {
            continue;
        }
        let base = factor.args()[0].clone();
        let derivative = add([
            mul([
                Expr::number(Number::Rational(Rational::from(2) * &a)),
                x.clone(),
            ]),
            Expr::number(Number::Rational(b.clone())),
        ]);
        let h = div(
            num.get(1).cloned().unwrap_or_else(|| Expr::int(0)),
            Expr::number(Number::Rational(Rational::from(2) * &a)),
        );
        let remainder = add([
            num[0].clone(),
            neg(mul([h.clone(), Expr::number(Number::Rational(b))])),
        ]);
        let delta = Expr::number(Number::Rational(if d > Rational::ZERO {
            d.clone()
        } else {
            -&d
        }));
        let root = sqrt(delta);
        let mut integral = if d > Rational::ZERO {
            div(
                mul([
                    Expr::int(2),
                    Expr::call(B::ARCTAN, [div(derivative.clone(), root.clone())]),
                ]),
                root,
            )
        } else {
            div(
                Expr::call(
                    B::LOG,
                    [div(
                        add([derivative.clone(), neg(root.clone())]),
                        add([derivative.clone(), root.clone()]),
                    )],
                ),
                root,
            )
        };
        for k in 2..=(-n) {
            ctx.tick()?;
            let denom = Expr::number(Number::Rational(Rational::from(k - 1) * &d));
            integral = add([
                div(
                    derivative.clone(),
                    mul([
                        denom.clone(),
                        pow(base.clone(), Expr::int(i64::from(k - 1))),
                    ]),
                ),
                mul([
                    div(
                        Expr::number(Number::Rational(Rational::from(2 * (2 * k - 3)) * &a)),
                        denom,
                    ),
                    integral,
                ]),
            ]);
        }
        let logarithmic = if n == -1 {
            mul([h, Expr::call(B::LOG, [base.clone()])])
        } else {
            div(
                mul([h, pow(base.clone(), Expr::int(i64::from(n + 1)))]),
                Expr::int(i64::from(n + 1)),
            )
        };
        return Ok(Some(add([logarithmic, mul([remainder, integral])])));
    }
    Ok(None)
}
