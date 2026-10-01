//! Heap-based symbolic differentiation preserves unspecified derivatives as syntax.
use crate::EvalError;
use om_core::{BUILTIN as B, Expr, Interrupt, Symbol, add, div, mul, neg, pow, sqrt};
use om_num::{Integer, Number};
fn unary(h: Symbol, e: Expr) -> Expr {
    let call = Expr::call(h, [e]);
    om_simplify::special::eval(&call).unwrap_or(call)
}
fn chain(e: &Expr, d: &[Expr], x: &Expr) -> Expr {
    let a = e.args();
    match e.head_symbol() {
        Some(B::LIST) => Expr::call(B::LIST, d.iter().cloned()),
        Some(B::PLUS) => add(d.iter().cloned()),
        Some(B::TIMES) => add(d
            .iter()
            .enumerate()
            .filter(|(_, v)| !v.is_zero())
            .map(|(i, v)| {
                mul(std::iter::once(v.clone()).chain(
                    a.iter()
                        .enumerate()
                        .filter(|(j, _)| *j != i)
                        .map(|(_, e)| e.clone()),
                ))
            })),
        Some(B::POWER) if a.len() == 2 => {
            if d[1].is_zero() {
                mul([
                    a[1].clone(),
                    pow(a[0].clone(), add([a[1].clone(), Expr::int(-1)])),
                    d[0].clone(),
                ])
            } else {
                mul([
                    e.clone(),
                    add([
                        mul([d[1].clone(), unary(B::LOG, a[0].clone())]),
                        div(mul([a[1].clone(), d[0].clone()]), a[0].clone()),
                    ]),
                ])
            }
        }
        Some(B::LOG) if a.len() == 2 => {
            let lb = unary(B::LOG, a[0].clone());
            let lv = unary(B::LOG, a[1].clone());
            add([
                div(d[1].clone(), mul([a[1].clone(), lb.clone()])),
                neg(div(
                    mul([lv, d[0].clone()]),
                    mul([a[0].clone(), pow(lb, Expr::int(2))]),
                )),
            ])
        }
        _ if a.len() == 1 => {
            let z = a[0].clone();
            let one = Expr::int(1);
            let square = pow(z.clone(), Expr::int(2));
            let factor = match e.head_symbol() {
                Some(B::SIN) => unary(B::COS, z),
                Some(B::COS) => neg(unary(B::SIN, z)),
                Some(B::TAN) => pow(unary(B::SEC, z), Expr::int(2)),
                Some(B::COT) => neg(pow(unary(B::CSC, z), Expr::int(2))),
                Some(B::SEC) => mul([e.clone(), unary(B::TAN, z)]),
                Some(B::CSC) => neg(mul([e.clone(), unary(B::COT, z)])),
                Some(B::EXP) => e.clone(),
                Some(B::LOG) => div(one, z),
                Some(B::SQRT) => div(one, mul([Expr::int(2), sqrt(z)])),
                Some(B::ARCSIN) => div(one, sqrt(add([Expr::int(1), neg(square)]))),
                Some(B::ARCCOS) => neg(div(one, sqrt(add([Expr::int(1), neg(square)])))),
                Some(B::ARCTAN) => div(one, add([Expr::int(1), square])),
                Some(B::ARCCOT) => neg(div(one, add([Expr::int(1), square]))),
                Some(B::ARCSEC) => div(
                    one,
                    mul([
                        square,
                        sqrt(add([Expr::int(1), neg(pow(z, Expr::int(-2)))])),
                    ]),
                ),
                Some(B::ARCCSC) => neg(div(
                    one,
                    mul([
                        square,
                        sqrt(add([Expr::int(1), neg(pow(z, Expr::int(-2)))])),
                    ]),
                )),
                Some(B::SINH) => unary(B::COSH, z),
                Some(B::COSH) => unary(B::SINH, z),
                Some(B::TANH) => pow(unary(B::SECH, z), Expr::int(2)),
                Some(B::COTH) => neg(pow(unary(B::CSCH, z), Expr::int(2))),
                Some(B::SECH) => neg(mul([e.clone(), unary(B::TANH, z)])),
                Some(B::CSCH) => neg(mul([e.clone(), unary(B::COTH, z)])),
                Some(B::ARCSINH) => div(one, sqrt(add([Expr::int(1), square]))),
                Some(B::ARCCOSH) => div(
                    one,
                    mul([
                        sqrt(add([z.clone(), Expr::int(-1)])),
                        sqrt(add([z, Expr::int(1)])),
                    ]),
                ),
                Some(B::ARCTANH) => div(one, add([Expr::int(1), neg(square)])),
                Some(B::PRODUCT_LOG) => div(e.clone(), mul([z, add([Expr::int(1), e.clone()])])),
                _ => return formal(e, d, x),
            };
            mul([factor, d[0].clone()])
        }
        _ => formal(e, d, x),
    }
}
fn formal(e: &Expr, d: &[Expr], x: &Expr) -> Expr {
    let h = e.head();
    if !h.free_of(x) {
        return Expr::call(Symbol::intern("D"), [e.clone(), x.clone()]);
    }
    let (base, old) = if h.args().len() == 1
        && h.head().is_head(Symbol::intern("Derivative"))
        && h.head().args().len() == e.args().len()
    {
        let orders = h
            .head()
            .args()
            .iter()
            .map(|v| {
                v.as_number().and_then(|n| {
                    if let Number::Integer(n) = n {
                        u32::try_from(n).ok()
                    } else {
                        None
                    }
                })
            })
            .collect::<Option<Vec<_>>>();
        match orders {
            Some(o) => (h.args()[0].clone(), o),
            None => (h, vec![0; e.args().len()]),
        }
    } else {
        (h, vec![0; e.args().len()])
    };
    add(d
        .iter()
        .enumerate()
        .filter(|(_, d)| !d.is_zero())
        .map(|(i, d)| {
            let mut orders = old.clone();
            orders[i] = orders[i].saturating_add(1);
            let derivative = Expr::call(
                Symbol::intern("Derivative"),
                orders.into_iter().map(|n| Expr::integer(Integer::from(n))),
            );
            let head = Expr::normal(derivative, [base.clone()]);
            mul([d.clone(), Expr::normal(head, e.args().iter().cloned())])
        }))
}
fn differentiate(e: &Expr, x: &Expr, ctx: &Interrupt) -> Result<Expr, EvalError> {
    enum Frame<'a> {
        Visit(&'a Expr),
        Build(&'a Expr),
    }
    let mut stack = vec![Frame::Visit(e)];
    let mut values = vec![];
    while let Some(frame) = stack.pop() {
        ctx.tick()?;
        match frame {
            Frame::Visit(e) if e == x => values.push(Expr::int(1)),
            Frame::Visit(e) if !e.is_head(B::LIST) && e.free_of(x) => values.push(Expr::int(0)),
            Frame::Visit(e) => {
                stack.push(Frame::Build(e));
                stack.extend(e.args().iter().rev().map(Frame::Visit));
            }
            Frame::Build(e) => {
                let d = values.split_off(values.len() - e.args().len());
                values.push(chain(e, &d, x));
            }
        }
    }
    Ok(values
        .pop()
        .expect("invariant: derivative traversal produces one value"))
}
pub(super) fn apply(args: &[Expr], ctx: &Interrupt) -> Result<Option<Expr>, EvalError> {
    if args.len() > 65 {
        return Ok(None);
    }
    let mut specs = vec![];
    let mut total = 0_u32;
    for spec in &args[1..] {
        ctx.tick()?;
        let (x, n) = if spec.is_head(B::LIST) && spec.args().len() == 2 {
            let Some(Number::Integer(n)) = spec.args()[1].as_number() else {
                return Ok(None);
            };
            let Ok(n) = u32::try_from(n) else {
                return Ok(None);
            };
            (&spec.args()[0], n)
        } else {
            (spec, 1)
        };
        if x.as_symbol().is_none() || n > 4096 {
            return Ok(None);
        }
        total = total.saturating_add(n);
        if total > 4096 {
            return Ok(None);
        }
        specs.push((x, n));
    }
    let mut value = args[0].clone();
    for (x, n) in specs {
        for _ in 0..n {
            value = differentiate(&value, x, ctx)?;
        }
    }
    Ok(Some(value))
}
