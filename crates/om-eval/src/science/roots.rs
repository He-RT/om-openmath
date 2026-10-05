//! Real cube root and principal nth root reuse exact constructors and existing guarded precision arithmetic.
use super::*;
use om_num::{Integer, Precision};
pub(super) fn dispatch(
    name: &str,
    args: &Args<'_>,
    ctx: &Interrupt,
) -> Result<Option<Expr>, EvalError> {
    let value = args.values[0];
    match name {
        "CubeRoot" => {
            let Some(n) = value.as_number() else {
                return Ok(Some(Expr::call(
                    om_core::Symbol::intern(name),
                    [value.clone()],
                )));
            };
            let q = rational(value)?;
            if n.precision() == Precision::Machine {
                return Ok(Some(real(
                    n.to_f64()
                        .filter(|x| x.is_finite())
                        .ok_or_else(|| error("需要有限实数"))?
                        .cbrt(),
                )?));
            }
            let negative = q < Rational::ZERO;
            let positive = if negative {
                Expr::number(n.neg())
            } else {
                value.clone()
            };
            let root = om_core::pow(positive, Expr::rational(1, 3));
            let root = if negative { om_core::neg(root) } else { root };
            if n.is_exact() {
                Ok(Some(root))
            } else {
                Ok(Some(Expr::number(
                    om_simplify::numeval::approximate(&root, n.precision(), ctx)?
                        .ok_or_else(|| error("当前精度的实立方根无法完成"))?,
                )))
            }
        }
        "NthRoot" => {
            let Some(Number::Integer(n)) = args.values[1].as_number() else {
                return Err(error("nth_root的degree须为正整数"));
            };
            let degree = u32::try_from(n)
                .ok()
                .filter(|n| (1..=4096).contains(n))
                .ok_or_else(|| error("nth_root的degree限1..4096"))?;
            if args
                .options
                .get("Branch")
                .is_some_and(|e| string(e).ok() != Some("principal"))
            {
                return Err(error("nth_root首版只支持principal分支"));
            }
            let power = om_core::pow(
                value.clone(),
                Expr::number(Number::Rational(Rational::from_parts(
                    Integer::ONE,
                    degree.into(),
                ))),
            );
            if let Some(n) = value.as_number()
                && !n.is_exact()
            {
                Ok(Some(Expr::number(
                    om_simplify::numeval::approximate(&power, n.precision(), ctx)?
                        .ok_or_else(|| error("当前精度的主值根无法完成"))?,
                )))
            } else {
                Ok(Some(power))
            }
        }
        _ => Ok(None),
    }
}
