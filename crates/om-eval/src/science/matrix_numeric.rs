//! Numeric matrix adapters consume stored real samples; the pure analysis layer computes all factors.
use super::*;
use om_analysis::matrix::{Matrix, cholesky, eigensystem, lu, qr, svd};
fn failure(e: om_analysis::Error) -> EvalError {
    match e {
        om_analysis::Error::Abort(e) => e.into(),
        e => EvalError::Other(e.to_string()),
    }
}
fn samples(value: &Expr, ctx: &Interrupt) -> Result<Matrix, EvalError> {
    if !value.is_head(B::LIST) || value.args().is_empty() {
        return Err(error("机器矩阵需要非空矩形列表"));
    }
    let rows = value.args().len();
    let cols = value.args()[0].args().len();
    if rows > 64 || cols == 0 || cols > 64 {
        return Err(error("矩阵需要1..64行/列"));
    }
    let mut data = vec![];
    for row in value.args() {
        if !row.is_head(B::LIST) || row.args().len() != cols {
            return Err(error("矩阵必须为矩形"));
        }
        for entry in row.args() {
            ctx.tick()?;
            data.push(
                number(entry)?
                    .to_f64()
                    .filter(|x| x.is_finite())
                    .ok_or_else(|| error("元素需要机器范围内有限实数"))?,
            );
        }
    }
    Matrix::new(rows, cols, data).map_err(failure)
}
fn view(matrix: &Matrix) -> Result<Expr, EvalError> {
    let mut rows = vec![];
    for row in matrix.data().chunks(matrix.cols()) {
        rows.push(list(
            row.iter()
                .map(|x| real(*x))
                .collect::<Result<Vec<_>, _>>()?,
        ));
    }
    Ok(list(rows))
}
fn numbers(values: &[f64]) -> Result<Expr, EvalError> {
    Ok(list(
        values
            .iter()
            .map(|x| real(*x))
            .collect::<Result<Vec<_>, _>>()?,
    ))
}
fn residual(a: &Matrix, x: &[f64], b: &[f64], ctx: &Interrupt) -> Result<f64, EvalError> {
    let mut norm = 0.0_f64;
    for (i, rhs) in b.iter().enumerate() {
        let mut value = -rhs;
        for (j, entry) in x.iter().enumerate() {
            ctx.tick()?;
            value = a.get(i, j).mul_add(*entry, value);
        }
        norm = norm.hypot(value);
    }
    if !norm.is_finite() {
        return Err(error("残差超出机器范围"));
    }
    Ok(norm)
}
pub(super) fn wants_numeric(name: &str, args: &Args<'_>) -> Result<bool, EvalError> {
    if matches!(
        name,
        "Lu" | "Qr" | "LeastSquares" | "Svd" | "Cholesky" | "Eigenvalues" | "Eigensystem"
    ) {
        return Ok(true);
    }
    if !matches!(
        name,
        "Det" | "Inverse" | "LinearSolve" | "MatrixRank" | "NullSpace"
    ) {
        return Ok(false);
    }
    if name == "LinearSolve" {
        return match args.options.get("Mode") {
            None => Ok(false),
            Some(value) => match value.kind() {
                ExprKind::String(s) if s.as_ref() == "exact" => Ok(false),
                ExprKind::String(s) if s.as_ref() == "numeric" => Ok(true),
                _ => Err(error("mode仅支持exact或numeric字面模式")),
            },
        };
    }
    Ok(args.values[0]
        .args()
        .iter()
        .flat_map(|r| r.args())
        .any(|e| e.as_number().is_some_and(|n| !n.is_exact())))
}
pub(super) fn dispatch(name: &str, args: &Args<'_>, ctx: &Interrupt) -> Result<Expr, EvalError> {
    let a = samples(args.values[0], ctx)?;
    match name {
        "Svd" => {
            let f = svd(&a, ctx).map_err(failure)?;
            let reconstruction =
                f.u.multiply(&f.s, ctx)
                    .map_err(failure)?
                    .multiply(&f.v.transpose(ctx).map_err(failure)?, ctx)
                    .map_err(failure)?;
            let mut residual = 0.0_f64;
            for (x, y) in a.data().iter().zip(reconstruction.data()) {
                ctx.tick()?;
                residual = residual.max((x - y).abs());
            }
            Ok(record([
                ("u", view(&f.u)?),
                ("s", view(&f.s)?),
                ("v", view(&f.v)?),
                ("values", numbers(&f.values)?),
                ("residual", real(residual)?),
                ("method", Expr::string("one_sided_jacobi")),
                ("sweeps", Expr::int(f.sweeps as i64)),
            ]))
        }
        "Cholesky" => view(&cholesky(&a, ctx).map_err(failure)?),
        "Eigenvalues" | "Eigensystem" => {
            let eigen = eigensystem(&a, ctx).map_err(failure)?;
            if name == "Eigenvalues" {
                numbers(&eigen.values)
            } else {
                let mut residual = 0.0_f64;
                for j in 0..a.cols() {
                    for i in 0..a.rows() {
                        let mut av = 0.0;
                        for k in 0..a.cols() {
                            ctx.tick()?;
                            av = a.get(i, k).mul_add(eigen.vectors.get(k, j), av);
                        }
                        residual =
                            residual.max((av - eigen.values[j] * eigen.vectors.get(i, j)).abs());
                    }
                }
                Ok(record([
                    ("values", numbers(&eigen.values)?),
                    ("vectors", view(&eigen.vectors)?),
                    ("residual", real(residual)?),
                    ("method", Expr::string("symmetric_jacobi")),
                    ("rotations", Expr::int(eigen.rotations as i64)),
                ]))
            }
        }
        "Lu" => {
            let f = lu(&a, ctx).map_err(failure)?;
            let product = f.l.multiply(&f.u, ctx).map_err(failure)?;
            let mut residual = 0.0_f64;
            for i in 0..a.rows() {
                for j in 0..a.cols() {
                    ctx.tick()?;
                    residual = residual.max((a.get(f.permutation[i], j) - product.get(i, j)).abs());
                }
            }
            Ok(record([
                ("l", view(&f.l)?),
                ("u", view(&f.u)?),
                (
                    "permutation",
                    list(f.permutation.iter().map(|i| Expr::int(*i as i64 + 1))),
                ),
                ("residual", real(residual)?),
                ("method", Expr::string("partial_pivot_lu")),
            ]))
        }
        "Qr" => {
            let f = qr(&a, ctx).map_err(failure)?;
            let product = f.q.multiply(&f.r, ctx).map_err(failure)?;
            let mut residual = 0.0_f64;
            for (x, y) in a.data().iter().zip(product.data()) {
                ctx.tick()?;
                residual = residual.max((x - y).abs());
            }
            Ok(record([
                ("q", view(&f.q)?),
                ("r", view(&f.r)?),
                ("residual", real(residual)?),
                ("method", Expr::string("householder_qr")),
            ]))
        }
        "Det" => real(
            lu(&a, ctx)
                .map_err(failure)?
                .determinant(ctx)
                .map_err(failure)?,
        ),
        "Inverse" => {
            let f = lu(&a, ctx).map_err(failure)?;
            let n = a.rows();
            let mut columns = vec![];
            for j in 0..n {
                let b = (0..n).map(|i| f64::from(i == j)).collect::<Vec<_>>();
                columns.push(f.solve(&b, 1e-12, ctx).map_err(failure)?);
            }
            let data = (0..n)
                .flat_map(|i| columns.iter().map(move |c| c[i]))
                .collect();
            view(&Matrix::new(n, n, data).map_err(failure)?)
        }
        "MatrixRank" | "NullSpace" => {
            let f = svd(&a, ctx).map_err(failure)?;
            if name == "MatrixRank" {
                Ok(Expr::int(f.rank(1e-12).map_err(failure)? as i64))
            } else {
                Ok(list(
                    f.nullspace(1e-12, ctx)
                        .map_err(failure)?
                        .iter()
                        .map(|row| numbers(row))
                        .collect::<Result<Vec<_>, _>>()?,
                ))
            }
        }
        "LinearSolve" | "LeastSquares" => {
            if args.values.len() != 2 {
                return Err(error("需要矩阵和右端向量"));
            }
            let b = vector(args.values[1], ctx)?
                .iter()
                .map(|n| {
                    n.to_f64()
                        .filter(|x| x.is_finite())
                        .ok_or_else(|| error("右端需要机器范围实数"))
                })
                .collect::<Result<Vec<_>, _>>()?;
            if b.len() != a.rows() {
                return Err(error("右端维度与矩阵行数不匹配"));
            }
            let f = svd(&a, ctx).map_err(failure)?;
            let mut solved = f.solve(&a, &b, 1e-12, ctx).map_err(failure)?;
            let method = if solved.rank == a.cols() && a.rows() >= a.cols() {
                solved.particular = if name == "LinearSolve" && a.rows() == a.cols() {
                    lu(&a, ctx)
                        .map_err(failure)?
                        .solve(&b, 1e-12, ctx)
                        .map_err(failure)?
                } else {
                    qr(&a, ctx)
                        .map_err(failure)?
                        .least_squares(&b, 1e-12, ctx)
                        .map_err(failure)?
                };
                solved.residual_norm = residual(&a, &solved.particular, &b, ctx)?;
                if name == "LinearSolve" && a.rows() == a.cols() {
                    "partial_pivot_lu"
                } else {
                    "householder_qr"
                }
            } else {
                "svd_minimum_norm"
            };
            if name == "LinearSolve" && !solved.consistent {
                return Err(error("线性系统在当前机器容差下不相容"));
            }
            let solution = numbers(&solved.particular)?;
            if name == "LinearSolve" && solved.nullspace.is_empty() {
                return Ok(solution);
            }
            let basis = list(
                solved
                    .nullspace
                    .iter()
                    .map(|row| numbers(row))
                    .collect::<Result<Vec<_>, _>>()?,
            );
            let parameters = list(
                (0..solved.nullspace.len()).map(|i| Expr::call(B::C, [Expr::int(i as i64 + 1)])),
            );
            let mut affine = vec![];
            for i in 0..a.cols() {
                let mut terms = vec![real(solved.particular[i])?];
                for (j, row) in solved.nullspace.iter().enumerate() {
                    ctx.tick()?;
                    terms.push(om_core::mul([real(row[i])?, parameters.args()[j].clone()]));
                }
                affine.push(om_core::add(terms));
            }
            let affine = list(affine);
            Ok(record([
                (
                    "solution",
                    if name == "LinearSolve" {
                        affine
                    } else {
                        solution.clone()
                    },
                ),
                ("particular", solution),
                ("null_space", basis),
                ("parameters", parameters),
                ("rank", Expr::int(solved.rank as i64)),
                ("rank_kind", Expr::string("numerical")),
                ("relative_tolerance", real(1e-12)?),
                ("residual_norm", real(solved.residual_norm)?),
                (
                    "consistent",
                    Expr::sym(if solved.consistent { B::TRUE } else { B::FALSE }),
                ),
                ("method", Expr::string(method)),
                ("converged", Expr::sym(B::TRUE)),
            ]))
        }

        _ => Err(error("未匹配机器矩阵算法")),
    }
}
