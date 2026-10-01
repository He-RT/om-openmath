//! Exact FGLM changes order via the finite quotient algebra, without recomputing a basis.
mod echelon;
use crate::{MPoly, MonoOrder, Monomial, groebner::compatible, normal_form, s_polynomial};
use om_num::{
    Rational,
    ctx::{Abort, Interrupt},
};
use std::collections::{BTreeMap, BTreeSet};
type Q = MPoly<Rational>;

/// Convert a finite-dimensional quotient's Groebner basis to a reduced monic lex basis.
/// Exact NF/echelon certificates preserve nonradical multiplicities. None rejects empty,
/// positive-dimensional, non-Groebner or incompatible input, and monomial degree overflow.
/// Unit ideals return [1]. Output is ordered by ascending lex leading monomial.
pub fn fglm(basis: &[Q], ctx: &Interrupt) -> Result<Option<Vec<Q>>, Abort> {
    ctx.tick()?;
    let Some(order) = basis.first().map(|g| g.order) else {
        return Ok(None);
    };
    if basis.iter().any(|g| g.nvars != 0 && g.order != order) {
        return Ok(None);
    }
    let Some(all) = compatible(basis, order, ctx)? else {
        return Ok(None);
    };
    let nvars = all.first().expect("invariant: nonempty FGLM input").nvars;
    let basis: Vec<_> = all.into_iter().filter(|g| !g.is_zero()).collect();
    if basis.iter().any(|g| g.terms[0].0.deg == 0) {
        let unit = Monomial::new(vec![0; nvars]).expect("invariant: degree-zero monomial");
        return Ok(Some(vec![Q::new(
            nvars,
            vec![(unit, Rational::ONE)],
            MonoOrder::Lex,
            ctx,
        )?]));
    }
    for i in 0..basis.len() {
        for j in 0..i {
            ctx.tick()?;
            let Some(s) = s_polynomial(&basis[i], &basis[j], ctx)? else {
                return Ok(None);
            };
            if normal_form(&s, &basis, ctx)?.is_none_or(|r| !r.is_zero()) {
                return Ok(None);
            }
        }
    }
    let Some(standard) = standard(&basis, nvars, ctx)? else {
        return Ok(None);
    };
    let mut positions = BTreeMap::new();
    for (i, m) in standard.iter().enumerate() {
        ctx.tick()?;
        positions.insert(m.exps.to_vec(), i);
    }
    let mut rows = echelon::Echelon::new();
    let mut independent: Vec<Monomial> = vec![];
    let mut result: Vec<Q> = vec![];
    let mut frontier = BTreeSet::new();
    frontier.insert(vec![0; nvars]);
    let mut visited = BTreeSet::new();
    while let Some(exps) = frontier.pop_first() {
        ctx.tick()?;
        if !visited.insert(exps.clone()) {
            continue;
        }
        let Some(m) = Monomial::new(exps) else {
            return Ok(None);
        };
        if result.iter().any(|g| divides(&g.terms[0].0, &m)) {
            continue;
        }
        let term = Q::new(nvars, vec![(m.clone(), Rational::ONE)], order, ctx)?;
        let Some(nf) = normal_form(&term, &basis, ctx)? else {
            return Ok(None);
        };
        let mut vector = vec![Rational::ZERO; standard.len()];
        for (t, c) in nf.terms {
            ctx.tick()?;
            let Some(i) = positions.get(t.exps.as_slice()) else {
                return Ok(None);
            };
            vector[*i] = c;
        }
        if let Some(coefficients) = rows.reduce(vector, ctx)? {
            let mut terms = vec![(m, Rational::ONE)];
            for (t, c) in independent.iter().zip(coefficients) {
                ctx.tick()?;
                if c != Rational::ZERO {
                    terms.push((t.clone(), -c));
                }
            }
            result.push(Q::new(nvars, terms, MonoOrder::Lex, ctx)?);
        } else {
            independent.push(m.clone());
            for variable in 0..nvars {
                ctx.tick()?;
                let Some(child) = increment(&m, variable) else {
                    return Ok(None);
                };
                if !visited.contains(child.exps.as_slice()) {
                    frontier.insert(child.exps.to_vec());
                }
            }
        }
    }
    if independent.len() != standard.len() {
        return Ok(None);
    }
    // Relations can be installed before a later leading monomial divides a tail.
    // Interreduce exactly rather than invoking Buchberger for this final cleanup.
    let mut minimal = vec![];
    for (i, g) in result.iter().enumerate() {
        ctx.tick()?;
        let mut redundant = false;
        for (j, h) in result.iter().enumerate() {
            ctx.tick()?;
            if i != j && divides(&h.terms[0].0, &g.terms[0].0) {
                redundant = true;
                break;
            }
        }
        if !redundant {
            minimal.push(g.clone());
        }
    }
    let mut reduced = vec![];
    for (i, g) in minimal.iter().enumerate() {
        ctx.tick()?;
        let others: Vec<_> = minimal
            .iter()
            .enumerate()
            .filter(|(j, _)| *j != i)
            .map(|(_, g)| g.clone())
            .collect();
        let Some(h) = normal_form(g, &others, ctx)? else {
            return Ok(None);
        };
        reduced.push(h);
    }
    for i in 1..reduced.len() {
        let mut j = i;
        while j > 0 {
            ctx.tick()?;
            if !reduced[j - 1].terms[0]
                .0
                .cmp(&reduced[j].terms[0].0, MonoOrder::Lex)
                .is_gt()
            {
                break;
            }
            reduced.swap(j - 1, j);
            j -= 1;
        }
    }
    Ok(Some(reduced))
}
fn divides(a: &Monomial, b: &Monomial) -> bool {
    a.exps.iter().zip(&b.exps).all(|(a, b)| a <= b)
}
fn increment(m: &Monomial, variable: usize) -> Option<Monomial> {
    let mut exps = m.exps.clone();
    exps[variable] = exps[variable].checked_add(1)?;
    Monomial::new(exps)
}
fn standard(basis: &[Q], nvars: usize, ctx: &Interrupt) -> Result<Option<Vec<Monomial>>, Abort> {
    // Pure leading powers bound every variable, proving enumeration terminates.
    for variable in 0..nvars {
        ctx.tick()?;
        let mut bounded = false;
        for g in basis {
            ctx.tick()?;
            let m = &g.terms[0].0;
            if m.exps[variable] > 0
                && m.exps
                    .iter()
                    .enumerate()
                    .all(|(j, e)| j == variable || *e == 0)
            {
                bounded = true;
                break;
            }
        }
        if !bounded {
            return Ok(None);
        }
    }
    let mut frontier = BTreeSet::new();
    frontier.insert(vec![0; nvars]);
    let mut seen = BTreeSet::new();
    let mut out = vec![];
    while let Some(exps) = frontier.pop_first() {
        ctx.tick()?;
        if !seen.insert(exps.clone()) {
            continue;
        }
        let Some(m) = Monomial::new(exps) else {
            return Ok(None);
        };
        if basis.iter().any(|g| divides(&g.terms[0].0, &m)) {
            continue;
        }
        for j in 0..nvars {
            ctx.tick()?;
            let Some(child) = increment(&m, j) else {
                return Ok(None);
            };
            if !seen.contains(child.exps.as_slice()) {
                frontier.insert(child.exps.to_vec());
            }
        }
        out.push(m);
    }
    let order = basis.first().map_or(MonoOrder::GrevLex, |g| g.order);
    for i in 1..out.len() {
        let mut j = i;
        while j > 0 {
            ctx.tick()?;
            if !out[j - 1].cmp(&out[j], order).is_gt() {
                break;
            }
            out.swap(j - 1, j);
            j -= 1;
        }
    }
    Ok(Some(out))
}
