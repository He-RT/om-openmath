//! Structural affine parameter extraction. Unsupported/nonlinear forms never silently switch methods.
use super::*;
use om_core::Symbol;
pub(super) fn extract(
    e: &Expr,
    params: &[Symbol],
    ctx: &Interrupt,
) -> Result<Vec<Expr>, EvalError> {
    fn walk(
        e: &Expr,
        params: &[Symbol],
        ctx: &Interrupt,
        depth: usize,
    ) -> Result<Vec<Expr>, EvalError> {
        ctx.tick()?;
        if depth > 64 {
            return Err(error("线性模型结构深度超过64"));
        }
        let n = params.len();
        let mut out = vec![Expr::int(0); n + 1];
        if params.iter().all(|p| e.free_of(&Expr::sym(*p))) {
            out[0] = e.clone();
            return Ok(out);
        }
        if let Some(i) = params.iter().position(|p| e.as_symbol() == Some(*p)) {
            out[i + 1] = Expr::int(1);
            return Ok(out);
        }
        if e.is_head(B::PLUS) {
            for term in e.args() {
                let c = walk(term, params, ctx, depth + 1)?;
                for (a, b) in out.iter_mut().zip(c) {
                    *a = Expr::call(B::PLUS, [a.clone(), b]);
                }
            }
            return Ok(out);
        }
        if e.is_head(B::TIMES) {
            let mut dependent = None;
            let mut constants = vec![];
            for factor in e.args() {
                ctx.tick()?;
                if params.iter().any(|p| !factor.free_of(&Expr::sym(*p))) {
                    if dependent.is_some() {
                        return Err(error(
                            "method:linear要求对参数仿射；参数乘积请显式使用nonlinear",
                        ));
                    }
                    dependent = Some(factor);
                } else {
                    constants.push(factor.clone());
                }
            }
            let coefficients = walk(
                dependent.ok_or_else(|| error("仿射模型内部参数依赖错误"))?,
                params,
                ctx,
                depth + 1,
            )?;
            let constant = Expr::call(B::TIMES, constants);
            return Ok(coefficients
                .into_iter()
                .map(|c| Expr::call(B::TIMES, [constant.clone(), c]))
                .collect());
        }
        if e.is_head(B::POWER) && e.args().len() == 2 && e.args()[1] == Expr::int(1) {
            return walk(&e.args()[0], params, ctx, depth + 1);
        }
        Err(error(
            "method:linear仅支持可识别参数仿射模型，参数函数/幂/分母请显式使用nonlinear",
        ))
    }
    walk(e, params, ctx, 0)
}
