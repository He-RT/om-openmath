//! Exact rational matrices reuse Bareiss certificates; arbitrary complex vectors retain conjugation.
use super::*;
use om_num::{Integer, Number, Rational};
use om_poly as linear;
use om_poly::LinearResult;
fn matrix(e: &Expr, ctx: &Interrupt) -> Result<Vec<Vec<Rational>>, EvalError> {
    if !e.is_head(B::LIST) || e.args().len() > 64 {
        return Err(error("矩阵最多64行"));
    }
    let mut output = vec![];
    let columns = e.args().first().map_or(0, |r| r.args().len());
    if columns > 64 {
        return Err(error("矩阵最多64列"));
    }
    for row in e.args() {
        ctx.tick()?;
        if !row.is_head(B::LIST) || row.args().len() != columns {
            return Err(error("矩阵必须为矩形"));
        }
        let mut values = vec![];
        for entry in row.args() {
            ctx.tick()?;
            let n = number(entry)?;
            if !n.is_exact() {
                return Err(error("本批矩阵消元先支持精确有理数，机器矩阵路径继续开发"));
            }
            values.push(crate::scalar::rational(&n).ok_or_else(|| error("需要有理数系数"))?);
        }
        output.push(values);
    }
    Ok(output)
}
type IntegralSystem = (Vec<Vec<Integer>>, Vec<Integer>, Integer);
fn integral(
    matrix: &[Vec<Rational>],
    rhs: Option<&[Rational]>,
    ctx: &Interrupt,
) -> Result<IntegralSystem, EvalError> {
    let mut a = vec![];
    let mut b = vec![];
    let mut scale_product = Integer::ONE;
    for (i, row) in matrix.iter().enumerate() {
        ctx.tick()?;
        let mut scale = Integer::ONE;
        for q in row.iter().chain(rhs.into_iter().map(|r| &r[i])) {
            let d = Integer::from(q.denominator().clone());
            scale = (&scale / om_num::gcd(&scale, &d)) * d;
        }
        scale_product *= &scale;
        a.push(
            row.iter()
                .map(|q| q.numerator() * (&scale / Integer::from(q.denominator().clone())))
                .collect(),
        );
        if let Some(rhs) = rhs {
            b.push(rhs[i].numerator() * (&scale / Integer::from(rhs[i].denominator().clone())));
        }
    }
    Ok((a, b, scale_product))
}
fn fraction(f: &linear::ExactFraction<Integer>) -> Expr {
    Expr::number(
        Number::Rational(Rational::from_parts(
            f.num.clone(),
            f.den.clone().into_parts().1,
        ))
        .normalize(),
    )
}
fn solve(
    a: &[Vec<Rational>],
    b: &[Rational],
    ctx: &Interrupt,
) -> Result<linear::LinearSolution<Integer>, EvalError> {
    if a.len() != b.len() {
        return Err(error("右端向量长度与矩阵行数不匹配"));
    }
    let (a, b, _) = integral(a, Some(b), ctx)?;
    let columns = a.first().map_or(0, Vec::len);
    match linear::linear_solve(&a, &b, columns, ctx)? {
        Some(LinearResult::Consistent(s)) => Ok(s),
        Some(LinearResult::Inconsistent) => Err(error("线性系统不相容")),
        _ => Err(error("无法完成精确线性消元")),
    }
}
fn dimension(e: &Expr) -> Result<usize, EvalError> {
    match e.as_number() {
        Some(Number::Integer(n)) => usize::try_from(n)
            .ok()
            .filter(|n| *n <= 64)
            .ok_or_else(|| error("尺寸需要0..64整数")),
        _ => Err(error("尺寸需要整数")),
    }
}
fn identity(n: usize) -> Expr {
    list((0..n).map(|i| list((0..n).map(|j| Expr::int(i64::from(i == j))))))
}
pub(super) fn dispatch(
    ev: &mut Evaluator,
    name: &str,
    args: &Args<'_>,
    ctx: &Interrupt,
) -> Result<Option<Expr>, EvalError> {
    let result = match name {
        "IdentityMatrix" => identity(dimension(args.values[0])?),
        "DiagonalMatrix" => {
            if !args.values[0].is_head(B::LIST) || args.values[0].args().len() > 64 {
                return Err(error("diag需要最多64项的向量"));
            }
            let entries = args.values[0].args();
            if entries.iter().any(|e| e.is_head(B::LIST)) {
                let rows = entries;
                let columns = rows.first().map_or(0, |e| e.args().len());
                if columns > 64
                    || rows
                        .iter()
                        .any(|e| !e.is_head(B::LIST) || e.args().len() != columns)
                {
                    return Err(error("diag提取需要最多64×64矩形矩阵"));
                }
                let count = rows.len().min(columns);
                let mut values = vec![];
                for (i, row) in rows.iter().enumerate().take(count) {
                    ctx.tick()?;
                    values.push(row.args()[i].clone());
                }
                return Ok(Some(list(values)));
            }
            list((0..entries.len()).map(|i| {
                list((0..entries.len()).map(|j| {
                    if i == j {
                        entries[i].clone()
                    } else {
                        Expr::int(0)
                    }
                }))
            }))
        }
        "Transpose" | "ConjugateTranspose" => {
            let a = args.values[0];
            if !a.is_head(B::LIST) || a.args().len() > 64 {
                return Err(error("需要矩阵"));
            }
            let columns = a.args().first().map_or(0, |r| r.args().len());
            if columns > 64
                || a.args()
                    .iter()
                    .any(|r| !r.is_head(B::LIST) || r.args().len() != columns)
            {
                return Err(error("需要最多64×64矩形矩阵"));
            }
            let mut rows = vec![];
            for j in 0..columns {
                let mut row = vec![];
                for source in a.args() {
                    ctx.tick()?;
                    row.push(if name == "Transpose" {
                        source.args()[j].clone()
                    } else {
                        ev.evaluate(&Expr::call(B::CONJUGATE, [source.args()[j].clone()]), ctx)?
                    });
                }
                rows.push(list(row));
            }
            list(rows)
        }
        "Tr" => {
            let a = args.values[0];
            if !a.is_head(B::LIST)
                || a.args().len() > 64
                || a.args()
                    .iter()
                    .any(|r| !r.is_head(B::LIST) || r.args().len() != a.args().len())
            {
                return Err(error("trace需要方阵"));
            }
            let mut values = vec![];
            for i in 0..a.args().len() {
                ctx.tick()?;
                values.push(a.args()[i].args()[i].clone());
            }
            om_core::add(values)
        }
        "Det" | "Inverse" | "MatrixRank" | "NullSpace" | "LinearSolve" => {
            let a = matrix(args.values[0], ctx)?;
            let rows = a.len();
            let columns = a.first().map_or(0, Vec::len);
            match name {
                "Det" => {
                    if rows != columns {
                        return Err(error("det需要方阵"));
                    }
                    let (a, _, scale) = integral(&a, None, ctx)?;
                    let det =
                        linear::determinant(&a, ctx)?.ok_or_else(|| error("无法计算行列式"))?;
                    Expr::number(
                        Number::Rational(Rational::from_parts(det, scale.into_parts().1))
                            .normalize(),
                    )
                }
                "LinearSolve" => {
                    if args.values.len() != 2 {
                        return Err(error("linear_solve需要矩阵和右端向量"));
                    }
                    let b = vector(args.values[1], ctx)?;
                    if b.iter().any(|n| !n.is_exact()) {
                        return Err(error("右端先支持精确有理数"));
                    }
                    let b = b
                        .iter()
                        .map(|n| crate::scalar::rational(n).ok_or_else(|| error("需要有理数右端")))
                        .collect::<Result<Vec<_>, _>>()?;
                    let solution = solve(&a, &b, ctx)?;
                    let particular = list(solution.particular.iter().map(fraction));
                    if solution.free_columns.is_empty() {
                        particular
                    } else {
                        let basis = list(
                            solution
                                .nullspace
                                .iter()
                                .map(|row| list(row.iter().map(fraction))),
                        );
                        let parameters = list(
                            (0..solution.free_columns.len())
                                .map(|i| Expr::call(B::C, [Expr::int(i as i64 + 1)])),
                        );
                        let affine = list((0..columns).map(|i| {
                            om_core::add(std::iter::once(particular.args()[i].clone()).chain(
                                basis.args().iter().enumerate().map(|(j, row)| {
                                    om_core::mul([
                                        row.args()[i].clone(),
                                        parameters.args()[j].clone(),
                                    ])
                                }),
                            ))
                        }));
                        record([
                            ("solution", affine),
                            ("particular", particular),
                            ("null_space", basis),
                            ("parameters", parameters),
                            (
                                "free_columns",
                                list(
                                    solution
                                        .free_columns
                                        .iter()
                                        .map(|i| Expr::int(*i as i64 + 1)),
                                ),
                            ),
                            (
                                "rank",
                                Expr::int((columns - solution.free_columns.len()) as i64),
                            ),
                            ("exact", Expr::sym(B::TRUE)),
                        ])
                    }
                }
                "Inverse" => {
                    if rows != columns {
                        return Err(error("inverse需要方阵"));
                    }
                    let mut columns_out = vec![];
                    for j in 0..columns {
                        ctx.tick()?;
                        let rhs = (0..rows)
                            .map(|i| {
                                if i == j {
                                    Rational::ONE
                                } else {
                                    Rational::ZERO
                                }
                            })
                            .collect::<Vec<_>>();
                        let solution = solve(&a, &rhs, ctx)?;
                        if !solution.free_columns.is_empty() {
                            return Err(error("奇异矩阵没有逆"));
                        }
                        columns_out
                            .push(solution.particular.iter().map(fraction).collect::<Vec<_>>());
                    }
                    list((0..rows).map(|i| list(columns_out.iter().map(|col| col[i].clone()))))
                }
                "MatrixRank" => {
                    let (a, _, _) = integral(&a, None, ctx)?;
                    let result = linear::bareiss(&a, columns, ctx)?
                        .ok_or_else(|| error("无法计算精确秩"))?;
                    Expr::int(result.pivots.len() as i64)
                }
                _ => {
                    let solution = solve(&a, &vec![Rational::ZERO; rows], ctx)?;
                    list(
                        solution
                            .nullspace
                            .iter()
                            .map(|row| list(row.iter().map(fraction))),
                    )
                }
            }
        }
        "Cross" => {
            if args.values.len() != 2 {
                return Err(error("cross需要两个三维向量"));
            }
            let a = args.values[0].args();
            let b = args.values[1].args();
            if !args.values[0].is_head(B::LIST)
                || !args.values[1].is_head(B::LIST)
                || a.len() != 3
                || b.len() != 3
            {
                return Err(error("cross需要三维向量"));
            }
            list([(1, 2), (2, 0), (0, 1)].into_iter().map(|(i, j)| {
                om_core::sub(
                    om_core::mul([a[i].clone(), b[j].clone()]),
                    om_core::mul([a[j].clone(), b[i].clone()]),
                )
            }))
        }
        "VectorAngle" | "Projection" => {
            if args.values.len() != 2 {
                return Err(error("需要两个等维向量"));
            }
            let (a, b) = (args.values[0], args.values[1]);
            if !a.is_head(B::LIST)
                || !b.is_head(B::LIST)
                || a.args().is_empty()
                || a.args().len() > 64
                || a.args().len() != b.args().len()
                || a.args()
                    .iter()
                    .chain(b.args())
                    .any(|x| x.as_number().is_none())
            {
                return Err(error("需要1..64维等长数值向量"));
            }
            let bb = ev.evaluate(&Expr::call(B::DOT, [b.clone(), b.clone()]), ctx)?;
            if bb.is_zero() {
                return Err(error("目标向量不能为零"));
            }
            if name == "Projection" {
                let inner = ev.evaluate(&Expr::call(B::DOT, [b.clone(), a.clone()]), ctx)?;
                let scale = ev.evaluate(&om_core::div(inner, bb), ctx)?;
                let mut result = vec![];
                for value in b.args() {
                    ctx.tick()?;
                    result.push(ev.evaluate(&om_core::mul([scale.clone(), value.clone()]), ctx)?);
                }
                list(result)
            } else {
                let aa = ev.evaluate(&Expr::call(B::DOT, [a.clone(), a.clone()]), ctx)?;
                if aa.is_zero() {
                    return Err(error("零向量的夹角没有定义"));
                }
                let inner = ev.evaluate(&Expr::call(B::DOT, [a.clone(), b.clone()]), ctx)?;
                let mut cosine = ev.evaluate(
                    &om_core::div(inner, om_core::sqrt(om_core::mul([aa, bb]))),
                    ctx,
                )?;
                if let Some(Number::Real(_)) = cosine.as_number()
                    && let Some(value) = cosine.as_number().and_then(Number::to_f64)
                    && value.abs() > 1.0
                {
                    if value.abs() > 1.0 + 64.0 * f64::EPSILON {
                        return Err(error("夹角余弦越界，不能伪造有效结果"));
                    }
                    cosine = real(value.clamp(-1.0, 1.0))?;
                }
                ev.evaluate(&Expr::call(B::ARCCOS, [cosine]), ctx)?
            }
        }
        "Norm" | "Normalize" => {
            let data = args.values[0];
            if !data.is_head(B::LIST)
                || data.args().is_empty()
                || data.args().len() > 64
                || data.args().iter().any(|e| e.as_number().is_none())
            {
                return Err(error("需要非空数值向量"));
            }
            let square = ev.evaluate(&Expr::call(B::DOT, [data.clone(), data.clone()]), ctx)?;
            let norm = ev.evaluate(&om_core::sqrt(square), ctx)?;
            if name == "Norm" {
                norm
            } else {
                if norm.is_zero() {
                    return Err(error("零向量不能归一化"));
                }
                let mut out = vec![];
                for e in data.args() {
                    ctx.tick()?;
                    out.push(ev.evaluate(&om_core::div(e.clone(), norm.clone()), ctx)?);
                }
                list(out)
            }
        }
        _ => return Ok(None),
    };
    Ok(Some(result))
}
