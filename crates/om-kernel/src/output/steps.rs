//! Every computational field is rendered without serializing expression internals.
use crate::protocol::{StepView, StepsView};
use om_core::{BUILTIN as B, Expr};
use om_solve::{RowOp, Step, StepKind, Steps};
use std::collections::BTreeMap;

pub(crate) fn render(steps: &Steps) -> StepsView {
    StepsView {
        root: steps.root.iter().map(step).collect(),
    }
}
fn list(values: &[Expr]) -> String {
    om_format::latex(&Expr::call(B::LIST, values.iter().cloned()))
}
fn rules(values: &[(Expr, Expr)]) -> String {
    list(
        &values
            .iter()
            .map(|(a, b)| Expr::call(B::RULE, [a.clone(), b.clone()]))
            .collect::<Vec<_>>(),
    )
}
fn parts(values: &[(Expr, u32)]) -> String {
    list(
        &values
            .iter()
            .map(|(e, n)| Expr::call(B::LIST, [e.clone(), Expr::int(*n as i64)]))
            .collect::<Vec<_>>(),
    )
}
fn step(step: &Step) -> StepView {
    let mut params = BTreeMap::new();
    let put = |p: &mut BTreeMap<String, String>, key: &str, e: &Expr| {
        p.insert(key.into(), om_format::latex(e));
    };
    match &step.kind {
        StepKind::Normalize | StepKind::Expand | StepKind::ZeroProduct => {}
        StepKind::RecordExclusion { cond, reason } => {
            put(&mut params, "cond", cond);
            params.insert("reason".into(), format!("{reason:?}"));
        }
        StepKind::GenericAssumption { cond } => put(&mut params, "cond", cond),
        StepKind::ClearDenominators { factor } | StepKind::SplitComponent { factor } => {
            put(&mut params, "factor", factor)
        }
        StepKind::Factor { factors } => {
            params.insert("factors".into(), parts(factors));
        }
        StepKind::SquareFree { parts: p } => {
            params.insert("parts".into(), parts(p));
        }
        StepKind::Substitute { new_var, def } => {
            put(&mut params, "new_var", new_var);
            put(&mut params, "def", def);
        }
        StepKind::BackSubstitute { var, value } => {
            put(&mut params, "var", var);
            put(&mut params, "value", value);
        }
        StepKind::ApplyFormula {
            formula,
            bindings,
            results,
        } => {
            params.insert("formula".into(), format!("{formula:?}"));
            for (name, value) in bindings {
                put(&mut params, name, value);
            }
            params.insert("results".into(), list(results));
        }
        StepKind::Discriminant { value, sign } => {
            put(&mut params, "value", value);
            if let Some(sign) = sign {
                params.insert("sign".into(), format!("{sign:?}"));
            }
        }
        StepKind::IsolateTerm { term } => put(&mut params, "term", term),
        StepKind::RaiseToPower { n } => {
            params.insert("n".into(), n.to_string());
        }
        StepKind::Resultant { var, result } => {
            put(&mut params, "var", var);
            put(&mut params, "result", result);
        }
        StepKind::InvertFunction {
            func,
            branches,
            constants,
        } => {
            params.insert("func".into(), om_format::latex(&Expr::sym(*func)));
            params.insert("branches".into(), list(branches));
            params.insert("constants".into(), list(constants));
        }
        StepKind::RowReduce { op, matrix } => {
            params.insert(
                "matrix".into(),
                list(
                    &matrix
                        .iter()
                        .map(|row| Expr::call(B::LIST, row.iter().cloned()))
                        .collect::<Vec<_>>(),
                ),
            );
            match op {
                RowOp::Swap { a, b } => {
                    params.insert("op".into(), "Swap".into());
                    params.insert("a".into(), a.to_string());
                    params.insert("b".into(), b.to_string());
                }
                RowOp::Scale { row, factor } => {
                    params.insert("op".into(), "Scale".into());
                    params.insert("row".into(), row.to_string());
                    put(&mut params, "factor", factor);
                }
                RowOp::Add {
                    target,
                    source,
                    factor,
                } => {
                    params.insert("op".into(), "Add".into());
                    params.insert("target".into(), target.to_string());
                    params.insert("source".into(), source.to_string());
                    put(&mut params, "factor", factor);
                }
            }
        }
        StepKind::Groebner { order, basis } => {
            params.insert("order".into(), format!("{order:?}"));
            params.insert("basis".into(), list(basis));
        }
        StepKind::Eliminant { var, poly } => {
            put(&mut params, "var", var);
            put(&mut params, "poly", poly);
        }
        StepKind::RootObjects { poly, real_count } => {
            put(&mut params, "poly", poly);
            params.insert("real_count".into(), real_count.to_string());
        }
        StepKind::Verify {
            candidate,
            outcome,
            residual,
        } => {
            params.insert("candidate".into(), rules(candidate));
            params.insert("outcome".into(), format!("{outcome:?}"));
            if let Some(residual) = residual {
                put(&mut params, "residual", residual);
            }
        }
        StepKind::DropExtraneous { candidate, why } => {
            params.insert("candidate".into(), rules(candidate));
            params.insert("why".into(), why.clone());
        }
        StepKind::DomainFilter {
            domain,
            kept,
            dropped,
        } => {
            params.insert("domain".into(), format!("{domain:?}"));
            params.insert("kept".into(), kept.to_string());
            params.insert("dropped".into(), dropped.to_string());
        }
        StepKind::SignChart { points, signs } => {
            params.insert("points".into(), list(points));
            params.insert(
                "signs".into(),
                signs
                    .iter()
                    .map(|s| format!("{s:?}"))
                    .collect::<Vec<_>>()
                    .join(","),
            );
        }
        StepKind::Branch { label } => {
            params.insert("label".into(), label.clone());
        }
        StepKind::Note { msg } => {
            params.insert("symbol".into(), msg.symbol.clone());
            params.insert("tag".into(), msg.tag.clone());
            params.insert("text".into(), msg.text.clone());
            params.insert("level".into(), format!("{:?}", msg.level));
        }
    }
    StepView {
        id: step.id.clone(),
        rule_id: step.rule_id.into(),
        level: step.level.into(),
        title_key: format!("step.{}", step.rule_id),
        params,
        before_latex: step.before.iter().map(om_format::latex).collect(),
        after_latex: step.after.iter().map(om_format::latex).collect(),
        children: step.children.iter().map(self::step).collect(),
    }
}
