//! Column Hermite form and complete integer affine solutions via unimodular transforms.
use om_num::{
    Integer,
    ctx::{Abort, Interrupt},
};
/// Column Hermite lattice basis, with A*u=[h|0] and det(u)=+/-1.
#[derive(Clone, Debug, PartialEq)]
pub struct HermiteForm {
    /// Nonzero lattice basis columns, with positive increasing bottommost pivots.
    pub h: Vec<Vec<Integer>>,
    /// Square unimodular change of coordinates in the original variables.
    pub u: Vec<Vec<Integer>>,
    /// Bottommost nonzero row in each H column, ascending.
    pub pivot_rows: Vec<usize>,
}
/// All integer solutions are particular + sum(C_j*nullspace_j), C_j in Z.
#[derive(Clone, Debug)]
pub struct IntegerLinearSolution {
    /// One exact integer solution.
    pub particular: Vec<Integer>,
    /// Complete Z-basis for the integer kernel, in free transformed-column order.
    pub nullspace: Vec<Vec<Integer>>,
    /// Unimodular certificate for the original equation matrix.
    pub hermite: HermiteForm,
}
/// Exact consistency classification over the integers.
#[derive(Clone, Debug)]
pub enum IntegerLinearResult {
    /// No integer vector satisfies the original system.
    Inconsistent,
    /// A complete affine Z-solution lattice.
    Consistent(IntegerLinearSolution),
}
/// Compute column HNF with its exact unimodular coordinate certificate.
/// H has positive increasing bottommost pivots, zeros below each pivot and canonical
/// [0,pivot) entries in later columns on pivot rows. Explicit nvars supports zero-row
/// matrices. None rejects malformed shapes; all work propagates Interrupt failures.
pub fn column_hnf(
    a: &[Vec<Integer>],
    nvars: usize,
    ctx: &Interrupt,
) -> Result<Option<HermiteForm>, Abort> {
    ctx.tick()?;
    let mut matrix = vec![];
    for row in a {
        ctx.tick()?;
        if row.len() != nvars {
            return Ok(None);
        }
        matrix.push(row.clone());
    }
    let mut u = vec![];
    for i in 0..nvars {
        ctx.tick()?;
        let mut row = vec![];
        for j in 0..nvars {
            ctx.tick()?;
            row.push(if i == j { Integer::ONE } else { Integer::ZERO });
        }
        u.push(row);
    }
    let mut boundary = nvars;
    let mut pivot_rows = vec![];
    for row in (0..a.len()).rev() {
        ctx.tick()?;
        if boundary == 0 {
            break;
        }
        let k = boundary - 1;
        for j in 0..k {
            ctx.tick()?;
            if matrix[row][j].is_zero() {
                continue;
            }
            let (a, b) = (matrix[row][k].clone(), matrix[row][j].clone());
            let (g, s, t) = extended_gcd(&a, &b, ctx)?;
            let (x, y) = (-(&b / &g), &a / &g);
            transform(&mut matrix, k, j, (&s, &t, &x, &y), ctx)?;
            transform(&mut u, k, j, (&s, &t, &x, &y), ctx)?;
        }
        if matrix[row][k].is_zero() {
            continue;
        }
        if matrix[row][k] < Integer::ZERO {
            for r in &mut matrix {
                ctx.tick()?;
                r[k] = -&r[k];
            }
            for r in &mut u {
                ctx.tick()?;
                r[k] = -&r[k];
            }
        }
        let pivot = matrix[row][k].clone();
        for j in k + 1..nvars {
            ctx.tick()?;
            let value = matrix[row][j].clone();
            let mut q = &value / &pivot;
            if &value - &q * &pivot < Integer::ZERO {
                q -= 1_u8;
            }
            for r in &mut matrix {
                ctx.tick()?;
                let delta = &q * &r[k];
                r[j] -= delta;
            }
            for r in &mut u {
                ctx.tick()?;
                let delta = &q * &r[k];
                r[j] -= delta;
            }
        }
        pivot_rows.push(row);
        boundary -= 1;
    }
    pivot_rows.reverse();
    let mut h = vec![];
    for row in &matrix {
        ctx.tick()?;
        h.push(row[boundary..].to_vec());
    }
    let mut ordered_u = vec![];
    for row in u {
        ctx.tick()?;
        let mut ordered = row[boundary..].to_vec();
        ordered.extend_from_slice(&row[..boundary]);
        ordered_u.push(ordered);
    }
    Ok(Some(HermiteForm {
        h,
        u: ordered_u,
        pivot_rows,
    }))
}
/// Solve an integer linear system with a full integer kernel basis and lattice certificate.
/// None rejects shape errors; failed pivot divisibility or any unsatisfied row yields
/// Inconsistent. A zero-row system returns all nvars variables freely over Z.
pub fn integer_linear_solve(
    a: &[Vec<Integer>],
    b: &[Integer],
    nvars: usize,
    ctx: &Interrupt,
) -> Result<Option<IntegerLinearResult>, Abort> {
    ctx.tick()?;
    if a.len() != b.len() {
        return Ok(None);
    }
    let Some(hermite) = column_hnf(a, nvars, ctx)? else {
        return Ok(None);
    };
    let rank = hermite.pivot_rows.len();
    let mut z = vec![Integer::ZERO; rank];
    for j in (0..rank).rev() {
        ctx.tick()?;
        let row = hermite.pivot_rows[j];
        let mut rhs = b[row].clone();
        for (k, value) in z.iter().enumerate().skip(j + 1) {
            ctx.tick()?;
            rhs -= &hermite.h[row][k] * value;
        }
        let pivot = &hermite.h[row][j];
        let q = &rhs / pivot;
        if &q * pivot != rhs {
            return Ok(Some(IntegerLinearResult::Inconsistent));
        }
        z[j] = q;
    }
    for (row, rhs) in hermite.h.iter().zip(b) {
        ctx.tick()?;
        let mut total = Integer::ZERO;
        for (a, z) in row.iter().zip(&z) {
            ctx.tick()?;
            total += a * z;
        }
        if &total != rhs {
            return Ok(Some(IntegerLinearResult::Inconsistent));
        }
    }
    let mut particular = vec![];
    for row in &hermite.u {
        ctx.tick()?;
        let mut value = Integer::ZERO;
        for (c, z) in row.iter().zip(&z) {
            ctx.tick()?;
            value += c * z;
        }
        particular.push(value);
    }
    let mut nullspace = vec![];
    for col in rank..nvars {
        ctx.tick()?;
        let mut vector = vec![];
        for row in &hermite.u {
            ctx.tick()?;
            vector.push(row[col].clone());
        }
        nullspace.push(vector);
    }
    Ok(Some(IntegerLinearResult::Consistent(
        IntegerLinearSolution {
            particular,
            nullspace,
            hermite,
        },
    )))
}
fn transform(
    matrix: &mut [Vec<Integer>],
    a: usize,
    b: usize,
    c: (&Integer, &Integer, &Integer, &Integer),
    ctx: &Interrupt,
) -> Result<(), Abort> {
    for row in matrix {
        ctx.tick()?;
        let (x, y) = (row[a].clone(), row[b].clone());
        row[a] = c.0 * &x + c.1 * &y;
        row[b] = c.2 * x + c.3 * y;
    }
    Ok(())
}
fn extended_gcd(
    a: &Integer,
    b: &Integer,
    ctx: &Interrupt,
) -> Result<(Integer, Integer, Integer), Abort> {
    ctx.tick()?;
    let (mut old_r, mut r) = (a.clone().max(-a), b.clone().max(-b));
    let (mut old_s, mut s) = (Integer::ONE, Integer::ZERO);
    let (mut old_t, mut t) = (Integer::ZERO, Integer::ONE);
    while !r.is_zero() {
        ctx.tick()?;
        let q = &old_r / &r;
        (old_r, r) = (r.clone(), old_r - &q * &r);
        (old_s, s) = (s.clone(), old_s - &q * &s);
        (old_t, t) = (t.clone(), old_t - &q * &t);
    }
    if a < &Integer::ZERO {
        old_s = -old_s;
    }
    if b < &Integer::ZERO {
        old_t = -old_t;
    }
    Ok((old_r, old_s, old_t))
}
