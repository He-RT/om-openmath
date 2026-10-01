//! Gebauer-Moller installation with stable arena ids and deterministic sugar selection.
use super::{
    Entry,
    arithmetic::{coprime, divides, lcm},
};
use crate::Monomial;
use om_num::ctx::{Abort, Interrupt};
#[derive(Clone)]
pub(super) struct Pair {
    pub a: usize,
    pub b: usize,
    pub lcm: Monomial,
    pub sugar: u64,
    pub serial: usize,
}
fn leading(arena: &[Entry], i: usize) -> &Monomial {
    &arena[i].poly.terms[0].0
}
pub(super) fn update(
    arena: &[Entry],
    active: &mut Vec<usize>,
    pairs: &mut Vec<Pair>,
    h: usize,
    serial: &mut usize,
    ctx: &Interrupt,
) -> Result<Option<()>, Abort> {
    let mh = leading(arena, h);
    let mut candidates = vec![];
    for &g in active.iter() {
        ctx.tick()?;
        let Some(l) = lcm(mh, leading(arena, g)) else {
            return Ok(None);
        };
        candidates.push((g, l));
    }
    let mut retained: Vec<(usize, Monomial)> = vec![];
    for i in 0..candidates.len() {
        ctx.tick()?;
        let (g, l) = &candidates[i];
        let disjoint = coprime(mh, leading(arena, *g));
        let mut redundant = false;
        for (_, m) in candidates[i + 1..].iter().chain(retained.iter()) {
            ctx.tick()?;
            if divides(m, l) {
                redundant = true;
                break;
            }
        }
        if disjoint || !redundant {
            retained.push((*g, l.clone()));
        }
    }
    let mut old = vec![];
    for pair in pairs.drain(..) {
        ctx.tick()?;
        let (Some(ah), Some(bh)) = (
            lcm(leading(arena, pair.a), mh),
            lcm(leading(arena, pair.b), mh),
        ) else {
            return Ok(None);
        };
        if !divides(mh, &pair.lcm) || ah == pair.lcm || bh == pair.lcm {
            old.push(pair);
        }
    }
    for (g, l) in retained {
        ctx.tick()?;
        if coprime(mh, leading(arena, g)) {
            continue;
        }
        let sugar = (arena[h].sugar + u64::from(l.deg - mh.deg))
            .max(arena[g].sugar + u64::from(l.deg - leading(arena, g).deg));
        old.push(Pair {
            a: g,
            b: h,
            lcm: l,
            sugar,
            serial: *serial,
        });
        *serial += 1;
    }
    *pairs = old;
    let mut next = vec![];
    for &g in active.iter() {
        ctx.tick()?;
        if !divides(mh, leading(arena, g)) {
            next.push(g);
        }
    }
    next.push(h);
    *active = next;
    Ok(Some(()))
}
pub(super) fn select(
    pairs: &mut Vec<Pair>,
    order: crate::MonoOrder,
    ctx: &Interrupt,
) -> Result<Option<Pair>, Abort> {
    let mut best = None;
    for (i, p) in pairs.iter().enumerate() {
        ctx.tick()?;
        if best.is_none_or(|j: usize| {
            let q = &pairs[j];
            p.sugar
                .cmp(&q.sugar)
                .then_with(|| p.lcm.cmp(&q.lcm, order))
                .then_with(|| p.serial.cmp(&q.serial))
                .is_lt()
        }) {
            best = Some(i);
        }
    }
    Ok(best.map(|i| pairs.remove(i)))
}
