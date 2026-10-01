//! Fraction-free Bareiss elimination over Z and Z[parameters], then exact back substitution.
mod domain;
mod fraction;
pub use domain::ExactDomain;
pub use fraction::ExactFraction;
use om_num::ctx::{Abort, Interrupt};
/// Exact operation just performed by fraction-free elimination.
#[derive(Clone, Debug)]
pub enum BareissOp<D: ExactDomain> {
    /// Exchange two rows.
    Swap {
        /// First row.
        a: usize,
        /// Second row.
        b: usize,
    },
    /// Replace target by (pivot*target-entry*source)/previous.
    Eliminate {
        /// Target row.
        target: usize,
        /// Pivot row.
        source: usize,
        /// Current pivot coefficient.
        pivot: D,
        /// Original target coefficient in the pivot column.
        entry: D,
        /// Previous pivot, whose division is exact in every updated cell.
        previous: D,
    },
}
/// Fraction-free echelon matrix and its exact pivot/parameter certificates.
#[derive(Clone, Debug)]
pub struct BareissResult<D: ExactDomain> {
    /// Echelon matrix, retaining all trailing right-hand-side columns.
    pub matrix: Vec<Vec<D>>,
    /// Pivot (row,column) pairs in ascending order.
    pub pivots: Vec<(usize, usize)>,
    /// Row transpositions, for determinant sign and later step rendering.
    pub row_swaps: Vec<(usize, usize)>,
    /// Primitive nonconstant pivots assumed nonzero on the generic path.
    pub assumptions: Vec<D>,
}
/// Complete affine solution space over the coefficient fraction field.
#[derive(Clone, Debug)]
pub struct LinearSolution<D: ExactDomain> {
    /// A particular solution with all free variables set to zero.
    pub particular: Vec<ExactFraction<D>>,
    /// Nullspace vectors with an identity submatrix on free_columns.
    pub nullspace: Vec<Vec<ExactFraction<D>>>,
    /// Nonpivot variables, ascending; each names its corresponding nullspace vector.
    pub free_columns: Vec<usize>,
    /// Nonzero parameter conditions for this generic solution path.
    pub assumptions: Vec<D>,
}
/// Exact classification of a linear system.
#[derive(Clone, Debug)]
pub enum LinearResult<D: ExactDomain> {
    /// A zero coefficient row has a nonzero right-hand side.
    Inconsistent,
    /// Contradiction holds only under the parameter pivot conditions listed here.
    GenericInconsistent {
        /// Primitive parameter expressions assumed nonzero on this path.
        assumptions: Vec<D>,
    },
    /// All solutions are particular + sum(parameter_j * nullspace_j).
    Consistent(LinearSolution<D>),
}
/// Bareiss elimination with exact division by the previous pivot.
/// Only pivot_columns leading columns select pivots; trailing columns follow each operation.
/// Prefer provably nonzero constants before parameter pivots. None rejects malformed shape,
/// incompatible contexts, degree overflow or failed exact division; Abort propagates.
pub fn bareiss<D: ExactDomain>(
    matrix: &[Vec<D>],
    pivot_columns: usize,
    ctx: &Interrupt,
) -> Result<Option<BareissResult<D>>, Abort> {
    bareiss_observed(matrix, pivot_columns, ctx, &mut |_, _| Ok(()))
}
fn bareiss_observed<D: ExactDomain>(
    matrix: &[Vec<D>],
    pivot_columns: usize,
    ctx: &Interrupt,
    observer: &mut impl FnMut(BareissOp<D>, &[Vec<D>]) -> Result<(), Abort>,
) -> Result<Option<BareissResult<D>>, Abort> {
    ctx.tick()?;
    let columns = matrix.first().map_or(pivot_columns, |r| r.len());
    if pivot_columns > columns {
        return Ok(None);
    }
    let mut out = vec![];
    let mut context = D::zero();
    for row in matrix {
        ctx.tick()?;
        if row.len() != columns {
            return Ok(None);
        }
        let mut next = vec![];
        for value in row {
            ctx.tick()?;
            if !context.compatible(value) {
                return Ok(None);
            }
            let Some(value) = value.canonical(ctx)? else {
                return Ok(None);
            };
            context = context.add(&value.sub(&value));
            next.push(value);
        }
        out.push(next);
    }
    let mut pivots = vec![];
    let mut row_swaps = vec![];
    let mut assumptions = vec![];
    let mut previous = D::one().add(&context);
    let mut rank = 0;
    for column in 0..pivot_columns {
        ctx.tick()?;
        if rank == out.len() {
            break;
        }
        let mut candidate = None;
        for (r, row) in out.iter().enumerate().skip(rank) {
            ctx.tick()?;
            if !row[column].is_zero() {
                candidate.get_or_insert(r);
                if !row[column].parameter() {
                    candidate = Some(r);
                    break;
                }
            }
        }
        let Some(candidate) = candidate else {
            continue;
        };
        if candidate != rank {
            out.swap(candidate, rank);
            row_swaps.push((rank, candidate));
            observer(
                BareissOp::Swap {
                    a: rank,
                    b: candidate,
                },
                &out,
            )?;
        }
        let pivot = out[rank][column].clone();
        if pivot.parameter() {
            let assumption = pivot.assumption(ctx)?;
            if !assumptions.contains(&assumption) {
                assumptions.push(assumption);
            }
        }
        let pivot_row = out[rank].clone();
        for i in rank + 1..out.len() {
            let row = &mut out[i];
            ctx.tick()?;
            let entry = row[column].clone();
            for j in column + 1..columns {
                ctx.tick()?;
                let (Some(a), Some(b)) = (
                    pivot.checked_mul(&row[j], ctx)?,
                    entry.checked_mul(&pivot_row[j], ctx)?,
                ) else {
                    return Ok(None);
                };
                let Some(value) = a.sub(&b).exact_quotient(&previous, ctx)? else {
                    return Ok(None);
                };
                row[j] = value;
            }
            row[column] = D::zero().add(&context);
            observer(
                BareissOp::Eliminate {
                    target: i,
                    source: rank,
                    pivot: pivot.clone(),
                    entry,
                    previous: previous.clone(),
                },
                &out,
            )?;
        }
        previous = pivot;
        pivots.push((rank, column));
        rank += 1;
    }
    Ok(Some(BareissResult {
        matrix: out,
        pivots,
        row_swaps,
        assumptions,
    }))
}
/// Exact square determinant, including det([])=1. None rejects nonsquare/invalid input.
pub fn determinant<D: ExactDomain>(matrix: &[Vec<D>], ctx: &Interrupt) -> Result<Option<D>, Abort> {
    ctx.tick()?;
    let n = matrix.len();
    if matrix.iter().any(|r| r.len() != n) {
        return Ok(None);
    }
    if n == 0 {
        return Ok(Some(D::one()));
    }
    let Some(result) = bareiss(matrix, n, ctx)? else {
        return Ok(None);
    };
    if result.pivots.len() != n {
        return Ok(Some(D::zero()));
    }
    let d = result.matrix[n - 1][n - 1].clone();
    Ok(Some(if result.row_swaps.len() % 2 == 0 {
        d
    } else {
        d.neg()
    }))
}
/// Solve A*x=b over Q or Q(parameters), giving every free variable explicitly.
/// None rejects shape/context/degree failures; contradictory systems return Inconsistent.
pub fn linear_solve<D: ExactDomain>(
    a: &[Vec<D>],
    b: &[D],
    nvars: usize,
    ctx: &Interrupt,
) -> Result<Option<LinearResult<D>>, Abort> {
    linear_solve_observed(a, b, nvars, ctx, &mut |_, _| Ok(()))
}
/// Solve with observations of actual row swaps and completed fraction-free updates.
/// The callback borrows the current integral matrix; no snapshots are allocated by
/// this adapter. Its Abort propagates before further elimination/back substitution.
/// An observer must not change caller arithmetic state; output matches linear_solve.
pub fn linear_solve_observed<D: ExactDomain>(
    a: &[Vec<D>],
    b: &[D],
    nvars: usize,
    ctx: &Interrupt,
    observer: &mut impl FnMut(BareissOp<D>, &[Vec<D>]) -> Result<(), Abort>,
) -> Result<Option<LinearResult<D>>, Abort> {
    ctx.tick()?;
    if a.len() != b.len() {
        return Ok(None);
    }
    let mut augmented = vec![];
    for (row, b) in a.iter().zip(b) {
        ctx.tick()?;
        if row.len() != nvars {
            return Ok(None);
        }
        let mut row = row.clone();
        row.push(b.clone());
        augmented.push(row);
    }
    let Some(result) = bareiss_observed(&augmented, nvars, ctx, observer)? else {
        return Ok(None);
    };
    for row in &result.matrix {
        ctx.tick()?;
        if row[..nvars].iter().all(D::is_zero) && !row[nvars].is_zero() {
            let Some(assumptions) =
                contradiction_assumptions(&row[nvars], &result.assumptions, ctx)?
            else {
                return Ok(None);
            };
            return Ok(Some(if assumptions.is_empty() {
                LinearResult::Inconsistent
            } else {
                LinearResult::GenericInconsistent { assumptions }
            }));
        }
    }
    let mut free = vec![];
    for col in 0..nvars {
        ctx.tick()?;
        if !result.pivots.iter().any(|(_, c)| *c == col) {
            free.push(col);
        }
    }
    let Some(particular) = back_substitute(&result, nvars, None, ctx)? else {
        return Ok(None);
    };
    let mut nullspace = vec![];
    for &col in &free {
        ctx.tick()?;
        let Some(vector) = back_substitute(&result, nvars, Some(col), ctx)? else {
            return Ok(None);
        };
        nullspace.push(vector);
    }
    Ok(Some(LinearResult::Consistent(LinearSolution {
        particular,
        nullspace,
        free_columns: free,
        assumptions: result.assumptions,
    })))
}
fn contradiction_assumptions<D: ExactDomain>(
    rhs: &D,
    pivots: &[D],
    ctx: &Interrupt,
) -> Result<Option<Vec<D>>, Abort> {
    let mut conditions = pivots.to_vec();
    if !rhs.parameter() {
        return Ok(Some(conditions));
    }
    let mut remaining = rhs.assumption(ctx)?;
    // A pivot already assumed nonzero also proves every factor of it nonzero.
    // Remove these factors to avoid repeating conditions such as a and a^2.
    for pivot in pivots {
        loop {
            ctx.tick()?;
            let common = remaining.gcd(pivot, ctx)?;
            if !common.parameter() {
                break;
            }
            let Some(next) = remaining.exact_quotient(&common, ctx)? else {
                return Ok(None);
            };
            remaining = next;
        }
    }
    if remaining.parameter() {
        let condition = remaining.assumption(ctx)?;
        if !conditions.contains(&condition) {
            conditions.push(condition);
        }
    }
    Ok(Some(conditions))
}
fn back_substitute<D: ExactDomain>(
    result: &BareissResult<D>,
    nvars: usize,
    free: Option<usize>,
    ctx: &Interrupt,
) -> Result<Option<Vec<ExactFraction<D>>>, Abort> {
    ctx.tick()?;
    let Some(zero) = ExactFraction::new(D::zero(), D::one(), ctx)? else {
        return Ok(None);
    };
    let mut values = vec![zero.clone(); nvars];
    if let Some(col) = free {
        let Some(one) = ExactFraction::new(D::one(), D::one(), ctx)? else {
            return Ok(None);
        };
        values[col] = one;
    }
    for &(row, col) in result.pivots.iter().rev() {
        ctx.tick()?;
        let rhs = if free.is_some() {
            D::zero()
        } else {
            result.matrix[row][nvars].clone()
        };
        let Some(mut value) = ExactFraction::new(rhs, D::one(), ctx)? else {
            return Ok(None);
        };
        for (j, x) in values.iter().enumerate().skip(col + 1) {
            ctx.tick()?;
            let Some(c) = ExactFraction::new(result.matrix[row][j].clone(), D::one(), ctx)? else {
                return Ok(None);
            };
            let Some(term) = c.mul(x, ctx)? else {
                return Ok(None);
            };
            let Some(next) = value.sub(&term, ctx)? else {
                return Ok(None);
            };
            value = next;
        }
        let Some(pivot) = ExactFraction::new(result.matrix[row][col].clone(), D::one(), ctx)?
        else {
            return Ok(None);
        };
        let Some(value) = value.div(&pivot, ctx)? else {
            return Ok(None);
        };
        values[col] = value;
    }
    Ok(Some(values))
}
