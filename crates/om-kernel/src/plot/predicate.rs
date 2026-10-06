//! Boolean region programs do not canonicalize source poles or run the evaluator per grid cell.
use super::*;
use om_eval::numeric::CompiledFn;
pub(super) enum Predicate {
    Constant(bool),
    Compare(CompiledFn, CompiledFn, &'static str),
    And(Vec<Predicate>),
    Or(Vec<Predicate>),
    Not(Box<Predicate>),
}
impl Predicate {
    pub fn compile(
        e: &Expr,
        vars: &[Symbol],
        ctx: &Interrupt,
        depth: usize,
    ) -> Result<Self, PlotError> {
        ctx.tick()?;
        if depth > 64 {
            return Err(extended::invalid("region_plot条件深度超过64"));
        }
        if e.as_symbol() == Some(B::TRUE) {
            return Ok(Self::Constant(true));
        }
        if e.as_symbol() == Some(B::FALSE) {
            return Ok(Self::Constant(false));
        }
        if matches!(e.head_symbol(), Some(B::AND | B::OR)) {
            let children = e
                .args()
                .iter()
                .map(|e| Self::compile(e, vars, ctx, depth + 1))
                .collect::<Result<Vec<_>, _>>()?;
            return Ok(if e.is_head(B::AND) {
                Self::And(children)
            } else {
                Self::Or(children)
            });
        }
        if e.is_head(B::NOT) && e.args().len() == 1 {
            return Ok(Self::Not(Box::new(Self::compile(
                &e.args()[0],
                vars,
                ctx,
                depth + 1,
            )?)));
        }
        if let Some(h) = e.head_symbol().filter(|h| {
            matches!(
                h.name(),
                "Less" | "LessEqual" | "Greater" | "GreaterEqual" | "Equal" | "Unequal"
            )
        }) {
            if e.args().len() != 2 {
                return Err(extended::invalid(
                    "region_plot比较条件需要两侧；复合条件使用and/or",
                ));
            }
            return Ok(Self::Compare(
                compile_f64_with_ctx(&e.args()[0], vars, ctx)?,
                compile_f64_with_ctx(&e.args()[1], vars, ctx)?,
                h.name(),
            ));
        }
        Err(extended::invalid(
            "region_plot首版支持实数比较/and/or/not，不把不可计算条件填成false",
        ))
    }
    pub fn evaluate(
        &self,
        p: &[f64],
        work: &mut Vec<f64>,
        ctx: &Interrupt,
    ) -> Result<Option<bool>, PlotError> {
        ctx.tick()?;
        Ok(match self {
            Self::Constant(v) => Some(*v),
            Self::Compare(a, b, op) => {
                let a = a.eval_with_ctx(p, work, ctx)?;
                let b = b.eval_with_ctx(p, work, ctx)?;
                if !a.is_finite() || !b.is_finite() {
                    None
                } else {
                    Some(match *op {
                        "Less" => a < b,
                        "LessEqual" => a <= b,
                        "Greater" => a > b,
                        "GreaterEqual" => a >= b,
                        "Equal" => a == b,
                        _ => a != b,
                    })
                }
            }
            Self::Not(v) => v.evaluate(p, work, ctx)?.map(|v| !v),
            Self::And(v) => {
                let mut valid = Some(true);
                for e in v {
                    match e.evaluate(p, work, ctx)? {
                        Some(false) => {
                            valid = Some(false);
                            break;
                        }
                        None => valid = None,
                        _ => {}
                    }
                }
                valid
            }
            Self::Or(v) => {
                let mut valid = Some(false);
                for e in v {
                    match e.evaluate(p, work, ctx)? {
                        Some(true) => {
                            valid = Some(true);
                            break;
                        }
                        None => valid = None,
                        _ => {}
                    }
                }
                valid
            }
        })
    }
}
