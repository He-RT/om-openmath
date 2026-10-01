//! Real algorithms retain their step template data and hierarchy.
pub mod support;
use om_kernel::protocol::*;
use om_solve::{Step, StepKind as K};
use support::*;
fn compare(raw: &Step, view: &StepView, seen: &mut std::collections::BTreeSet<String>) {
    seen.insert(view.rule_id.clone());
    assert_eq!(view.id, raw.id);
    assert_eq!(view.rule_id, raw.rule_id);
    assert_eq!(
        view.level,
        match raw.level {
            om_solve::Level::Major => Level::Major,
            om_solve::Level::Minor => Level::Minor,
        }
    );
    let p = &view.params;
    let expr = |key: &str, e: &om_core::Expr| {
        assert_eq!(
            p.get(key),
            Some(&om_format::latex(e)),
            "{}: {key}",
            raw.rule_id
        )
    };
    match &raw.kind {
        K::ApplyFormula {
            bindings, results, ..
        } => {
            for (k, v) in bindings {
                expr(k, v);
            }
            assert_eq!(
                p["results"],
                om_format::latex(&om_core::Expr::call(
                    om_core::BUILTIN::LIST,
                    results.iter().cloned()
                ))
            );
        }
        K::RecordExclusion { cond, reason } => {
            expr("cond", cond);
            assert_eq!(p["reason"], format!("{reason:?}"));
        }
        K::GenericAssumption { cond } => expr("cond", cond),
        K::Discriminant { value, sign } => {
            expr("value", value);
            if let Some(sign) = sign {
                assert_eq!(p["sign"], format!("{sign:?}"));
            }
        }
        K::Verify {
            outcome, residual, ..
        } => {
            assert_eq!(p["outcome"], format!("{outcome:?}"));
            if let Some(e) = residual {
                expr("residual", e);
            }
        }
        K::RootObjects { poly, real_count } => {
            expr("poly", poly);
            assert_eq!(p["real_count"], real_count.to_string());
        }
        K::ClearDenominators { factor } | K::SplitComponent { factor } => expr("factor", factor),
        K::Substitute { new_var, def } => {
            expr("new_var", new_var);
            expr("def", def);
        }
        K::BackSubstitute { var, value } => {
            expr("var", var);
            expr("value", value);
        }
        K::RaiseToPower { n } => assert_eq!(p["n"], n.to_string()),
        K::IsolateTerm { term } => expr("term", term),
        K::Eliminant { var, poly } => {
            expr("var", var);
            expr("poly", poly);
        }
        K::Resultant { var, result } => {
            expr("var", var);
            expr("result", result);
        }
        K::DomainFilter { kept, dropped, .. } => {
            assert_eq!(p["kept"], kept.to_string());
            assert_eq!(p["dropped"], dropped.to_string());
        }
        K::Branch { label } => assert_eq!(p["label"], *label),
        K::Note { msg } => {
            assert_eq!(p["symbol"], msg.symbol);
            assert_eq!(p["tag"], msg.tag);
            assert_eq!(p["text"], msg.text);
        }
        K::SignChart { points, signs } => {
            assert!(p.contains_key("points"));
            assert_eq!(p["signs"].split(',').count(), signs.len());
            assert!(!points.is_empty());
        }
        K::RowReduce { op, .. } => {
            assert!(p["matrix"].contains('\\'));
            assert_eq!(
                p["op"],
                match op {
                    om_solve::RowOp::Swap { .. } => "Swap",
                    om_solve::RowOp::Scale { .. } => "Scale",
                    om_solve::RowOp::Add { .. } => "Add",
                }
            );
        }
        K::InvertFunction { func, .. } => {
            assert_eq!(p["func"], om_format::latex(&om_core::Expr::sym(*func)))
        }
        K::DropExtraneous { why, .. } => assert_eq!(p["why"], *why),
        K::Groebner { order, .. } => assert_eq!(p["order"], format!("{order:?}")),
        K::Factor { .. } => assert!(p.contains_key("factors")),
        K::SquareFree { .. } => assert!(p.contains_key("parts")),
        K::Normalize | K::Expand | K::ZeroProduct => assert!(p.is_empty()),
    }
    assert_eq!(view.children.len(), raw.children.len());
    for (a, b) in raw.children.iter().zip(&view.children) {
        compare(a, b, seen);
    }
}
#[test]
fn real_formula_domain_branch_linear_root_and_region_steps_preserve_template_fields() {
    let mut s = sequential();
    let mut seen = std::collections::BTreeSet::new();
    for src in [
        "Solve[x^2==9,x]",
        "Solve[Sqrt[x]==x-2,x]",
        "Solve[a*x==b,x,MaxExtraConditions->All]",
        "Solve[Sin[x]==0,x]",
        "Reduce[(x-1)/(x+2)>=0,x]",
        "Solve[{x+y==1,2*x-y==0},{x,y}]",
        "Solve[{x^2+y^2==1,x-y==0},{x,y}]",
        "Solve[x^3-x-1==0,x]",
    ] {
        let o = output(&mut s, "steps", src, Dialect::Wolfram);
        let OutputItem::Solutions {
            out_index,
            steps: Some(view),
            ..
        } = &o.items[0]
        else {
            panic!("{src}: {o:?}")
        };
        let raw = s.notebook.cells[0].steps(*out_index).unwrap();
        assert_eq!(raw.root.len(), view.root.len());
        for (a, b) in raw.root.iter().zip(&view.root) {
            compare(a, b, &mut seen);
        }
    }
    for key in [
        "quadratic_formula",
        "record_exclusion",
        "row_reduce",
        "root_objects",
        "sign_chart",
        "invert_function",
    ] {
        assert!(seen.contains(key), "{key}: {seen:?}");
    }
}
