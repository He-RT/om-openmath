//! Approximate dispatch and exact extensions for M4's elementary families.
use crate::EvalError;
use om_core::{BUILTIN as B, Expr, Interrupt, Symbol, div, mul};
use om_num::Precision;

pub(crate) fn numeric(
    head: Symbol,
    args: &[Expr],
    ctx: &Interrupt,
) -> Result<Option<Expr>, EvalError> {
    // Canonical arithmetic and scalar component/sign rules already handle
    // numeric atoms. Preserve their rounding order and exact zero/unit outputs.
    if args.iter().all(|e| e.as_number().is_some())
        && matches!(
            head.name(),
            "Plus"
                | "Times"
                | "Power"
                | "Subtract"
                | "Divide"
                | "Minus"
                | "Sqrt"
                | "Exp"
                | "Abs"
                | "Sign"
                | "Re"
                | "Im"
                | "Conjugate"
        )
    {
        return Ok(None);
    }
    if !matches!(
        head.name(),
        "Plus"
            | "Times"
            | "Power"
            | "Subtract"
            | "Divide"
            | "Minus"
            | "Sqrt"
            | "Exp"
            | "Log"
            | "Abs"
            | "Sign"
            | "Re"
            | "Im"
            | "Conjugate"
            | "Arg"
            | "Sin"
            | "Cos"
            | "Tan"
            | "Cot"
            | "Sec"
            | "Csc"
            | "ArcSin"
            | "ArcCos"
            | "ArcTan"
            | "ArcCot"
            | "ArcSec"
            | "ArcCsc"
            | "Sinh"
            | "Cosh"
            | "Tanh"
            | "Coth"
            | "Sech"
            | "Csch"
            | "ArcSinh"
            | "ArcCosh"
            | "ArcTanh"
    ) {
        return Ok(None);
    }
    let mut precision = Precision::Exact;
    let mut stack: Vec<_> = args.iter().collect();
    while let Some(e) = stack.pop() {
        ctx.tick()?;
        if let Some(n) = e.as_number() {
            precision = match (precision, n.precision()) {
                (Precision::Machine, _) | (_, Precision::Machine) => Precision::Machine,
                (Precision::Exact, p) | (p, Precision::Exact) => p,
                (Precision::Bits(a), Precision::Bits(b)) => Precision::Bits(a.min(b)),
            };
        }
        stack.extend(e.args());
    }
    if precision == Precision::Exact {
        return Ok(None);
    }
    Ok(
        om_simplify::numeval::approximate(&Expr::call(head, args.iter().cloned()), precision, ctx)?
            .map(Expr::number),
    )
}

pub(crate) fn exact(head: Symbol, args: &[Expr]) -> Option<Expr> {
    if let Some(value) = om_simplify::special::eval(&Expr::call(head, args.iter().cloned())) {
        return Some(value);
    }
    let [x] = args else {
        return None;
    };
    match head {
        B::COT | B::SEC | B::CSC => {
            let (a, b) = match head {
                B::COT => (
                    om_simplify::special::eval(&Expr::call(B::COS, [x.clone()]))?,
                    om_simplify::special::eval(&Expr::call(B::SIN, [x.clone()]))?,
                ),
                B::SEC => (
                    Expr::int(1),
                    om_simplify::special::eval(&Expr::call(B::COS, [x.clone()]))?,
                ),
                _ => (
                    Expr::int(1),
                    om_simplify::special::eval(&Expr::call(B::SIN, [x.clone()]))?,
                ),
            };
            // Only extend the exact angle table, not a symbolic parity rewrite.
            let mut stack = vec![&a, &b];
            while let Some(e) = stack.pop() {
                if e.is_head(B::SIN) || e.is_head(B::COS) {
                    return None;
                }
                stack.extend(e.args());
            }
            let square = mul([b.clone(), b.clone()]);
            Some(
                if !b.is_zero() && square.as_number().is_some_and(|n| n.is_exact()) {
                    div(mul([a, b]), square)
                } else {
                    div(a, b)
                },
            )
        }
        B::ARCCOT if x.is_zero() => Some(mul([Expr::rational(1, 2), Expr::sym(B::PI)])),
        B::ARCCOT | B::ARCSEC | B::ARCCSC => {
            let h = match head {
                B::ARCCOT => B::ARCTAN,
                B::ARCSEC => B::ARCCOS,
                _ => B::ARCSIN,
            };
            om_simplify::special::eval(&Expr::call(h, [div(Expr::int(1), x.clone())]))
        }
        B::SINH | B::TANH | B::ARCSINH | B::ARCTANH if x == &Expr::int(0) => Some(Expr::int(0)),
        B::COSH | B::SECH if x == &Expr::int(0) => Some(Expr::int(1)),
        B::COTH | B::CSCH if x == &Expr::int(0) => Some(Expr::call(B::DIRECTED_INFINITY, [])),
        B::ARCCOSH if x == &Expr::int(1) => Some(Expr::int(0)),
        B::ARCCOSH if x == &Expr::int(0) => Some(mul([
            Expr::rational(1, 2),
            Expr::sym(B::I),
            Expr::sym(B::PI),
        ])),
        _ => None,
    }
}
