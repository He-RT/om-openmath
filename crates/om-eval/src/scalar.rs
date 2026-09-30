//! Exact scalar arithmetic, integer utilities and elementary dispatch.

use crate::{EvalError, Evaluator};
use om_core::{
    BUILTIN as B, Expr, Interrupt, MsgLevel, Symbol, add, div, mul, neg, pow, sqrt, sub,
};
use om_num::{Integer, Number, Rational, Real};
use std::cmp::Ordering;

pub(crate) fn dispatch(
    ev: &mut Evaluator,
    head: Symbol,
    args: &[Expr],
    ctx: &Interrupt,
) -> Result<Option<Expr>, EvalError> {
    ctx.tick()?;
    let result = match head.name() {
        "Plus" | "Times" | "Power" | "Sqrt" | "Exp" => None,
        "Subtract" => Some(sub(args[0].clone(), args[1].clone())),
        "Divide" => Some(div(args[0].clone(), args[1].clone())),
        "Minus" => Some(neg(args[0].clone())),
        "Floor" | "Ceiling" | "Round" => rounded(head, args),
        "Mod" | "Quotient" => modulo(head, args),
        "GCD" | "LCM" => gcd_lcm(head, args),
        "Factorial" => factorial(&args[0], ctx)?,
        "Binomial" => binomial(args, ctx)?,
        "FactorInteger" => factor_integer(ev, &args[0], ctx)?,
        "PrimeQ" => Some(boolean(if let Some(n) = integer(&args[0]) {
            om_num::is_probable_prime(&abs(n))
        } else {
            false
        })),
        "Numerator" | "Denominator" => Some(fraction_part(head, &args[0])),
        "Equal" | "Unequal" | "Less" | "LessEqual" | "Greater" | "GreaterEqual" | "SameQ"
        | "Not" | "And" | "Or" | "Inequality" => crate::logic::eval(head, args),
        "Abs" => magnitude(&args[0])
            .or_else(|| om_simplify::special::eval(&Expr::call(head, args.iter().cloned()))),
        "Sign" => sign(&args[0]),
        "Arg" => argument(&args[0]),
        "ArcTan" if args.len() == 2 => angle(&args[0], &args[1]),
        "Log" if args.len() == 2 => base_log(&args[0], &args[1], ctx)?,
        _ => om_simplify::special::eval(&Expr::call(head, args.iter().cloned())),
    };
    Ok(result)
}
fn integer(e: &Expr) -> Option<&Integer> {
    if let Some(Number::Integer(n)) = e.as_number() {
        Some(n)
    } else {
        None
    }
}
fn abs(n: &Integer) -> Integer {
    Integer::from(n.clone().into_parts().1)
}
pub(crate) fn rational(n: &Number) -> Option<Rational> {
    match n {
        Number::Integer(n) => Some(Rational::from(n.clone())),
        Number::Rational(q) => Some(q.clone()),
        Number::Real(Real::Machine(x)) => Rational::try_from(*x).ok(),
        Number::Real(Real::Big(x)) => Rational::try_from(x.clone()).ok(),
        _ => None,
    }
}
fn floor(q: &Rational) -> Integer {
    let denominator = Integer::from(q.denominator().clone());
    let mut value = q.numerator() / &denominator;
    if q.numerator() < &Integer::ZERO && q.numerator() % denominator != Integer::ZERO {
        value -= 1;
    }
    value
}
fn rounded(head: Symbol, args: &[Expr]) -> Option<Expr> {
    let x = rational(args[0].as_number()?)?;
    let step = if args.len() == 2 {
        rational(args[1].as_number()?)?
    } else {
        Rational::ONE
    };
    if step.is_zero() {
        return None;
    }
    let value = &x / &step;
    let lower = floor(&value);
    let result = match head.name() {
        "Floor" => lower,
        "Ceiling" => {
            if value == Rational::from(lower.clone()) {
                lower
            } else {
                lower + 1
            }
        }
        _ => {
            let fraction = value - Rational::from(lower.clone());
            match fraction.cmp(&(Rational::from(1) / Rational::from(2))) {
                Ordering::Less => lower,
                Ordering::Greater => lower + 1,
                Ordering::Equal => {
                    if &lower % 2 == 0 {
                        lower
                    } else {
                        lower + 1
                    }
                }
            }
        }
    };
    Some(mul([
        Expr::integer(result),
        if args.len() == 2 {
            args[1].clone()
        } else {
            Expr::int(1)
        },
    ]))
}
fn modulo(head: Symbol, args: &[Expr]) -> Option<Expr> {
    let n = rational(args[0].as_number()?)?;
    let m = rational(args[1].as_number()?)?;
    if m.is_zero() {
        return None;
    }
    let offset = if args.len() == 3 {
        rational(args[2].as_number()?)?
    } else {
        Rational::ZERO
    };
    let q = floor(&((&n - offset) / m));
    Some(if head.name() == "Quotient" {
        Expr::integer(q)
    } else {
        sub(args[0].clone(), mul([args[1].clone(), Expr::integer(q)]))
    })
}
fn lcm(a: &Integer, b: &Integer) -> Integer {
    if a.is_zero() || b.is_zero() {
        Integer::ZERO
    } else {
        abs(&(a / om_num::gcd(a, b) * b))
    }
}
fn gcd_lcm(head: Symbol, args: &[Expr]) -> Option<Expr> {
    let qs = args
        .iter()
        .map(|e| e.as_number().filter(|n| n.is_exact()).and_then(rational))
        .collect::<Option<Vec<_>>>()?;
    let is_gcd = head == B::GCD;
    let mut numerator = if is_gcd { Integer::ZERO } else { Integer::ONE };
    let mut denominator = if is_gcd { Integer::ONE } else { Integer::ZERO };
    for q in qs {
        let d = Integer::from(q.denominator().clone());
        numerator = if is_gcd {
            om_num::gcd(&numerator, q.numerator())
        } else {
            lcm(&numerator, q.numerator())
        };
        denominator = if is_gcd {
            lcm(&denominator, &d)
        } else {
            om_num::gcd(&denominator, &d)
        };
    }
    if denominator.is_zero() {
        denominator = Integer::ONE;
    }
    Some(Expr::number(Number::Rational(
        Rational::from(numerator) / Rational::from(denominator),
    )))
}
fn factorial(e: &Expr, ctx: &Interrupt) -> Result<Option<Expr>, EvalError> {
    let Some(n) = integer(e)
        .and_then(|n| u64::try_from(n).ok())
        .filter(|n| *n <= 1_000_000)
    else {
        return Ok(None);
    };
    let mut product = Integer::ONE;
    for k in 2..=n {
        ctx.tick()?;
        product *= k;
    }
    Ok(Some(Expr::integer(product)))
}
fn binomial(args: &[Expr], ctx: &Interrupt) -> Result<Option<Expr>, EvalError> {
    let Some(n) = integer(&args[0]) else {
        return Ok(None);
    };
    let Some(k) = integer(&args[1]) else {
        return Ok(None);
    };
    if k < &Integer::ZERO || (n >= &Integer::ZERO && k > n) {
        return Ok(Some(Expr::int(0)));
    }
    let Some(mut k) = u64::try_from(k).ok().filter(|k| *k <= 1_000_000) else {
        return Ok(None);
    };
    if n >= &Integer::ZERO
        && let Ok(complement) = u64::try_from(n - Integer::from(k))
    {
        k = k.min(complement);
    }
    let mut result = Integer::ONE;
    for i in 0..k {
        ctx.tick()?;
        result = result * (n - Integer::from(i)) / Integer::from(i + 1);
    }
    Ok(Some(Expr::integer(result)))
}
fn factor_integer(
    ev: &mut Evaluator,
    e: &Expr,
    ctx: &Interrupt,
) -> Result<Option<Expr>, EvalError> {
    let Some(n) = integer(e) else {
        return Ok(None);
    };
    let initial = ctx.steps_left.get().min(1_000_000);
    let mut budget = initial;
    let factors = om_num::factor_integer(n, &mut budget);
    ctx.steps_left
        .set(ctx.steps_left.get().saturating_sub(initial - budget));
    ctx.tick()?;
    if factors
        .iter()
        .any(|(n, _)| n > &Integer::ONE && !om_num::is_probable_prime(n))
    {
        ev.message(
            "FactorInteger",
            "budget",
            "Factorization did not finish within the available budget.".into(),
            MsgLevel::Warning,
        );
        return Ok(None);
    }
    Ok(Some(Expr::call(
        B::LIST,
        factors
            .into_iter()
            .map(|(n, k)| Expr::call(B::LIST, [Expr::integer(n), Expr::int(k as i64)])),
    )))
}
fn magnitude(e: &Expr) -> Option<Expr> {
    let n = e.as_number()?;
    Some(if let Number::Complex(c) = n {
        sqrt(Expr::number(c.re.mul(&c.re).add(&c.im.mul(&c.im))))
    } else {
        Expr::number(
            if n.cmp_real(&Number::Integer(0.into())) == Some(Ordering::Less) {
                n.neg()
            } else {
                n.clone()
            },
        )
    })
}
fn sign(e: &Expr) -> Option<Expr> {
    let n = e.as_number()?;
    if n.is_zero() {
        return Some(Expr::int(0));
    }
    if !matches!(n, Number::Complex(_)) {
        return Some(Expr::int(
            if n.cmp_real(&Number::Integer(0.into())) == Some(Ordering::Less) {
                -1
            } else {
                1
            },
        ));
    }
    Some(div(e.clone(), magnitude(e)?))
}
fn argument(e: &Expr) -> Option<Expr> {
    let n = e.as_number()?;
    if let Number::Complex(c) = n {
        angle(&Expr::number(c.re.clone()), &Expr::number(c.im.clone()))
    } else if n.is_zero() {
        Some(Expr::int(0))
    } else {
        Some(
            if n.cmp_real(&Number::Integer(0.into())) == Some(Ordering::Less) {
                Expr::sym(B::PI)
            } else {
                Expr::int(0)
            },
        )
    }
}
fn angle(x: &Expr, y: &Expr) -> Option<Expr> {
    let a = x.as_number()?;
    let b = y.as_number()?;
    let x_sign = a.cmp_real(&Number::Integer(0.into()))?;
    let y_sign = b.cmp_real(&Number::Integer(0.into()))?;
    if a.is_zero() {
        return Some(if b.is_zero() {
            Expr::sym(B::INDETERMINATE)
        } else {
            mul([
                Expr::rational(if y_sign == Ordering::Less { -1 } else { 1 }, 2),
                Expr::sym(B::PI),
            ])
        });
    }
    if b.is_zero() {
        return Some(if x_sign == Ordering::Less {
            Expr::sym(B::PI)
        } else {
            Expr::int(0)
        });
    }
    let t = Expr::call(B::ARCTAN, [div(y.clone(), x.clone())]);
    let t = om_simplify::special::eval(&t).unwrap_or(t);
    Some(if x_sign == Ordering::Less {
        add([
            t,
            mul([
                Expr::int(if y_sign == Ordering::Less { -1 } else { 1 }),
                Expr::sym(B::PI),
            ]),
        ])
    } else {
        t
    })
}
fn base_log(base: &Expr, x: &Expr, ctx: &Interrupt) -> Result<Option<Expr>, EvalError> {
    if *base == Expr::int(1) || base.is_zero() {
        return Ok(None);
    }
    if let (Some(b), Some(n)) = (integer(base), integer(x))
        && b > &Integer::ONE
        && n > &Integer::ZERO
    {
        let mut n = n.clone();
        let mut k = 0;
        while &n % b == Integer::ZERO {
            ctx.tick()?;
            n /= b;
            k += 1;
        }
        if n.is_one() {
            return Ok(Some(Expr::int(k)));
        }
    }
    Ok(Some(div(
        Expr::call(B::LOG, [x.clone()]),
        Expr::call(B::LOG, [base.clone()]),
    )))
}
fn fraction_part(head: Symbol, e: &Expr) -> Expr {
    let denominator = head.name() == "Denominator";
    if let Some(Number::Rational(q)) = e.as_number() {
        return if denominator {
            Expr::integer(q.denominator().clone().into())
        } else {
            Expr::integer(q.numerator().clone())
        };
    }
    if e.is_head(B::TIMES) {
        return mul(e.args().iter().map(|e| fraction_part(head, e)));
    }
    if e.is_head(B::POWER)
        && e.args().len() == 2
        && e.args()[1]
            .as_number()
            .is_some_and(|n| n.cmp_real(&Number::Integer(0.into())) == Some(Ordering::Less))
    {
        return if denominator {
            pow(e.args()[0].clone(), neg(e.args()[1].clone()))
        } else {
            Expr::int(1)
        };
    }
    if denominator { Expr::int(1) } else { e.clone() }
}
pub(crate) fn boolean(value: bool) -> Expr {
    Expr::sym(if value { B::TRUE } else { B::FALSE })
}
