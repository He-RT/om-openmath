//! Products reach a fixed point after exponent and numeric-radical merging.

use super::{
    add,
    order::canonical_cmp,
    pow,
    special::{infinity_direction, literal_alias, rational},
};
use crate::{BUILTIN as B, Expr};
use om_num::{Number, Rational};

/// Construct a canonical product, collecting coefficients and adding like-base exponents.
pub fn mul(factors: impl IntoIterator<Item = Expr>) -> Expr {
    let mut product = Product {
        coefficient: Number::Integer(1.into()),
        factors: Vec::new(),
        infinities: Vec::new(),
    };
    if let Err(e) = product.absorb(factors) {
        return e;
    }
    loop {
        if product.coefficient.is_zero() {
            return product.zero();
        }
        let previous = product.factors.clone();
        let mut powers: Vec<_> = product.factors.drain(..).map(base_exp).collect();
        powers.sort_by(|(a, _), (b, _)| canonical_cmp(a, b));
        let mut groups: Vec<(Expr, Vec<Expr>)> = Vec::with_capacity(powers.len());
        for (base, exp) in powers {
            if let Some((last, exps)) = groups.last_mut()
                && *last == base
            {
                exps.push(exp);
            } else {
                groups.push((base, vec![exp]));
            }
        }
        let rebuilt = groups.into_iter().map(|(base, exps)| pow(base, add(exps)));
        if let Err(e) = product.absorb(rebuilt) {
            return e;
        }
        if let Some((a, b, merged)) = merge_radicals(&product.factors) {
            product.factors.remove(b);
            product.factors.remove(a);
            if let Err(e) = product.absorb([merged]) {
                return e;
            }
            continue;
        }
        if product.factors == previous {
            break;
        }
    }
    product.finish()
}

struct Product {
    coefficient: Number,
    factors: Vec<Expr>,
    infinities: Vec<Option<Expr>>,
}
impl Product {
    fn absorb(&mut self, args: impl IntoIterator<Item = Expr>) -> Result<(), Expr> {
        let mut stack: Vec<_> = args.into_iter().collect();
        stack.reverse();
        while let Some(e) = stack.pop() {
            let e = literal_alias(e);
            if e.is_head(B::TIMES) {
                stack.extend(e.args().iter().rev().cloned());
            } else if e.as_symbol() == Some(B::INDETERMINATE) {
                return Err(e);
            } else if let Some(direction) = infinity_direction(&e) {
                self.infinities.push(direction);
            } else if let Some(n) = e.as_number() {
                self.coefficient = self.coefficient.mul(n);
            } else {
                self.factors.push(e);
            }
        }
        Ok(())
    }
    fn zero(&self) -> Expr {
        if self.infinities.is_empty() {
            Expr::number(self.coefficient.clone())
        } else {
            Expr::sym(B::INDETERMINATE)
        }
    }
    fn finish(mut self) -> Expr {
        if self.coefficient.is_zero() {
            return self.zero();
        }
        if !self.infinities.is_empty() {
            if self.infinities.iter().any(Option::is_none) {
                return Expr::call(B::DIRECTED_INFINITY, []);
            }
            let mut directions = vec![phase(Expr::number(self.coefficient))];
            directions.extend(self.infinities.into_iter().flatten().map(phase));
            let direction = phase(mul(directions));
            if direction.as_symbol() == Some(B::INDETERMINATE) {
                return direction;
            }
            self.factors
                .push(Expr::call(B::DIRECTED_INFINITY, [direction]));
            self.coefficient = Number::Integer(1.into());
        }
        if self.coefficient == Number::Integer((-1).into())
            && let [sum] = self.factors.as_slice()
            && sum.is_head(B::PLUS)
        {
            return add(sum.args().iter().map(|e| mul([Expr::int(-1), e.clone()])));
        }
        self.factors.sort_by(canonical_cmp);
        if !self.coefficient.is_exact() || !self.coefficient.is_one() {
            self.factors.insert(0, Expr::number(self.coefficient));
        }
        match self.factors.len() {
            0 => Expr::int(1),
            1 => self.factors.remove(0),
            _ => Expr::call(B::TIMES, self.factors),
        }
    }
}

fn base_exp(e: Expr) -> (Expr, Expr) {
    if e.is_head(B::POWER) && e.args().len() == 2 {
        (e.args()[0].clone(), e.args()[1].clone())
    } else {
        (e, Expr::int(1))
    }
}

fn radical(e: &Expr) -> Option<(Rational, Rational)> {
    if e.is_head(B::POWER)
        && let [base, exp] = e.args()
        && let Some(base) = base.as_number().and_then(rational)
        && base > Rational::ZERO
        && let Some(Number::Rational(exp)) = exp.as_number()
    {
        Some((base, exp.clone()))
    } else {
        None
    }
}
fn merge_radicals(factors: &[Expr]) -> Option<(usize, usize, Expr)> {
    for (i, a) in factors.iter().enumerate() {
        let Some((ar, ae)) = radical(a) else {
            continue;
        };
        for (j, b) in factors.iter().enumerate().skip(i + 1) {
            let Some((br, be)) = radical(b) else {
                continue;
            };
            let (base, exp) = if ae == be {
                (&ar * &br, ae.clone())
            } else if ae == -&be {
                if ae > Rational::ZERO {
                    (&ar / &br, ae.clone())
                } else {
                    (&br / &ar, be)
                }
            } else {
                continue;
            };
            return Some((
                i,
                j,
                pow(
                    Expr::number(Number::Rational(base)),
                    Expr::number(Number::Rational(exp)),
                ),
            ));
        }
    }
    None
}

pub(super) fn phase(e: Expr) -> Expr {
    if e.is_head(B::SIGN) && e.args().len() == 1 {
        return e;
    }
    if e.is_head(B::TIMES)
        && let [coefficient, root] = e.args()
        && let Some(Number::Complex(c)) = coefficient.as_number()
        && c.re.is_exact()
        && c.im.is_exact()
        && root.is_head(B::POWER)
        && let [base, exponent] = root.args()
        && *exponent == Expr::rational(-1, 2)
        && base.as_number() == Some(&c.re.mul(&c.re).add(&c.im.mul(&c.im)))
    {
        return e;
    }
    let Some(n) = e.as_number() else {
        return Expr::call(B::SIGN, [e]);
    };
    if n.is_zero() {
        return Expr::sym(B::INDETERMINATE);
    }
    if let Number::Complex(c) = n {
        if !n.is_exact() {
            return Expr::call(B::SIGN, [e]);
        }
        let norm = c.re.mul(&c.re).add(&c.im.mul(&c.im));
        mul([e.clone(), pow(Expr::number(norm), Expr::rational(-1, 2))])
    } else {
        Expr::int(if n.is_negative() { -1 } else { 1 })
    }
}
