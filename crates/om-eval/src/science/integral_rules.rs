//! Bounded exact rule integration. Returned expressions are local principal-branch primitives.
use super::*;
use om_core::{add, div, mul, neg, pow, sqrt};
pub(super) struct Rules<'a> {
    pub x: Expr,
    pub ctx: &'a Interrupt,
    pub conditions: Vec<Expr>,
    pub guards: Vec<Expr>,
}
impl Rules<'_> {
    pub fn nonzero(&mut self, e: Expr) -> Result<(), EvalError> {
        if e.is_zero() {
            return Err(error("符号积分需要非零系数"));
        }
        if e.as_number().is_none() {
            let c = Expr::call(B::UNEQUAL, [e, Expr::int(0)]);
            if !self.conditions.contains(&c) {
                self.conditions.push(c);
            }
        }
        Ok(())
    }
    pub fn source_guards(&mut self, e: &Expr) -> Result<(), EvalError> {
        let mut stack = vec![e];
        while let Some(e) = stack.pop() {
            self.ctx.tick()?;
            if e.is_head(B::POWER)
                && e.args().len() == 2
                && e.args()[1]
                    .as_number()
                    .and_then(crate::scalar::rational)
                    .is_some_and(|q| q < Rational::ZERO)
            {
                self.guard(e.args()[0].clone());
            }
            if e.is_head(B::LOG) && e.args().len() == 1 {
                self.guard(e.args()[0].clone());
            }
            stack.extend(e.args());
        }
        Ok(())
    }
    fn guard(&mut self, e: Expr) {
        if !self.guards.contains(&e) {
            self.guards.push(e);
        }
    }
    fn template(&mut self, e: &Expr) -> Result<Option<(Expr, Expr)>, EvalError> {
        if e.is_head(B::POWER)
            && e.args().len() == 2
            && !e.args()[0].free_of(&self.x)
            && e.args()[1].free_of(&self.x)
        {
            let u = e.args()[0].clone();
            let n = e.args()[1].clone();
            if n.as_number().and_then(crate::scalar::rational) == Some(Rational::from(-1)) {
                self.guard(u.clone());
                return Ok(Some((Expr::call(B::LOG, [u.clone()]), u)));
            }
            let exponent = add([n, Expr::int(1)]);
            self.nonzero(exponent.clone())?;
            return Ok(Some((div(pow(u.clone(), exponent.clone()), exponent), u)));
        }
        let (name, u) =
            if e.is_head(B::POWER) && e.args().len() == 2 && e.args()[0] == Expr::sym(B::E) {
                ("Exp", e.args()[1].clone())
            } else if let Some(h) = e.head_symbol()
                && e.args().len() == 1
            {
                (h.name(), e.args()[0].clone())
            } else {
                return Ok(None);
            };
        if u.free_of(&self.x) {
            return Ok(None);
        }
        let f = match name {
            "Exp" => Expr::call(B::EXP, [u.clone()]),
            "Sin" => neg(Expr::call(B::COS, [u.clone()])),
            "Cos" => Expr::call(B::SIN, [u.clone()]),
            "Sinh" => Expr::call(B::COSH, [u.clone()]),
            "Cosh" => Expr::call(B::SINH, [u.clone()]),
            "ArcTan" | "ArcTanh" | "ArcCoth" => {
                let square = pow(u.clone(), Expr::int(2));
                let base = if name == "ArcTan" {
                    add([Expr::int(1), square])
                } else if name == "ArcCoth" {
                    add([square, Expr::int(-1)])
                } else {
                    add([Expr::int(1), neg(square)])
                };
                if name == "ArcCoth" {
                    self.conditions
                        .push(Expr::call(B::ELEMENT, [u.clone(), Expr::sym(B::REALS)]));
                    self.conditions
                        .push(Expr::call(B::GREATER, [base.clone(), Expr::int(0)]));
                    self.guard(base.clone());
                }
                let term = mul([Expr::rational(1, 2), Expr::call(B::LOG, [base])]);
                add([
                    mul([u.clone(), e.clone()]),
                    if name == "ArcTan" { neg(term) } else { term },
                ])
            }
            "ArcSin" | "ArcCos" => {
                let root = sqrt(add([Expr::int(1), neg(pow(u.clone(), Expr::int(2)))]));
                add([
                    mul([u.clone(), e.clone()]),
                    if name == "ArcSin" { root } else { neg(root) },
                ])
            }
            "ArcSinh" => add([
                mul([u.clone(), e.clone()]),
                neg(sqrt(add([Expr::int(1), pow(u.clone(), Expr::int(2))]))),
            ]),
            "CubeRoot" => {
                self.conditions
                    .push(Expr::call(B::ELEMENT, [u.clone(), Expr::sym(B::REALS)]));
                mul([Expr::rational(3, 4), pow(e.clone(), Expr::int(4))])
            }
            "Tan" => {
                let g = Expr::call(B::COS, [u.clone()]);
                self.guard(g.clone());
                neg(Expr::call(B::LOG, [g]))
            }
            "Cot" => {
                let g = Expr::call(B::SIN, [u.clone()]);
                self.guard(g.clone());
                Expr::call(B::LOG, [g])
            }
            "Tanh" => Expr::call(B::LOG, [Expr::call(B::COSH, [u.clone()])]),
            "Coth" => {
                let g = Expr::call(B::SINH, [u.clone()]);
                self.guard(g.clone());
                Expr::call(B::LOG, [g])
            }
            "Log" => {
                self.guard(u.clone());
                add([
                    mul([u.clone(), Expr::call(B::LOG, [u.clone()])]),
                    neg(u.clone()),
                ])
            }
            _ => return Ok(None),
        };
        Ok(Some((f, u)))
    }
    pub fn gaussian(&mut self, e: &Expr) -> Result<Option<(Expr, Expr, Expr)>, EvalError> {
        let exponent = if e.is_head(B::EXP) && e.args().len() == 1 {
            &e.args()[0]
        } else if e.is_head(B::POWER) && e.args().len() == 2 && e.args()[0] == Expr::sym(B::E) {
            &e.args()[1]
        } else {
            return Ok(None);
        };
        let Some(c) = super::integral_poly::coefficients(exponent, &self.x, self.ctx)? else {
            return Ok(None);
        };
        if c.len() != 3 {
            return Ok(None);
        }
        let a = neg(c[2].clone());
        self.nonzero(a.clone())?;
        if a.as_number()
            .and_then(crate::scalar::rational)
            .is_some_and(|q| q < Rational::ZERO)
        {
            return Ok(None);
        }
        let center = div(c[1].clone(), mul([Expr::int(2), a.clone()]));
        let constant = add([
            c[0].clone(),
            div(
                pow(c[1].clone(), Expr::int(2)),
                mul([Expr::int(4), a.clone()]),
            ),
        ]);
        Ok(Some((a, center, constant)))
    }
    pub fn integrate(
        &mut self,
        e: &Expr,
        depth: usize,
        apart: bool,
    ) -> Result<Option<Expr>, EvalError> {
        self.ctx.tick()?;
        if depth > 32 {
            return Ok(None);
        }
        if let Some(c) = super::integral_poly::coefficients(e, &self.x, self.ctx)? {
            return Ok(Some(super::integral_poly::primitive(&c, &self.x)));
        }
        if e.is_head(B::PLUS) {
            let mut out = vec![];
            for term in e.args() {
                let Some(p) = self.integrate(term, depth + 1, apart)? else {
                    return Ok(None);
                };
                out.push(p);
            }
            return Ok(Some(add(out)));
        }
        let factors = if e.is_head(B::TIMES) {
            e.args().to_vec()
        } else {
            vec![e.clone()]
        };
        let constants: Vec<_> = factors
            .iter()
            .filter(|f| f.free_of(&self.x))
            .cloned()
            .collect();
        let dependent: Vec<_> = factors
            .iter()
            .filter(|f| !f.free_of(&self.x))
            .cloned()
            .collect();
        if !constants.is_empty() {
            let Some(p) = self.integrate(&mul(dependent), depth + 1, apart)? else {
                return Ok(None);
            };
            return Ok(Some(mul([mul(constants), p])));
        }
        if let Some((a, center, constant)) = self.gaussian(e)? {
            let root = sqrt(a);
            return Ok(Some(mul([
                Expr::call(B::EXP, [constant]),
                div(sqrt(Expr::sym(B::PI)), mul([Expr::int(2), root.clone()])),
                Expr::call(
                    om_core::Symbol::intern("Erf"),
                    [mul([root, add([self.x.clone(), neg(center)])])],
                ),
            ])));
        }
        if let Some(p) = super::integral_rational::term(e, &self.x, self.ctx)? {
            return Ok(Some(p));
        }
        if let Some((primitive, u)) = self.template(e)?
            && let Some((slope, _)) = super::integral_poly::affine(&u, &self.x, self.ctx)?
        {
            self.nonzero(slope.clone())?;
            return Ok(Some(div(primitive, slope)));
        }
        if e.is_head(B::TIMES) {
            for (index, factor) in factors.iter().enumerate() {
                self.ctx.tick()?;
                if let Some((primitive, u)) = self.template(factor)? {
                    let d = crate::algebra::differentiate(&u, &self.x, self.ctx)?;
                    if !d.is_zero() {
                        let rest = mul(factors
                            .iter()
                            .enumerate()
                            .filter(|(i, _)| *i != index)
                            .map(|(_, f)| f.clone()));
                        let quotient = div(rest, d);
                        let ratio = om_simplify::algebra::cancel_with(
                            &quotient,
                            std::slice::from_ref(&self.x),
                            self.ctx,
                        )?
                        .unwrap_or(quotient);
                        if ratio.free_of(&self.x) {
                            return Ok(Some(mul([ratio, primitive])));
                        }
                    }
                }
            }
            for (index, factor) in factors.iter().enumerate() {
                self.ctx.tick()?;
                let name = factor.head_symbol().map(|h| h.name());
                if !(matches!(name, Some("Exp" | "Sin" | "Cos"))
                    || (factor.is_head(B::POWER)
                        && factor.args().len() == 2
                        && factor.args()[0] == Expr::sym(B::E)))
                {
                    continue;
                }
                let rest = mul(factors
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| *i != index)
                    .map(|(_, f)| f.clone()));
                let Some(poly) = super::integral_poly::coefficients(&rest, &self.x, self.ctx)?
                else {
                    continue;
                };
                if !(2..=17).contains(&poly.len()) {
                    continue;
                }
                let Some(v) = self.integrate(factor, depth + 1, false)? else {
                    continue;
                };
                let d = crate::algebra::differentiate(&rest, &self.x, self.ctx)?;
                let Some(tail) = self.integrate(&mul([d, v.clone()]), depth + 1, false)? else {
                    continue;
                };
                return Ok(Some(add([mul([rest, v]), neg(tail)])));
            }
            for (index, factor) in factors.iter().enumerate() {
                if !factor.is_head(B::LOG) || factor.args().len() != 1 {
                    continue;
                }
                let rest = mul(factors
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| *i != index)
                    .map(|(_, f)| f.clone()));
                let Some(poly) = super::integral_poly::coefficients(&rest, &self.x, self.ctx)?
                else {
                    continue;
                };
                if poly.len() > 17 {
                    continue;
                }
                let v = super::integral_poly::primitive(&poly, &self.x);
                let d = crate::algebra::differentiate(factor, &self.x, self.ctx)?;
                let tail_expr = om_simplify::algebra::cancel_with(
                    &mul([v.clone(), d]),
                    std::slice::from_ref(&self.x),
                    self.ctx,
                )?
                .ok_or_else(|| error("分部积分尾项未完成精确有理化"))?;
                if let Some(tail) = self.integrate(&tail_expr, depth + 1, apart)? {
                    return Ok(Some(add([mul([v, factor.clone()]), neg(tail)])));
                }
            }
        }
        if apart
            && let Some(parts) = crate::algebra::partial_fractions(e, &self.x, self.ctx)?
            && parts != *e
        {
            return self.integrate(&parts, depth + 1, false);
        }
        Ok(None)
    }
}
