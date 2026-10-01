//! Integer primitive Buchberger/Gebauer-Moller with exact monic rational output.
mod arithmetic;
mod pairs;
use crate::{MonoOrder, Monomial};
use arithmetic::{
    Q, Z, divides, primitive, quotient, reduce, shift, to_integer, to_rational, z_spoly,
};
use om_num::{
    Rational,
    ctx::{Abort, Interrupt},
};
#[derive(Clone)]
struct Entry {
    poly: Z,
    sugar: u64,
}

/// Compute the reduced monic Q Groebner basis in ascending leading-monomial order.
/// Inputs may use a different order; coefficients are cleared to primitive integers
/// throughout Buchberger reduction. None rejects incompatible variable contexts or
/// monomial degree overflow. Zero/empty ideals return []; unit ideals return [1].
pub fn groebner(inputs: &[Q], order: MonoOrder, ctx: &Interrupt) -> Result<Option<Vec<Q>>, Abort> {
    ctx.tick()?;
    let Some(inputs) = prepare(inputs, order, ctx)? else {
        return Ok(None);
    };
    let mut arena: Vec<Entry> = vec![];
    let mut active = vec![];
    let mut pairs = vec![];
    let mut serial = 0;
    for f in inputs {
        ctx.tick()?;
        let sugar = f
            .terms
            .iter()
            .map(|(m, _)| u64::from(m.deg))
            .max()
            .unwrap_or(0);
        let reducers: Vec<_> = active
            .iter()
            .map(|i: &usize| (&arena[*i].poly, arena[*i].sugar))
            .collect();
        let Some((h, sugar)) = reduce(f, &reducers, sugar, ctx)? else {
            return Ok(None);
        };
        if h.is_zero() {
            continue;
        }
        if h.terms[0].0.deg == 0 {
            return Ok(Some(vec![to_rational(&h, ctx)?]));
        }
        let id = arena.len();
        arena.push(Entry { poly: h, sugar });
        if pairs::update(&arena, &mut active, &mut pairs, id, &mut serial, ctx)?.is_none() {
            return Ok(None);
        }
    }
    while let Some(pair) = pairs::select(&mut pairs, order, ctx)? {
        ctx.tick()?;
        let Some(s) = z_spoly(&arena[pair.a].poly, &arena[pair.b].poly, ctx)? else {
            return Ok(None);
        };
        let reducers: Vec<_> = active
            .iter()
            .map(|i| (&arena[*i].poly, arena[*i].sugar))
            .collect();
        let Some((h, sugar)) = reduce(s, &reducers, pair.sugar, ctx)? else {
            return Ok(None);
        };
        if h.is_zero() {
            continue;
        }
        if h.terms[0].0.deg == 0 {
            return Ok(Some(vec![to_rational(&h, ctx)?]));
        }
        let id = arena.len();
        arena.push(Entry { poly: h, sugar });
        if pairs::update(&arena, &mut active, &mut pairs, id, &mut serial, ctx)?.is_none() {
            return Ok(None);
        }
    }
    finish(&arena, &active, ctx)
}
/// Exact unscaled multivariate rational normal form in the input monomial order.
/// Reducers are tried in slice order; None rejects incompatible contexts or degree overflow.
pub fn normal_form(f: &Q, basis: &[Q], ctx: &Interrupt) -> Result<Option<Q>, Abort> {
    ctx.tick()?;
    let mut all = vec![f.clone()];
    all.extend_from_slice(basis);
    let Some(mut all) = compatible(&all, f.order, ctx)? else {
        return Ok(None);
    };
    let mut p = all.remove(0);
    let mut out = Q::zero_in(p.nvars, p.order);
    while let Some((m, c)) = p.terms.first().cloned() {
        ctx.tick()?;
        let mut reducer = None;
        for g in &all {
            ctx.tick()?;
            if let Some((n, b)) = g.terms.first()
                && let Some(d) = quotient(&m, n)
            {
                reducer = Some((g, d, &c / b));
                break;
            }
        }
        if let Some((g, m, c)) = reducer {
            let Some(t) = shift(g, &m, &c, ctx)? else {
                return Ok(None);
            };
            p = p.sub(&t, ctx)?;
        } else {
            let t = Q::new(p.nvars, vec![(m, c)], p.order, ctx)?;
            out = out.add(&t, ctx)?;
            p = p.sub(&t, ctx)?;
        }
    }
    Ok(Some(out))
}
/// Exact S-polynomial with leading-monomial cancellation, over compatible Q rings.
/// A zero operand yields zero; degree overflow or incompatible contexts return None.
pub fn s_polynomial(f: &Q, g: &Q, ctx: &Interrupt) -> Result<Option<Q>, Abort> {
    ctx.tick()?;
    let Some(all) = compatible(&[f.clone(), g.clone()], f.order, ctx)? else {
        return Ok(None);
    };
    let (f, g) = (&all[0], &all[1]);
    let (Some((m, a)), Some((n, b))) = (f.terms.first(), g.terms.first()) else {
        return Ok(Some(Q::zero_in(f.nvars, f.order)));
    };
    let Some(l) = arithmetic::lcm(m, n) else {
        return Ok(None);
    };
    let (lm, ln) = (
        quotient(&l, m).expect("invariant: lcm divisible by first LM"),
        quotient(&l, n).expect("invariant: lcm divisible by second LM"),
    );
    let (Some(left), Some(right)) = (
        shift(f, &lm, &(Rational::ONE / a), ctx)?,
        shift(g, &ln, &(Rational::ONE / b), ctx)?,
    ) else {
        return Ok(None);
    };
    Ok(Some(left.sub(&right, ctx)?))
}
pub(super) fn compatible(
    inputs: &[Q],
    order: MonoOrder,
    ctx: &Interrupt,
) -> Result<Option<Vec<Q>>, Abort> {
    let nvars = inputs.iter().find(|f| f.nvars > 0).map_or(0, |f| f.nvars);
    let mut out = vec![];
    for f in inputs {
        ctx.tick()?;
        if f.nvars != 0 && f.nvars != nvars {
            return Ok(None);
        }
        let mut terms = vec![];
        for (m, c) in &f.terms {
            ctx.tick()?;
            let mut m = m.clone();
            if f.nvars == 0 {
                if m.deg != 0 || !m.exps.is_empty() {
                    return Ok(None);
                }
                m.exps.resize(nvars, 0);
            }
            if m.exps.len() != nvars || Monomial::new(m.exps.iter().copied()).as_ref() != Some(&m) {
                return Ok(None);
            }
            terms.push((m, c.clone()));
        }
        out.push(Q::new(nvars, terms, order, ctx)?);
    }
    Ok(Some(out))
}
fn prepare(inputs: &[Q], order: MonoOrder, ctx: &Interrupt) -> Result<Option<Vec<Z>>, Abort> {
    let Some(inputs) = compatible(inputs, order, ctx)? else {
        return Ok(None);
    };
    let mut out = vec![];
    for f in inputs {
        ctx.tick()?;
        if !f.is_zero() {
            out.push(to_integer(&f, ctx)?);
        }
    }
    // Input permutation should not choose a different installation order. Compare
    // all terms after LM, so tied leading monomials do not fall back to input order.
    for i in 1..out.len() {
        let mut j = i;
        while j > 0 {
            ctx.tick()?;
            if !compare(&out[j - 1], &out[j]).is_gt() {
                break;
            }
            out.swap(j - 1, j);
            j -= 1;
        }
    }
    Ok(Some(out))
}
fn compare(a: &Z, b: &Z) -> std::cmp::Ordering {
    for ((m, c), (n, d)) in a.terms.iter().zip(&b.terms) {
        let order = m.cmp(n, a.order).then_with(|| c.cmp(d));
        if !order.is_eq() {
            return order;
        }
    }
    a.terms.len().cmp(&b.terms.len())
}
fn finish(arena: &[Entry], active: &[usize], ctx: &Interrupt) -> Result<Option<Vec<Q>>, Abort> {
    let mut minimal = vec![];
    for (i, &id) in active.iter().enumerate() {
        ctx.tick()?;
        let m = &arena[id].poly.terms[0].0;
        let mut redundant = false;
        for (j, &other) in active.iter().enumerate() {
            ctx.tick()?;
            if j != i && divides(&arena[other].poly.terms[0].0, m) {
                redundant = true;
                break;
            }
        }
        if !redundant {
            minimal.push(id);
        }
    }
    let mut out = vec![];
    for &id in &minimal {
        ctx.tick()?;
        let reducers: Vec<_> = minimal
            .iter()
            .filter(|other| **other != id)
            .map(|i| (&arena[*i].poly, arena[*i].sugar))
            .collect();
        let Some((f, _)) = reduce(arena[id].poly.clone(), &reducers, arena[id].sugar, ctx)? else {
            return Ok(None);
        };
        out.push(to_rational(&primitive(f, ctx)?, ctx)?);
    }
    for i in 1..out.len() {
        let mut j = i;
        while j > 0 {
            ctx.tick()?;
            if !out[j - 1].terms[0]
                .0
                .cmp(&out[j].terms[0].0, out[j].order)
                .is_gt()
            {
                break;
            }
            out.swap(j - 1, j);
            j -= 1;
        }
    }
    Ok(Some(out))
}
