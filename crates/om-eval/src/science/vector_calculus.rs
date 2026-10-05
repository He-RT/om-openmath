//! Cartesian differentiation uses real derivative expressions under a readonly localized scope.
use super::*;
use om_core::{Symbol, add, sub};
use std::collections::{BTreeMap, BTreeSet};
fn vars(e: &Expr) -> Result<Vec<Symbol>, EvalError> {
    if !e.is_head(B::LIST) || e.args().is_empty() || e.args().len() > 64 {
        return Err(error("需要1..64个有序坐标变量"));
    }
    let mut seen = BTreeSet::new();
    e.args()
        .iter()
        .map(|v| {
            let s = v
                .as_symbol()
                .filter(|s| {
                    !om_core::builtins::names().contains(&s.name())
                        && om_core::catalog::by_runtime(s.name()).is_none()
                })
                .ok_or_else(|| error("坐标必须是用户符号"))?;
            if !seen.insert(s) {
                return Err(error("坐标变量必须互异"));
            }
            Ok(s)
        })
        .collect()
}
fn scalar(e: &Expr) -> Result<(), EvalError> {
    if e.is_head(B::LIST)
        || e.is_head(B::RECORD)
        || e.is_head(B::DATA_TABLE)
        || e.is_head(B::SERIES_DATA)
    {
        Err(error("此微分入口需要标量表达式"))
    } else {
        Ok(())
    }
}
fn vector(e: &Expr, n: Option<usize>) -> Result<&[Expr], EvalError> {
    if !e.is_head(B::LIST)
        || e.args().is_empty()
        || e.args().len() > 64
        || n.is_some_and(|n| n != e.args().len())
    {
        return Err(error("向量维度与坐标不匹配，最多64项"));
    }
    for v in e.args() {
        scalar(v)?;
    }
    Ok(e.args())
}
pub(super) fn dispatch(
    ev: &Evaluator,
    name: &str,
    args: &Args<'_>,
    ctx: &Interrupt,
) -> Result<Option<Expr>, EvalError> {
    if !matches!(
        name,
        "Grad" | "Jacobian" | "Hessian" | "Divergence" | "Curl" | "Laplacian"
    ) {
        return Ok(None);
    }
    let variables = vars(args.values[1])?;
    let mut fork = ev.fork_readonly();
    fork.scopes.push(
        variables
            .iter()
            .map(|s| (*s, None))
            .collect::<BTreeMap<_, _>>(),
    );
    let expression = fork.prepare_numeric(args.values[0], &[], ctx)?;
    let derivative =
        |e: &Expr, i: usize| crate::algebra::differentiate(e, &Expr::sym(variables[i]), ctx);
    let output = match name {
        "Grad" => {
            scalar(&expression)?;
            list(
                (0..variables.len())
                    .map(|i| derivative(&expression, i))
                    .collect::<Result<Vec<_>, _>>()?,
            )
        }
        "Jacobian" => {
            let v = vector(&expression, None)?;
            let mut rows = vec![];
            for e in v {
                ctx.tick()?;
                rows.push(list(
                    (0..variables.len())
                        .map(|i| derivative(e, i))
                        .collect::<Result<Vec<_>, _>>()?,
                ));
            }
            list(rows)
        }
        "Hessian" => {
            scalar(&expression)?;
            let mut rows = vec![];
            for i in 0..variables.len() {
                ctx.tick()?;
                let first = derivative(&expression, i)?;
                rows.push(list(
                    (0..variables.len())
                        .map(|j| derivative(&first, j))
                        .collect::<Result<Vec<_>, _>>()?,
                ));
            }
            list(rows)
        }
        "Laplacian" => {
            scalar(&expression)?;
            add((0..variables.len())
                .map(|i| derivative(&derivative(&expression, i)?, i))
                .collect::<Result<Vec<_>, _>>()?)
        }
        "Divergence" => {
            let v = vector(&expression, Some(variables.len()))?;
            add(v
                .iter()
                .enumerate()
                .map(|(i, e)| derivative(e, i))
                .collect::<Result<Vec<_>, _>>()?)
        }
        _ => {
            if variables.len() != 3 {
                return Err(error("curl首版只支持三维笛卡尔坐标"));
            }
            let v = vector(&expression, Some(3))?;
            list([
                sub(derivative(&v[2], 1)?, derivative(&v[1], 2)?),
                sub(derivative(&v[0], 2)?, derivative(&v[2], 0)?),
                sub(derivative(&v[1], 0)?, derivative(&v[0], 1)?),
            ])
        }
    };
    Ok(Some(output))
}
