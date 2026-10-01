//! Certified realness intersects an enclosure without changing its computed real part.
use super::{elementary, zero};
use om_core::{BUILTIN as B, Expr, mul};
use om_num::{
    CBall, Rational,
    ctx::{Abort, Interrupt},
};
use om_poly::Algebraic;
use std::{
    cell::{Cell, RefCell},
    collections::VecDeque,
};

const CAPACITY: usize = 256;
thread_local! {
    static REAL: RefCell<VecDeque<Expr>> = const { RefCell::new(VecDeque::new()) };
    static PROVING: Cell<bool> = const { Cell::new(false) };
}

/// Remember realness of a constant expression associated with a certified real value.
/// The caller must establish expression/value equality by exact construction and
/// certified root association. Only realness is retained, never a replacement value.
/// Symbolic expressions and certified complex values are rejected.
pub fn remember_real(e: &Expr, value: &Algebraic) -> bool {
    if !e.free_symbols().is_empty() || matches!(value, Algebraic::Complex(_)) {
        return false;
    }
    REAL.with(|memo| {
        let mut memo = memo.borrow_mut();
        if !memo.contains(e) {
            if memo.len() == CAPACITY {
                memo.pop_front();
            }
            memo.push_back(e.clone());
        }
    });
    true
}
fn remembered(e: &Expr) -> bool {
    REAL.with(|memo| memo.borrow().contains(e))
}
fn term(e: &Expr) -> (Expr, Rational) {
    if e.is_head(B::TIMES)
        && let Some(q) = e.args().first().and_then(crate::convert::exact)
    {
        return (mul(e.args()[1..].iter().cloned()), q);
    }
    (e.clone(), Rational::ONE)
}
pub(super) fn project_sum(
    e: &Expr,
    args: &[CBall],
    z: CBall,
    ctx: &Interrupt,
) -> Result<CBall, Abort> {
    if !e.is_head(B::PLUS) || zero(&z.im) || !z.im.contains_zero() {
        return Ok(project(e, z));
    }
    let mut terms = e
        .args()
        .iter()
        .zip(args)
        .filter(|(e, _)| crate::convert::exact(e).is_none())
        .map(|(e, b)| {
            let (kernel, q) = term(e);
            (kernel, q, zero(&b.im))
        })
        .collect::<Vec<_>>();
    let proven = REAL.with(|memo| -> Result<bool, Abort> {
        for known in memo.borrow().iter().filter(|e| e.is_head(B::PLUS)) {
            ctx.tick()?;
            let relation = known
                .args()
                .iter()
                .filter(|e| crate::convert::exact(e).is_none())
                .map(term)
                .filter(|(_, q)| q != &Rational::ZERO)
                .collect::<Vec<_>>();
            let Some(indices) = relation
                .iter()
                .map(|(kernel, _)| terms.iter().position(|(k, _, _)| k == kernel))
                .collect::<Option<Vec<_>>>()
            else {
                continue;
            };
            let Some(pivot) = indices
                .iter()
                .position(|i| !terms[*i].2 && terms[*i].1 != Rational::ZERO)
            else {
                continue;
            };
            let scale = &terms[indices[pivot]].1 / &relation[pivot].1;
            for ((_, q), i) in relation.iter().zip(indices) {
                ctx.tick()?;
                terms[i].1 -= &scale * q;
            }
        }
        Ok(terms
            .iter()
            .all(|(_, q, real)| *real || q == &Rational::ZERO))
    })?;
    if proven {
        Ok(elementary::real(z.re))
    } else {
        Ok(project(e, z))
    }
}
pub(super) fn project(e: &Expr, z: CBall) -> CBall {
    if !zero(&z.im) && z.im.contains_zero() && remembered(e) {
        // Intersect with a proven real axis; do not infer realness from a small radius.
        elementary::real(z.re)
    } else {
        z
    }
}
struct ProofGuard;
impl Drop for ProofGuard {
    fn drop(&mut self) {
        PROVING.with(|flag| flag.set(false));
    }
}
pub(super) fn resolve(e: &Expr, z: CBall, ctx: &Interrupt) -> Result<CBall, Abort> {
    let z = project(e, z);
    if zero(&z.im)
        || !z.im.contains_zero()
        || z.re.mid > z.re.rad
        || !e.free_symbols().is_empty()
        || PROVING.with(|flag| flag.replace(true))
    {
        return Ok(z);
    }
    let _guard = ProofGuard;
    // Optional branch proof must not spend the entire numerical caller budget on
    // dependent-radical resultants. Charge capped work and propagate real aborts.
    const LIMIT: u64 = 32_768;
    let available = ctx.steps_left.get();
    let budget = available.min(LIMIT);
    let local = Interrupt {
        flag: ctx.flag.clone(),
        deadline_ms: ctx.deadline_ms,
        clock: ctx.clock.clone(),
        steps_left: Cell::new(budget),
    };
    let result = crate::root_reduce::to_algebraic(e, &local);
    ctx.steps_left
        .set(available - (budget - local.steps_left.get()));
    let result = match result {
        Err(Abort::Budget) if available > LIMIT => Ok(None),
        result => result,
    };
    if let Some(value) = result?
        && remember_real(e, &value)
    {
        return Ok(project(e, z));
    }
    Ok(z)
}
