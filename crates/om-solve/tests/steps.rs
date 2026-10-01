//! Recorder contracts; these synthetic events do not claim a solver implementation.
use om_core::Expr;
use om_solve::{Domain, Formula, Level, NoSteps, Step, StepKind, StepRecorder, StepSink, rule_ids};
fn step(kind: StepKind) -> Step {
    Step::new(kind, vec![Expr::int(1)], vec![Expr::int(2)], Level::Major)
}
#[test]
fn stable_hierarchical_ids_follow_emission_and_branch_order() {
    let mut sink = StepRecorder::new();
    sink.push(step(StepKind::Normalize));
    sink.enter("factor_component");
    sink.push(step(StepKind::ZeroProduct));
    sink.enter("linear_component");
    sink.push(step(StepKind::ApplyFormula {
        formula: Formula::Linear,
        bindings: vec![("a".into(), Expr::int(2))],
        results: vec![Expr::int(1)],
    }));
    sink.exit();
    sink.push(step(StepKind::Expand));
    sink.exit();
    sink.push(step(StepKind::DomainFilter {
        domain: Domain::Reals,
        kept: 1,
        dropped: 1,
    }));
    let got = sink.finish();
    assert_eq!(
        got.root.iter().map(|s| s.id.as_str()).collect::<Vec<_>>(),
        ["S1", "S2", "S3"]
    );
    let children = &got.root[1].children;
    assert_eq!(
        children.iter().map(|s| s.id.as_str()).collect::<Vec<_>>(),
        ["S2.1", "S2.2", "S2.3"]
    );
    assert_eq!(children[1].children[0].id, "S2.2.1");
    assert_eq!(children[1].children[0].rule_id, "linear_formula");
    assert_eq!(children[1].children[0].before, [Expr::int(1)]);
    assert_eq!(children[1].children[0].after, [Expr::int(2)]);
    assert!(matches!(&got.root[1].kind,StepKind::Branch{label} if label=="factor_component"));
}
#[test]
fn finish_closes_open_branches_and_extra_exit_is_harmless() {
    let mut sink = StepRecorder::default();
    sink.exit();
    sink.enter("outer");
    sink.enter("inner");
    sink.push(step(StepKind::Normalize));
    let got = sink.finish();
    assert_eq!(got.root[0].id, "S1");
    assert_eq!(got.root[0].children[0].children[0].id, "S1.1.1");
    assert!(StepRecorder::default().finish().root.is_empty());
}
#[test]
fn prebuilt_children_are_renumbered_and_kinds_own_rule_ids() {
    let mut parent = step(StepKind::Normalize);
    parent.id = "stale".into();
    parent.rule_id = "stale";
    parent.children.push(step(StepKind::Expand));
    parent.children[0]
        .children
        .push(step(StepKind::ZeroProduct));
    let mut sink = StepRecorder::default();
    sink.enter("branch");
    sink.push(parent);
    sink.exit();
    let got = sink.finish();
    let parent = &got.root[0].children[0];
    assert_eq!(parent.id, "S1.1");
    assert_eq!(parent.rule_id, "normalize");
    assert_eq!(parent.children[0].id, "S1.1.1");
    assert_eq!(parent.children[0].children[0].id, "S1.1.1.1");
}
#[test]
fn disabled_recording_does_not_construct_steps() {
    let mut built = 0;
    let mut sink = NoSteps;
    sink.enter("no allocation");
    sink.record(|| {
        built += 1;
        step(StepKind::Normalize)
    });
    sink.exit();
    assert!(!sink.enabled());
    assert_eq!(built, 0);
    let mut enabled = StepRecorder::new();
    enabled.record(|| {
        built += 1;
        step(StepKind::Normalize)
    });
    assert!(enabled.enabled());
    assert_eq!(built, 1);
    assert_eq!(enabled.finish().root.len(), 1);
}
#[test]
fn export_rule_ids() {
    let ids = rule_ids();
    assert!(ids.windows(2).all(|pair| pair[0] < pair[1]));
    for formula in [
        Formula::Linear,
        Formula::Quadratic,
        Formula::Cardano,
        Formula::Ferrari,
        Formula::Binomial,
        Formula::Palindromic,
    ] {
        let s = step(StepKind::ApplyFormula {
            formula,
            bindings: vec![],
            results: vec![],
        });
        assert!(ids.contains(&s.rule_id));
    }
    let text = ids.join("\n") + "\n";
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("rule_ids.txt");
    std::fs::write(path, text).unwrap();
}
#[test]
fn all_step_families_match_the_exported_closed_registry() {
    use om_core::{BUILTIN as B, Message, MsgLevel};
    use om_solve::{ExclReason, RowOp, Sign};
    let kinds = vec![
        StepKind::Normalize,
        StepKind::RecordExclusion {
            cond: Expr::int(1),
            reason: ExclReason::ZeroDenominator,
        },
        StepKind::GenericAssumption { cond: Expr::int(1) },
        StepKind::ClearDenominators {
            factor: Expr::int(2),
        },
        StepKind::Expand,
        StepKind::Factor { factors: vec![] },
        StepKind::ZeroProduct,
        StepKind::SquareFree { parts: vec![] },
        StepKind::Substitute {
            new_var: Expr::int(1),
            def: Expr::int(2),
        },
        StepKind::BackSubstitute {
            var: Expr::int(1),
            value: Expr::int(2),
        },
        StepKind::Discriminant {
            value: Expr::int(2),
            sign: Some(Sign::Positive),
        },
        StepKind::IsolateTerm { term: Expr::int(1) },
        StepKind::RaiseToPower { n: 2 },
        StepKind::Resultant {
            var: Expr::int(1),
            result: Expr::int(2),
        },
        StepKind::InvertFunction {
            func: B::SIN,
            branches: vec![],
            constants: vec![],
        },
        StepKind::RowReduce {
            op: RowOp::Swap { a: 0, b: 1 },
            matrix: vec![],
        },
        StepKind::Groebner {
            order: om_poly::MonoOrder::Lex,
            basis: vec![],
        },
        StepKind::Eliminant {
            var: Expr::int(1),
            poly: Expr::int(2),
        },
        StepKind::SplitComponent {
            factor: Expr::int(2),
        },
        StepKind::RootObjects {
            poly: Expr::int(2),
            real_count: 1,
        },
        StepKind::Verify {
            candidate: vec![],
            outcome: om_simplify::zero::Tri::Zero,
            residual: Some(Expr::int(0)),
        },
        StepKind::DropExtraneous {
            candidate: vec![],
            why: "original_equation".into(),
        },
        StepKind::DomainFilter {
            domain: Domain::Integers,
            kept: 0,
            dropped: 1,
        },
        StepKind::SignChart {
            points: vec![],
            signs: vec![],
        },
        StepKind::Branch {
            label: "branch".into(),
        },
        StepKind::Note {
            msg: Message {
                symbol: "Solve".into(),
                tag: "nsmet".into(),
                text: "Unsupported".into(),
                level: MsgLevel::Warning,
            },
        },
    ];
    let mut ids = kinds
        .iter()
        .map(StepKind::rule_id)
        .collect::<std::collections::BTreeSet<_>>();
    for formula in [
        Formula::Linear,
        Formula::Quadratic,
        Formula::Cardano,
        Formula::Ferrari,
        Formula::Binomial,
        Formula::Palindromic,
    ] {
        ids.insert(
            StepKind::ApplyFormula {
                formula,
                bindings: vec![],
                results: vec![],
            }
            .rule_id(),
        );
    }
    assert_eq!(ids.into_iter().collect::<Vec<_>>(), rule_ids());
}
