//! Exact LDL^T + rational stationarity/KKT are independent of the local optimizer's stopping flag.
use super::optimization_quadratic::{self as q, Quadratic, checked};
use super::*;
type Bounds = Vec<(Rational, Rational)>;
type LinearSolution = (Vec<Rational>, Vec<Vec<Rational>>);
struct Ldlt {
    l: Vec<Vec<Rational>>,
    d: Vec<Rational>,
}
fn factor(h: &[Vec<Rational>], ctx: &Interrupt) -> Result<Ldlt, EvalError> {
    let n = h.len();
    let mut l = vec![vec![Rational::ZERO; n]; n];
    let mut d = vec![Rational::ZERO; n];
    for j in 0..n {
        ctx.tick()?;
        l[j][j] = Rational::ONE;
        let mut diagonal = h[j][j].clone();
        for k in 0..j {
            diagonal = checked(diagonal - checked(&l[j][k] * &l[j][k] * &d[k], ctx)?, ctx)?;
        }
        if diagonal < Rational::ZERO {
            return Err(error("有理Hessian不是半正定，不能认证此方向的全局最优"));
        }
        d[j] = diagonal;
        for i in j + 1..n {
            let mut v = h[i][j].clone();
            for k in 0..j {
                v = checked(v - checked(&l[i][k] * &l[j][k] * &d[k], ctx)?, ctx)?;
            }
            if d[j] == Rational::ZERO {
                if v != Rational::ZERO {
                    return Err(error("零LDLT主元仍有非零余行，Hessian不是半正定"));
                }
            } else {
                l[i][j] = checked(v / &d[j], ctx)?;
            }
        }
    }
    // A returned factor is independently reconstructed, not merely accepted from its own pivots.
    for i in 0..n {
        for j in 0..n {
            let mut value = Rational::ZERO;
            for k in 0..n {
                value = checked(value + checked(&l[i][k] * &d[k] * &l[j][k], ctx)?, ctx)?;
            }
            if value != h[i][j] {
                return Err(error("精确LDLT重构不等于原Hessian"));
            }
        }
    }
    Ok(Ldlt { l, d })
}
fn back(
    l: &[Vec<Rational>],
    mut z: Vec<Rational>,
    ctx: &Interrupt,
) -> Result<Vec<Rational>, EvalError> {
    for i in (0..z.len()).rev() {
        for j in i + 1..z.len() {
            z[i] = checked(&z[i] - checked(&l[j][i] * &z[j], ctx)?, ctx)?;
        }
    }
    Ok(z)
}
fn solve(f: &Ldlt, rhs: &[Rational], ctx: &Interrupt) -> Result<Option<LinearSolution>, EvalError> {
    let n = rhs.len();
    let mut z = rhs.to_vec();
    for i in 0..n {
        for j in 0..i {
            z[i] = checked(&z[i] - checked(&f.l[i][j] * &z[j], ctx)?, ctx)?;
        }
    }
    let mut w = vec![Rational::ZERO; n];
    let mut free = vec![];
    for i in 0..n {
        ctx.tick()?;
        if f.d[i] == Rational::ZERO {
            if z[i] != Rational::ZERO {
                return Ok(None);
            }
            let mut basis = vec![Rational::ZERO; n];
            basis[i] = Rational::ONE;
            free.push(back(&f.l, basis, ctx)?);
        } else {
            w[i] = checked(&z[i] / &f.d[i], ctx)?;
        }
    }
    Ok(Some((back(&f.l, w, ctx)?, free)))
}
fn multipliers(
    x: &[Rational],
    g: &[Rational],
    bounds: Option<&Bounds>,
) -> Option<(Vec<Rational>, Vec<Rational>)> {
    let mut lower = vec![Rational::ZERO; x.len()];
    let mut upper = lower.clone();
    for i in 0..x.len() {
        if let Some(bounds) = bounds {
            if x[i] < bounds[i].0 || x[i] > bounds[i].1 {
                return None;
            }
            if x[i] == bounds[i].0 && g[i] >= Rational::ZERO {
                lower[i] = g[i].clone();
            } else if x[i] == bounds[i].1 && g[i] <= Rational::ZERO {
                upper[i] = -g[i].clone();
            } else if g[i] != Rational::ZERO {
                return None;
            }
        } else if g[i] != Rational::ZERO {
            return None;
        }
    }
    Some((lower, upper))
}
pub(super) fn apply(
    polynomial: Quadratic,
    bounds: Option<Bounds>,
    maximize: bool,
    max_faces: usize,
    ctx: &Interrupt,
) -> Result<Expr, EvalError> {
    let n = polynomial.linear.len();
    let mut signed = polynomial;
    if maximize {
        signed.negate();
    }
    let ldlt = factor(&signed.h, ctx)?;
    let mut faces = 0;
    let (point, null_space) = if let Some(bounds) = &bounds {
        let varying: Vec<_> = (0..n).filter(|i| bounds[*i].0 != bounds[*i].1).collect();
        if varying.len() > 8 {
            return Err(error("全局有理盒约束首版最多8个非固定维度"));
        }
        let mut answer = None;
        for face in 0..3usize.pow(varying.len() as u32) {
            ctx.tick()?;
            if faces >= max_faces {
                return Err(error(
                    "全局盒约束枚举超过max_iterations面数预算，未认证成功",
                ));
            }
            faces += 1;
            let mut state = vec![1; n];
            let mut code = face;
            for i in &varying {
                state[*i] = code % 3;
                code /= 3;
            }
            let free: Vec<_> = (0..n).filter(|i| state[*i] == 0).collect();
            let mut x: Vec<_> = (0..n)
                .map(|i| {
                    if state[i] == 2 {
                        bounds[i].1.clone()
                    } else {
                        bounds[i].0.clone()
                    }
                })
                .collect();
            let mut h = vec![vec![Rational::ZERO; free.len()]; free.len()];
            let mut rhs = vec![Rational::ZERO; free.len()];
            for (i, axis) in free.iter().enumerate() {
                let mut linear = signed.linear[*axis].clone();
                for j in 0..n {
                    if state[j] != 0 {
                        linear = checked(linear + checked(&signed.h[*axis][j] * &x[j], ctx)?, ctx)?;
                    }
                }
                rhs[i] = -linear;
                for (j, other) in free.iter().enumerate() {
                    h[i][j] = signed.h[*axis][*other].clone();
                }
            }
            let Some((solution, _)) = solve(&factor(&h, ctx)?, &rhs, ctx)? else {
                continue;
            };
            for (i, axis) in free.iter().enumerate() {
                x[*axis] = solution[i].clone();
            }
            if multipliers(&x, &signed.gradient(&x, ctx)?, Some(bounds)).is_some() {
                answer = Some(x);
                break;
            }
        }
        (
            answer.ok_or_else(|| error("没有找到满足精确盒KKT条件的候选"))?,
            None,
        )
    } else {
        let rhs: Vec<_> = signed.linear.iter().map(|q| -q.clone()).collect();
        let (point, free) = solve(&ldlt, &rhs, ctx)?
            .ok_or_else(|| error("半正定二次目标在线性自由方向无界，没有全局最优点"))?;
        (point, Some(free))
    };
    let gradient = signed.gradient(&point, ctx)?;
    let (lower, upper) = multipliers(&point, &gradient, bounds.as_ref())
        .ok_or_else(|| error("精确KKT可行性/梯度/乘子检查失败"))?;
    let mut residual = vec![];
    for i in 0..n {
        let r = checked(&gradient[i] - &lower[i] + &upper[i], ctx)?;
        if r != Rational::ZERO {
            return Err(error("精确KKT残差非零"));
        }
        residual.push(r);
    }
    if let Some(free) = &null_space {
        for direction in free {
            let mut g = signed.gradient(direction, ctx)?;
            for (g, l) in g.iter_mut().zip(&signed.linear) {
                *g = checked(&*g - l, ctx)?;
            }
            if g.iter().any(|g| *g != Rational::ZERO) {
                return Err(error("精确自由方向未通过Hessian零空间验证"));
            }
        }
    }
    let mut value = signed.value(&point, ctx)?;
    if maximize {
        value = -value;
    }
    Ok(record([
        ("point", q::row(point)),
        ("value", q::expr(value)),
        ("scope", Expr::string("global")),
        ("goal", Expr::string(if maximize { "max" } else { "min" })),
        ("converged", Expr::sym(B::TRUE)),
        ("guarantee", Expr::string("certified_global")),
        ("method", Expr::string("exact_ldlt")),
        ("iterations", Expr::int(0)),
        ("evaluations", Expr::int(0)),
        ("faces_examined", Expr::int(faces as i64)),
        ("projected_gradient_norm", Expr::int(0)),
        ("bracket_width", Expr::sym(B::NULL)),
        (
            "optimal_set",
            Expr::string(if bounds.is_some() {
                "one_certified_box_optimum"
            } else {
                "affine"
            }),
        ),
        (
            "null_space",
            null_space
                .map(q::matrix)
                .unwrap_or_else(|| Expr::sym(B::NULL)),
        ),
        (
            "certificate",
            record([
                ("objective_sign", Expr::int(if maximize { -1 } else { 1 })),
                ("constant", q::expr(signed.constant)),
                ("linear", q::row(signed.linear)),
                ("hessian", q::matrix(signed.h)),
                ("ldlt_l", q::matrix(ldlt.l)),
                ("ldlt_d", q::row(ldlt.d)),
                ("gradient", q::row(gradient)),
                ("lower_multipliers", q::row(lower)),
                ("upper_multipliers", q::row(upper)),
                ("kkt_residual", q::row(residual)),
                (
                    "bounds",
                    bounds
                        .map(|b| q::matrix(b.into_iter().map(|(a, b)| vec![a, b]).collect()))
                        .unwrap_or_else(|| Expr::sym(B::NULL)),
                ),
            ]),
        ),
    ]))
}
