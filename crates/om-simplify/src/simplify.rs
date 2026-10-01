//! Deterministic bounded best-first search over exact, branch-safe neighborhoods.
mod rules;
use crate::{algebra, convert::normalize, zero::radicals::rewrite};
use om_core::{Expr, ExprKind, Symbol, canonicalize};
use om_num::{
    Integer, Number,
    ctx::{Abort, Interrupt},
};
use std::{
    cmp::Ordering,
    collections::{BTreeMap, BinaryHeap, hash_map::DefaultHasher},
    hash::{Hash, Hasher},
};

/// Explicit assumptions accepted by the safe PowerExpand neighborhood.
#[derive(Clone, Debug, Default)]
pub struct SimplifyOptions {
    /// Variables asserted to have positive real values; other symbols are unconstrained.
    pub positive: Vec<Symbol>,
}
/// Weighted LeafCount in exact quarter units; Root has cost twelve.
/// Integers contribute 4+ceil(log10(abs(n))), with zero and units contributing four.
pub fn complexity(e: &Expr) -> u64 {
    cost(e, None).expect("invariant: cost without a context cannot abort")
}
fn cost(e: &Expr, ctx: Option<&Interrupt>) -> Result<u64, Abort> {
    let mut stack = vec![e];
    let mut total = 0_u64;
    while let Some(e) = stack.pop() {
        if let Some(ctx) = ctx {
            ctx.tick()?;
        }
        let weight = if e.is_head(om_core::BUILTIN::ROOT) {
            12
        } else {
            match e.kind() {
                ExprKind::Normal(n) => {
                    stack.push(&n.head);
                    stack.extend(n.args.iter());
                    0
                }
                ExprKind::Number(Number::Integer(n)) => integer_cost(n),
                _ => 4,
            }
        };
        total = total.saturating_add(weight);
    }
    Ok(total)
}
fn integer_cost(n: &Integer) -> u64 {
    if n.is_zero() {
        return 4;
    }
    let s = n.to_string();
    let s = s.trim_start_matches('-');
    let exact_ten = s.starts_with('1') && s[1..].bytes().all(|b| b == b'0');
    4 + s.len() as u64 - u64::from(exact_ten)
}
/// Search for a lower-cost equivalent expression using default unconstrained variables.
/// Aborted work preserves the canonical input; use simplify_with to observe Abort.
pub fn simplify(e: &Expr) -> Expr {
    simplify_with(e, &SimplifyOptions::default(), &Interrupt::default())
        .unwrap_or_else(|_| canonicalize(e))
}
#[derive(Clone)]
struct Candidate {
    e: Expr,
    cost: u64,
    serial: usize,
}
impl PartialEq for Candidate {
    fn eq(&self, b: &Self) -> bool {
        self.cost == b.cost && self.serial == b.serial
    }
}
impl Eq for Candidate {}
impl PartialOrd for Candidate {
    fn partial_cmp(&self, b: &Self) -> Option<Ordering> {
        Some(self.cmp(b))
    }
}
impl Ord for Candidate {
    fn cmp(&self, b: &Self) -> Ordering {
        b.cost
            .cmp(&self.cost)
            .then_with(|| b.serial.cmp(&self.serial))
    }
}
struct Search {
    frontier: BinaryHeap<Candidate>,
    seen: BTreeMap<u64, Vec<Expr>>,
    best: Candidate,
    next: usize,
    ceiling: u64,
}
impl Search {
    fn insert(&mut self, e: Expr, ctx: &Interrupt) -> Result<(), Abort> {
        ctx.tick()?;
        let candidate_cost = cost(&e, Some(ctx))?;
        if candidate_cost > self.ceiling {
            return Ok(());
        }
        let mut hash = DefaultHasher::new();
        e.hash(&mut hash);
        let bucket = self.seen.entry(hash.finish()).or_default();
        for previous in bucket.iter() {
            ctx.tick()?;
            if previous == &e {
                return Ok(());
            }
        }
        bucket.push(e.clone());
        let candidate = Candidate {
            cost: candidate_cost,
            e,
            serial: self.next,
        };
        self.next += 1;
        if candidate.cost < self.best.cost {
            self.best = candidate.clone();
        }
        self.frontier.push(candidate);
        Ok(())
    }
}
/// Explore at most fifty nodes, retaining the cheapest candidate and deterministic ties.
/// Exact arithmetic/Root conversion and assumed-positive PowerExpand are interruptible.
pub fn simplify_with(e: &Expr, options: &SimplifyOptions, ctx: &Interrupt) -> Result<Expr, Abort> {
    ctx.tick()?;
    let input = normalize(e, ctx)?;
    let best = Candidate {
        cost: cost(&input, Some(ctx))?,
        e: input.clone(),
        serial: 0,
    };
    let mut search = Search {
        frontier: BinaryHeap::new(),
        seen: BTreeMap::new(),
        ceiling: best.cost.saturating_mul(4),
        best,
        next: 1,
    };
    search.insert(input.clone(), ctx)?;
    for _ in 0..50 {
        ctx.tick()?;
        let Some(node) = search.frontier.pop() else {
            break;
        };
        if search.best.cost == 4 {
            break;
        }
        for operation in 0..13 {
            ctx.tick()?;
            let limit = if operation == 6 { 1_000_000 } else { 100_000 };
            let Some(next) = bounded_neighbor(&node.e, ctx, limit, |e, ctx| {
                let next = match operation {
                    0 => algebra::together_with(e, &[], ctx)?,
                    1 => algebra::cancel_with(e, &[], ctx)?,
                    2 => algebra::expand_with(e, ctx)?,
                    3 => algebra::factor_with(e, &[], ctx)?,
                    4 => Some(rules::factor_terms(e, ctx)?),
                    5 => Some(crate::zero::radicals::denest(e, ctx)?),
                    6 => rules::root_reduce(e, ctx)?,
                    7 => Some(rules::power_expand(e, &options.positive, ctx)?),
                    mode => Some(rules::trig(e, mode - 8, ctx)?),
                };
                Ok(next.unwrap_or_else(|| e.clone()))
            })?
            else {
                continue;
            };
            search.insert(next, ctx)?;
            if search.best.cost == 4 {
                break;
            }
        }
    }
    #[cfg(debug_assertions)]
    {
        // L3 may be unable to evaluate an opaque function. It can still refute a
        // bad rewrite; every supported sample uses the caller's positive assumptions.
        let delta = om_core::sub(input, search.best.e.clone());
        let check = crate::zero::numerical_assuming(&delta, &options.positive, ctx)?;
        debug_assert_ne!(
            check,
            crate::zero::Tri::NonZero,
            "simplification changed the value"
        );
    }
    Ok(search.best.e)
}

fn bounded_neighbor(
    e: &Expr,
    ctx: &Interrupt,
    limit: u64,
    f: impl FnMut(&Expr, &Interrupt) -> Result<Expr, Abort>,
) -> Result<Option<Expr>, Abort> {
    // Together followed by Expand can repeatedly duplicate uncanceled denominators.
    // A heuristic neighbor must not exhaust the caller's budget after finding a winner.
    let available = ctx.steps_left.get();
    // A capped branch starts on a polling boundary, so many short branches cannot
    // skip the deadline checks when their consumed ticks are charged in a batch.
    let budget = if available > limit {
        limit / 4096 * 4096
    } else {
        available
    };
    let local = Interrupt {
        flag: ctx.flag.clone(),
        deadline_ms: ctx.deadline_ms,
        clock: ctx.clock.clone(),
        steps_left: std::cell::Cell::new(budget),
    };
    let mut f = f;
    let result = rewrite(e, &local, |e| f(e, &local));
    ctx.steps_left
        .set(available - (budget - local.steps_left.get()));
    match result {
        Err(Abort::Budget) if available > limit => Ok(None),
        result => result.map(Some),
    }
}
