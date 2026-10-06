//! Only raw rational degree-two polynomials qualify; source holes never disappear in a certificate.
use super::*;
use om_core::Symbol;
use om_num::BitTest;
use std::collections::BTreeMap;
pub(super) type Polynomial = BTreeMap<Vec<u8>, Rational>;
pub(super) fn checked(q: Rational, ctx: &Interrupt) -> Result<Rational, EvalError> {
    ctx.tick()?;
    if q.numerator().clone().into_parts().1.bit_len() > 20000 || q.denominator().bit_len() > 20000 {
        return Err(error("全局二次认证有理数超过20000位资源限额"));
    }
    Ok(q)
}
pub(super) fn scalar(ev: &mut Evaluator, e: &Expr, ctx: &Interrupt) -> Result<Rational, EvalError> {
    super::calculus_source::exact_source(e, ctx)?;
    let e = ev.evaluate(e, ctx)?;
    let n = e
        .as_number()
        .filter(|n| n.is_exact())
        .ok_or_else(|| error("全局认证要求精确有理系数与边界，不把近似数转成证明"))?;
    checked(
        crate::scalar::rational(n).ok_or_else(|| error("需要精确有理数"))?,
        ctx,
    )
}
fn add_poly(out: &mut Polynomial, p: Polynomial, ctx: &Interrupt) -> Result<(), EvalError> {
    for (key, value) in p {
        ctx.tick()?;
        let entry = out.entry(key).or_insert(Rational::ZERO);
        *entry = checked(&*entry + value, ctx)?;
    }
    out.retain(|_, q| *q != Rational::ZERO);
    Ok(())
}
fn multiply(a: &Polynomial, b: &Polynomial, ctx: &Interrupt) -> Result<Polynomial, EvalError> {
    let mut out = Polynomial::new();
    for (a, qa) in a {
        for (b, qb) in b {
            ctx.tick()?;
            let powers: Vec<_> = a.iter().zip(b).map(|(a, b)| a + b).collect();
            if powers.iter().map(|p| *p as usize).sum::<usize>() > 2 {
                return Err(error("全局认证首版仅支持总次数不超过2的有理多项式"));
            }
            let entry = out.entry(powers).or_insert(Rational::ZERO);
            *entry = checked(&*entry + checked(qa * qb, ctx)?, ctx)?;
        }
    }
    out.retain(|_, q| *q != Rational::ZERO);
    Ok(out)
}
pub(super) fn extract(
    ev: &mut Evaluator,
    e: &Expr,
    variables: &[Symbol],
    ctx: &Interrupt,
) -> Result<Polynomial, EvalError> {
    fn walk(
        ev: &mut Evaluator,
        e: &Expr,
        variables: &[Symbol],
        ctx: &Interrupt,
        depth: usize,
    ) -> Result<Polynomial, EvalError> {
        ctx.tick()?;
        if depth > 64 {
            return Err(error("全局二次表达式深度超过64"));
        }
        let n = variables.len();
        if variables.iter().all(|s| e.free_of(&Expr::sym(*s))) {
            return Ok([(vec![0; n], scalar(ev, e, ctx)?)].into());
        }
        if let Some(i) = variables.iter().position(|s| e.as_symbol() == Some(*s)) {
            let mut p = vec![0; n];
            p[i] = 1;
            return Ok([(p, Rational::ONE)].into());
        }
        if e.is_head(B::PLUS) {
            let mut out = Polynomial::new();
            for e in e.args() {
                add_poly(&mut out, walk(ev, e, variables, ctx, depth + 1)?, ctx)?;
            }
            return Ok(out);
        }
        if e.is_head(B::TIMES) {
            // Walk every factor even when an earlier zero would cancel a raw excluded domain.
            let mut out: Polynomial = [(vec![0; n], Rational::ONE)].into();
            for factor in e.args() {
                let p = walk(ev, factor, variables, ctx, depth + 1)?;
                out = multiply(&out, &p, ctx)?;
            }
            return Ok(out);
        }
        if e.is_head(B::POWER) && e.args().len() == 2 {
            let power = match e.args()[1].as_number() {
                Some(Number::Integer(n)) => usize::try_from(n).ok(),
                _ => None,
            };
            if let Some(power) = power.filter(|n| matches!(n, 1 | 2)) {
                let p = walk(ev, &e.args()[0], variables, ctx, depth + 1)?;
                return if power == 1 {
                    Ok(p)
                } else {
                    multiply(&p, &p, ctx)
                };
            }
        }
        Err(error(
            "全局认证只接受原始有理二次多项式；高次、变量分母、函数与原孔洞不作局部近似证明",
        ))
    }
    super::calculus_source::exact_source(e, ctx)?;
    walk(ev, e, variables, ctx, 0)
}
pub(super) struct Quadratic {
    pub h: Vec<Vec<Rational>>,
    pub linear: Vec<Rational>,
    pub constant: Rational,
}
impl Quadratic {
    pub fn new(p: Polynomial, n: usize, ctx: &Interrupt) -> Result<Self, EvalError> {
        let mut out = Self {
            h: vec![vec![Rational::ZERO; n]; n],
            linear: vec![Rational::ZERO; n],
            constant: Rational::ZERO,
        };
        for (powers, c) in p {
            ctx.tick()?;
            let axes: Vec<_> = powers
                .iter()
                .enumerate()
                .flat_map(|(i, p)| std::iter::repeat_n(i, *p as usize))
                .collect();
            match &axes[..] {
                [] => out.constant = c,
                [i] => out.linear[*i] = c,
                [i, j] if i == j => out.h[*i][*i] = checked(c * Rational::from(2), ctx)?,
                [i, j] => {
                    out.h[*i][*j] = c.clone();
                    out.h[*j][*i] = c;
                }
                _ => return Err(error("二次系数内部形状无效")),
            }
        }
        Ok(out)
    }
    pub fn negate(&mut self) {
        self.constant = -self.constant.clone();
        for q in &mut self.linear {
            *q = -q.clone();
        }
        for row in &mut self.h {
            for q in row {
                *q = -q.clone();
            }
        }
    }
    pub fn gradient(&self, x: &[Rational], ctx: &Interrupt) -> Result<Vec<Rational>, EvalError> {
        let mut out = self.linear.clone();
        for (i, row) in self.h.iter().enumerate() {
            for (a, x) in row.iter().zip(x) {
                out[i] = checked(&out[i] + checked(a * x, ctx)?, ctx)?;
            }
        }
        Ok(out)
    }
    pub fn value(&self, x: &[Rational], ctx: &Interrupt) -> Result<Rational, EvalError> {
        let g = self.gradient(x, ctx)?;
        let mut value = self.constant.clone();
        for ((x, g), linear) in x.iter().zip(g).zip(&self.linear) {
            value = checked(
                value + checked(x * checked((g + linear) / Rational::from(2), ctx)?, ctx)?,
                ctx,
            )?;
        }
        Ok(value)
    }
}
pub(super) fn expr(q: Rational) -> Expr {
    Expr::number(Number::Rational(q).normalize())
}
pub(super) fn row(q: Vec<Rational>) -> Expr {
    list(q.into_iter().map(expr))
}
pub(super) fn matrix(q: Vec<Vec<Rational>>) -> Expr {
    list(q.into_iter().map(row))
}
