//! Approximate coefficient atoms represent their exact stored binary points.
use crate::SolveError;
use om_core::{Expr, ExprKind};
use om_num::{Complex, Number, Rational, Real, ctx::Interrupt};
fn number(n: &Number) -> Result<Number, SolveError> {
    Ok(match n {
        Number::Real(Real::Machine(f)) => Number::Rational(
            Rational::try_from(*f)
                .map_err(|_| SolveError::Invalid("nonfinite numerical coefficient".into()))?,
        ),
        Number::Real(Real::Big(f)) => {
            if f.repr().exponent().unsigned_abs() > 1_048_576 {
                return Err(SolveError::Unsupported(
                    "numerical coefficient exponent exceeds exact conversion limit".into(),
                ));
            }
            Number::Rational(
                Rational::try_from(f.clone())
                    .map_err(|_| SolveError::Invalid("nonfinite numerical coefficient".into()))?,
            )
        }
        Number::Complex(c) => Number::Complex(Box::new(Complex {
            re: number(&c.re)?,
            im: number(&c.im)?,
        })),
        n => n.clone(),
    }
    .normalize())
}
pub(super) fn input(e: &Expr, ctx: &Interrupt) -> Result<Expr, SolveError> {
    enum Frame<'a> {
        Enter(&'a Expr),
        Build(&'a om_core::Normal),
    }
    let mut stack = vec![Frame::Enter(e)];
    let mut values = vec![];
    while let Some(f) = stack.pop() {
        ctx.tick()?;
        match f {
            Frame::Enter(e) => match e.kind() {
                ExprKind::Number(n) => values.push(Expr::number(number(n)?)),
                ExprKind::Normal(n) => {
                    stack.push(Frame::Build(n));
                    stack.extend(n.args.iter().rev().map(Frame::Enter));
                    stack.push(Frame::Enter(&n.head));
                }
                _ => values.push(e.clone()),
            },
            Frame::Build(n) => {
                let args = values.split_off(values.len() - n.args.len());
                let head = values
                    .pop()
                    .expect("invariant: exactified numerical head visited");
                values.push(Expr::normal(head, args));
            }
        }
    }
    Ok(values
        .pop()
        .expect("invariant: one exactified numerical input"))
}
