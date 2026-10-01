//! Display approximations do not establish a solution's coordinate order.
use crate::{
    Solution, SolveError,
    univariate::order::{compare_coordinates, coordinate},
};
use om_core::{Expr, canonical_cmp};
use om_num::ctx::Interrupt;
use std::cmp::Ordering;
pub(super) fn sort(
    roots: &mut [Solution],
    vars: &[Expr],
    ctx: &Interrupt,
) -> Result<(), SolveError> {
    let mut values = roots
        .iter()
        .map(|root| {
            let zero = root
                .constants
                .iter()
                .map(|(c, _)| (c.clone(), Expr::int(0)))
                .collect::<Vec<_>>();
            vars.iter()
                .map(|v| {
                    let value = root
                        .rules
                        .iter()
                        .find(|(a, _)| a == v)
                        .map_or(v, |(_, v)| v)
                        .replace_all(&zero);
                    coordinate(&value)
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    for i in 1..roots.len() {
        let mut j = i;
        while j > 0 {
            let mut ordering = Ordering::Equal;
            for (a, b) in values[j - 1].iter().zip(&values[j]) {
                ordering = compare_coordinates(a, b, ctx)?;
                if ordering != Ordering::Equal {
                    break;
                }
            }
            if ordering == Ordering::Equal {
                for ((_, a), (_, b)) in roots[j - 1].rules.iter().zip(&roots[j].rules) {
                    ordering = canonical_cmp(a, b);
                    if ordering != Ordering::Equal {
                        break;
                    }
                }
            }
            if ordering != Ordering::Greater {
                break;
            }
            roots.swap(j - 1, j);
            values.swap(j - 1, j);
            j -= 1;
        }
    }
    Ok(())
}
