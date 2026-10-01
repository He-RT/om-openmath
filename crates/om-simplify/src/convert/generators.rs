//! Fractional-base discovery precedes ordinary traversal, avoiding unused interior axes.
use super::{exact, integer_power};
use om_core::{BUILTIN as B, Expr, canonical_cmp, pow};
use om_num::{
    Integer, Number, Rational,
    ctx::{Abort, Interrupt},
    gcd,
};
#[derive(Clone)]
pub(super) struct Group {
    pub base: Expr,
    pub denominator: Integer,
    pub requested: bool,
}
pub(super) fn discover(
    e: &Expr,
    vars: &[Expr],
    ctx: &Interrupt,
) -> Result<Option<Vec<Group>>, Abort> {
    let mut groups = vec![];
    for v in vars {
        ctx.tick()?;
        if exact(v).is_some() {
            continue;
        }
        if let Some((base, q)) = fractional(v) {
            register(&mut groups, base, q, true, ctx)?;
        } else {
            register(&mut groups, v.clone(), Integer::ONE, true, ctx)?;
        }
    }
    for fractions_only in [true, false] {
        let mut stack = vec![e];
        while let Some(e) = stack.pop() {
            ctx.tick()?;
            if exact(e).is_some() {
                continue;
            }
            if groups.iter().any(|g| g.base == *e) {
                continue;
            }
            if e.is_head(B::PLUS) || e.is_head(B::TIMES) {
                stack.extend(e.args().iter().rev());
            } else if let Some((base, q)) = fractional(e) {
                register(&mut groups, base, q, false, ctx)?;
            } else if integer_power(e).is_some() {
                stack.push(&e.args()[0]);
            } else if !fractions_only {
                register(&mut groups, e.clone(), Integer::ONE, false, ctx)?;
            }
        }
    }
    for group in &groups {
        ctx.tick()?;
        if u32::try_from(&group.denominator).is_err() {
            return Ok(None);
        }
    }
    // Preserve requested priority while canonically sorting newly discovered axes.
    for i in 1..groups.len() {
        let mut j = i;
        while j > 0 {
            ctx.tick()?;
            let a = &groups[j - 1];
            let b = &groups[j];
            let order = match (a.requested, b.requested) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                (true, true) => std::cmp::Ordering::Less,
                (false, false) => canonical_cmp(&generator(a), &generator(b)),
            };
            if !order.is_gt() {
                break;
            }
            groups.swap(j - 1, j);
            j -= 1;
        }
    }
    Ok(Some(groups))
}
fn fractional(e: &Expr) -> Option<(Expr, Integer)> {
    if !e.is_head(B::POWER) || e.args().len() != 2 {
        return None;
    }
    let q = exact(&e.args()[1])?;
    (q.denominator() != &1_u8.into())
        .then(|| (e.args()[0].clone(), Integer::from(q.denominator().clone())))
}
fn register(
    groups: &mut Vec<Group>,
    base: Expr,
    den: Integer,
    requested: bool,
    ctx: &Interrupt,
) -> Result<(), Abort> {
    for group in groups.iter_mut() {
        ctx.tick()?;
        if group.base == base {
            group.denominator = (&group.denominator / gcd(&group.denominator, &den)) * den;
            group.requested |= requested;
            return Ok(());
        }
    }
    groups.push(Group {
        base,
        denominator: den,
        requested,
    });
    Ok(())
}
pub(super) fn generator(group: &Group) -> Expr {
    if group.denominator == Integer::ONE {
        group.base.clone()
    } else {
        pow(
            group.base.clone(),
            Expr::number(Number::Rational(
                Rational::ONE / Rational::from(group.denominator.clone()),
            )),
        )
    }
}
pub(super) fn axis(
    e: &Expr,
    groups: &[Group],
    ctx: &Interrupt,
) -> Result<Option<(usize, Integer)>, Abort> {
    for (i, g) in groups.iter().enumerate() {
        ctx.tick()?;
        if g.base == *e {
            return Ok(Some((i, g.denominator.clone())));
        }
    }
    if e.is_head(B::POWER)
        && e.args().len() == 2
        && let Some(q) = exact(&e.args()[1])
    {
        for (i, g) in groups.iter().enumerate() {
            ctx.tick()?;
            if g.base == e.args()[0] {
                return Ok(Some((
                    i,
                    q.numerator() * (&g.denominator / Integer::from(q.denominator().clone())),
                )));
            }
        }
    }
    Ok(None)
}
