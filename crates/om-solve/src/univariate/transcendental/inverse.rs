//! Each inverse records its range restrictions and every represented periodic branch.
use super::kernel;
use crate::{Domain, SolveError};
use om_core::{BUILTIN as B, Expr, Symbol, add, div, mul, neg, pow, sub};
use om_num::ctx::Interrupt;
use om_simplify::{
    numeval::enclose,
    zero::{Tri, is_zero_with},
};

pub(super) struct Branch {
    pub target: Expr,
    pub condition: Option<Expr>,
    pub constants: Vec<(Expr, Domain)>,
    pub known_real: bool,
}
pub(super) struct Inversion {
    pub argument: Expr,
    pub head: Symbol,
    pub branches: Vec<Branch>,
    pub ifun: bool,
}
fn parts(e: &Expr) -> Option<(Expr, Expr)> {
    if let Some(n) = e.as_number() {
        return Some(if let om_num::Number::Complex(c) = n {
            (Expr::number(c.re.clone()), Expr::number(c.im.clone()))
        } else {
            (e.clone(), Expr::int(0))
        });
    }
    if matches!(e.as_symbol(), Some(B::PI | B::E)) {
        return Some((e.clone(), Expr::int(0)));
    }
    if e.is_head(B::PLUS) {
        let mut re = vec![];
        let mut im = vec![];
        for t in e.args() {
            let (r, i) = parts(t)?;
            re.push(r);
            im.push(i);
        }
        return Some((add(re), add(im)));
    }
    if e.is_head(B::TIMES) {
        let mut re = Expr::int(1);
        let mut im = Expr::int(0);
        for t in e.args() {
            let (r, i) = parts(t)?;
            let nr = sub(mul([re.clone(), r.clone()]), mul([im.clone(), i.clone()]));
            im = add([mul([re, i]), mul([im, r])]);
            re = nr;
        }
        return Some((re, im));
    }
    None
}
fn unary(h: Symbol, v: Expr) -> Expr {
    let call = Expr::call(h, [v.clone()]);
    if let Some(value) = om_simplify::special::eval(&call) {
        return value;
    }
    if let Some((re, im)) = parts(&v) {
        if h == B::RE {
            return re;
        }
        if h == B::IM {
            return im;
        }
        let half = mul([Expr::rational(1, 2), Expr::sym(B::PI)]);
        if h == B::SIN && (re == half || re == neg(half.clone())) {
            let value = unary(B::COSH, im);
            return if re == half { value } else { neg(value) };
        }
    }
    if v.is_zero() && h == B::COSH {
        return Expr::int(1);
    }
    if v.is_zero() && matches!(h, B::ARCSINH | B::ARCTANH) {
        return Expr::int(0);
    }
    if v == Expr::int(1) && h == B::ARCCOSH {
        return Expr::int(0);
    }
    if matches!(h, B::ARCSEC | B::ARCCSC) {
        return unary(
            if h == B::ARCSEC { B::ARCCOS } else { B::ARCSIN },
            div(Expr::int(1), v),
        );
    }
    if h == B::ARCCOT && v.is_zero() {
        return mul([Expr::rational(1, 2), Expr::sym(B::PI)]);
    }
    call
}
pub(super) fn and(a: Option<Expr>, b: Option<Expr>) -> Option<Expr> {
    match (a, b) {
        (None, b) => b,
        (a, None) => a,
        (Some(a), Some(b)) if a == b => Some(a),
        (Some(a), Some(b)) => Some(Expr::call(B::AND, [a, b])),
    }
}
fn allowed_difference(e: &Expr, strict: bool, ctx: &Interrupt) -> Result<Option<bool>, SolveError> {
    if is_zero_with(e, ctx)? == Tri::Zero {
        return Ok(Some(!strict));
    }
    let Some(z) = enclose(e, 256, ctx)? else {
        return Ok(None);
    };
    if z.im.mid != om_num::BigFloat::ZERO || z.im.rad != om_num::BigFloat::ZERO {
        return Ok(None);
    }
    if z.re.mid > z.re.rad {
        return Ok(Some(true));
    }
    if z.re.mid < -&z.re.rad {
        return Ok(Some(false));
    }
    Ok(None)
}
fn range(v: &Expr, h: Symbol, ctx: &Interrupt) -> Result<Option<Option<Expr>>, SolveError> {
    let pi = Expr::sym(B::PI);
    let half = mul([Expr::rational(1, 2), pi.clone()]);
    let (component, lo, hi, strict_lo, strict_hi) = match h {
        B::LOG => (B::IM, neg(pi.clone()), pi, true, false),
        B::ARCSIN => (B::RE, neg(half.clone()), half, false, false),
        B::ARCCOS => (B::RE, Expr::int(0), pi, false, false),
        B::ARCTAN => (B::RE, neg(half.clone()), half, true, true),
        _ => return Ok(Some(None)),
    };
    let c = if let Some(z) = enclose(v, 256, ctx)? {
        if z.im.mid == om_num::BigFloat::ZERO && z.im.rad == om_num::BigFloat::ZERO {
            if component == B::RE {
                v.clone()
            } else {
                Expr::int(0)
            }
        } else if component == B::IM
            && z.re.mid == om_num::BigFloat::ZERO
            && z.re.rad == om_num::BigFloat::ZERO
        {
            mul([neg(om_core::canonicalize(&Expr::sym(B::I))), v.clone()])
        } else {
            unary(component, v.clone())
        }
    } else {
        unary(component, v.clone())
    };
    let left = allowed_difference(&sub(c.clone(), lo.clone()), strict_lo, ctx)?;
    let right = allowed_difference(&sub(hi.clone(), c.clone()), strict_hi, ctx)?;
    if left == Some(false) || right == Some(false) {
        return Ok(None);
    }
    if left == Some(true) && right == Some(true) {
        return Ok(Some(None));
    }
    Ok(Some(Some(Expr::call(
        B::AND,
        [
            Expr::call(
                if strict_lo { B::LESS } else { B::LESS_EQUAL },
                [lo, c.clone()],
            ),
            Expr::call(if strict_hi { B::LESS } else { B::LESS_EQUAL }, [c, hi]),
        ],
    ))))
}
fn zero(v: &Expr, ctx: &Interrupt) -> Result<bool, SolveError> {
    Ok(is_zero_with(v, ctx)? == Tri::Zero)
}
pub(super) fn periodic(k: &Expr) -> bool {
    matches!(
        k.head_symbol(),
        Some(B::SIN | B::COS | B::TAN | B::COT | B::SEC | B::CSC | B::SINH | B::COSH | B::TANH)
    ) || (k.is_head(B::POWER)
        && k.args().len() == 2
        && !matches!(
            k.args()[1].as_number(),
            Some(om_num::Number::Integer(_) | om_num::Number::Rational(_))
        ))
}
pub(super) fn invert(
    k: &Expr,
    v: &Expr,
    c: Option<&Expr>,
    domain: Domain,
    x: &Expr,
    ctx: &Interrupt,
) -> Result<Option<Inversion>, SolveError> {
    ctx.tick()?;
    let i = om_core::canonicalize(&Expr::sym(B::I));
    let pi = Expr::sym(B::PI);
    let mut condition = None;
    let mut known_real = false;
    let mut ifun = false;
    let mut head = k.head_symbol().unwrap_or(B::POWER);
    let (argument, targets, period) = if k.is_head(B::TIMES) {
        let Some(p) = k.args().iter().find(|p| {
            p.is_head(B::POWER) && p.args().len() == 2 && p.args()[0].as_symbol() == Some(B::E)
        }) else {
            return Ok(None);
        };
        head = B::PRODUCT_LOG;
        let u = p.args()[1].clone();
        if mul([u.clone(), p.clone()]) != *k {
            return Ok(None);
        }
        let mut targets = vec![Expr::call(B::PRODUCT_LOG, [v.clone()])];
        if domain == Domain::Reals {
            let lo = neg(pow(Expr::sym(B::E), Expr::int(-1)));
            let in_range = allowed_difference(&sub(v.clone(), lo.clone()), false, ctx)?;
            if in_range == Some(false) {
                targets.clear()
            } else {
                known_real = true;
                if in_range.is_none() {
                    condition = Some(Expr::call(B::GREATER_EQUAL, [v.clone(), lo.clone()]))
                }
                let above = allowed_difference(&sub(v.clone(), lo), true, ctx)?;
                let below = allowed_difference(&neg(v.clone()), true, ctx)?;
                if above == Some(true) && below == Some(true) {
                    targets.push(Expr::call(B::PRODUCT_LOG, [Expr::int(-1), v.clone()]))
                } else if above.is_none() || below.is_none() {
                    return Ok(None);
                }
            }
        } else {
            ifun = true
        }
        (u, targets, None)
    } else if k.is_head(B::POWER) && k.args().len() == 2 {
        let base = k.args()[0].clone();
        if zero(&base, ctx)? || zero(&kernel::log(base.clone()), ctx)? {
            return Ok(None);
        }
        let targets = if zero(v, ctx)? {
            vec![]
        } else {
            if !v.free_symbols().is_empty() {
                condition = Some(Expr::call(B::UNEQUAL, [v.clone(), Expr::int(0)]));
            }
            vec![div(kernel::log(v.clone()), kernel::log(base.clone()))]
        };
        let period = div(
            mul([Expr::int(2), pi.clone(), i.clone()]),
            kernel::log(base),
        );
        (k.args()[1].clone(), targets, Some(period))
    } else {
        let [argument] = k.args() else {
            return Ok(None);
        };
        let a = argument.clone();
        match head {
            B::ABS => {
                if domain != Domain::Reals || !kernel::real_argument(argument, x, ctx)? {
                    return Ok(None);
                }
                let nonnegative = allowed_difference(v, false, ctx)?;
                if nonnegative == Some(false) {
                    (a, vec![], None)
                } else {
                    if nonnegative.is_none() {
                        condition = Some(Expr::call(
                            B::AND,
                            [
                                Expr::call(B::GREATER_EQUAL, [v.clone(), Expr::int(0)]),
                                Expr::call(B::ELEMENT, [v.clone(), Expr::sym(B::REALS)]),
                            ],
                        ));
                    }
                    (a, vec![neg(v.clone()), v.clone()], None)
                }
            }
            B::SIN => (
                a,
                vec![
                    unary(B::ARCSIN, v.clone()),
                    sub(pi.clone(), unary(B::ARCSIN, v.clone())),
                ],
                Some(mul([Expr::int(2), pi.clone()])),
            ),
            B::COS => (
                a,
                vec![
                    neg(unary(B::ARCCOS, v.clone())),
                    unary(B::ARCCOS, v.clone()),
                ],
                Some(mul([Expr::int(2), pi.clone()])),
            ),
            B::TAN | B::COT => {
                let excluded = zero(&sub(v.clone(), i.clone()), ctx)?
                    || zero(&add([v.clone(), i.clone()]), ctx)?;
                if !excluded && !v.free_symbols().is_empty() {
                    condition = Some(Expr::call(
                        B::AND,
                        [
                            Expr::call(B::UNEQUAL, [v.clone(), i.clone()]),
                            Expr::call(B::UNEQUAL, [v.clone(), neg(i.clone())]),
                        ],
                    ))
                }
                (
                    a,
                    if excluded {
                        vec![]
                    } else {
                        vec![unary(
                            if head == B::TAN { B::ARCTAN } else { B::ARCCOT },
                            v.clone(),
                        )]
                    },
                    Some(pi.clone()),
                )
            }
            B::SEC => (
                a,
                if zero(v, ctx)? {
                    vec![]
                } else {
                    vec![
                        neg(unary(B::ARCSEC, v.clone())),
                        unary(B::ARCSEC, v.clone()),
                    ]
                },
                Some(mul([Expr::int(2), pi.clone()])),
            ),
            B::CSC => (
                a,
                if zero(v, ctx)? {
                    vec![]
                } else {
                    vec![
                        unary(B::ARCCSC, v.clone()),
                        sub(pi.clone(), unary(B::ARCCSC, v.clone())),
                    ]
                },
                Some(mul([Expr::int(2), pi.clone()])),
            ),
            B::SINH => (
                a,
                vec![
                    unary(B::ARCSINH, v.clone()),
                    sub(mul([i.clone(), pi.clone()]), unary(B::ARCSINH, v.clone())),
                ],
                Some(mul([Expr::int(2), pi.clone(), i.clone()])),
            ),
            B::COSH => (
                a,
                vec![
                    neg(unary(B::ARCCOSH, v.clone())),
                    unary(B::ARCCOSH, v.clone()),
                ],
                Some(mul([Expr::int(2), pi.clone(), i.clone()])),
            ),
            B::TANH => {
                let excluded = zero(&sub(v.clone(), Expr::int(1)), ctx)?
                    || zero(&add([v.clone(), Expr::int(1)]), ctx)?;
                (
                    a,
                    if excluded {
                        vec![]
                    } else {
                        vec![unary(B::ARCTANH, v.clone())]
                    },
                    Some(mul([pi.clone(), i.clone()])),
                )
            }
            B::LOG | B::ARCSIN | B::ARCCOS | B::ARCTAN => {
                if head == B::ARCTAN
                    && let Some((re, im)) = parts(v)
                {
                    let half = mul([Expr::rational(1, 2), pi.clone()]);
                    if (re == half || re == neg(half)) && !im.is_zero() {
                        return Ok(None);
                    }
                }
                let Some(cond) = range(v, head, ctx)? else {
                    return Ok(Some(Inversion {
                        argument: a,
                        head,
                        branches: vec![],
                        ifun: false,
                    }));
                };
                condition = cond;
                let target = match head {
                    B::LOG => pow(Expr::sym(B::E), v.clone()),
                    B::ARCSIN => unary(B::SIN, v.clone()),
                    B::ARCCOS => unary(B::COS, v.clone()),
                    _ => unary(B::TAN, v.clone()),
                };
                if head != B::LOG && !v.free_symbols().is_empty() {
                    if head == B::ARCTAN {
                        let half = mul([Expr::rational(1, 2), pi.clone()]);
                        let re = unary(B::RE, v.clone());
                        condition = Some(Expr::call(
                            B::AND,
                            [
                                Expr::call(B::LESS_EQUAL, [neg(half.clone()), re.clone()]),
                                Expr::call(B::LESS_EQUAL, [re, half]),
                            ],
                        ));
                    }
                    condition = and(
                        condition,
                        Some(Expr::call(
                            B::EQUAL,
                            [Expr::call(head, [target.clone()]), v.clone()],
                        )),
                    );
                }
                (a, vec![target], None)
            }
            _ => return Ok(None),
        }
    };
    let mut branches = vec![];
    for target in targets {
        let mut constants = vec![];
        let target = if let Some(period) = &period {
            let Some(c) = c else { return Ok(None) };
            constants.push((c.clone(), Domain::Integers));
            let ratio =
                om_simplify::algebra::cancel_with(&div(target.clone(), period.clone()), &[], ctx)?;
            let target = if let Some(q) = ratio.as_ref().and_then(super::super::extract::exact) {
                let d = om_num::Integer::from(q.denominator().clone());
                let mut floor = q.numerator() / &d;
                if q < om_num::Rational::ZERO && q.numerator() % &d != om_num::Integer::ZERO {
                    floor -= 1;
                }
                sub(
                    target,
                    mul([Expr::number(om_num::Number::Integer(floor)), period.clone()]),
                )
            } else {
                target
            };
            add([target, mul([period.clone(), c.clone()])])
        } else {
            target
        };
        if !branches.iter().any(|b: &Branch| b.target == target) {
            branches.push(Branch {
                target,
                condition: condition.clone(),
                constants,
                known_real,
            })
        }
    }
    Ok(Some(Inversion {
        argument,
        head,
        branches,
        ifun,
    }))
}
