//! Persist real solver events as references to a shared expression graph, never rendered guesses.
use om_core::{Expr, Interrupt, Symbol};
use om_solve::{
    Level, Step, StepKind, Steps,
    checkpoint::{EvidenceLimits, decode_steps, encode_steps},
};
#[test]
fn structured_step_kinds_rule_ids_and_nested_order_roundtrip() {
    let mut branch = Step::new(
        StepKind::Branch {
            label: "actual branch".into(),
        },
        vec![Expr::int(1)],
        vec![Expr::int(2)],
        Level::Major,
    );
    branch.id = "S1".into();
    let mut child = Step::new(
        StepKind::Verify {
            candidate: vec![(Expr::sym(Symbol::intern("x")), Expr::rational(1, 3))],
            outcome: om_simplify::zero::Tri::Zero,
            residual: Some(Expr::int(0)),
        },
        vec![],
        vec![],
        Level::Minor,
    );
    child.id = "S1.1".into();
    branch.children.push(child);
    let original = Steps { root: vec![branch] };
    let mut roots = Vec::new();
    let data = encode_steps(
        &original,
        &mut roots,
        EvidenceLimits::default(),
        &Interrupt::default(),
    )
    .unwrap();
    let bytes =
        om_core::checkpoint::encode_expressions(&roots, Default::default(), &Interrupt::default())
            .unwrap();
    let restored_roots =
        om_core::checkpoint::decode_expressions(&bytes, Default::default(), &Interrupt::default())
            .unwrap();
    let restored = decode_steps(
        data,
        &restored_roots,
        EvidenceLimits::default(),
        &Interrupt::default(),
    )
    .unwrap();
    assert_eq!(restored.root[0].rule_id, "branch");
    assert_eq!(restored.root[0].id, "S1");
    assert_eq!(restored.root[0].before, original.root[0].before);
    assert_eq!(restored.root[0].children[0].rule_id, "verify");
    let StepKind::Verify {
        candidate,
        outcome,
        residual,
    } = &restored.root[0].children[0].kind
    else {
        panic!("wrong kind")
    };
    assert_eq!(candidate[0].1, Expr::rational(1, 3));
    assert_eq!(*outcome, om_simplify::zero::Tri::Zero);
    assert_eq!(residual, &Some(Expr::int(0)));
}

#[test]
fn real_quadratic_solver_steps_survive_raw_evidence_roundtrip() {
    let source = om_parse::parse_expr("x^2==2", om_parse::Dialect::Wolfram).unwrap();
    let variable = Expr::symbol("x");
    let actual = om_solve::solve(
        &source,
        &[variable],
        &Default::default(),
        &Interrupt::default(),
    )
    .unwrap();
    let original = actual.steps.unwrap();
    assert!(!original.root.is_empty());
    let mut roots = Vec::new();
    let data = encode_steps(
        &original,
        &mut roots,
        EvidenceLimits::default(),
        &Interrupt::default(),
    )
    .unwrap();
    let restored = decode_steps(
        data,
        &roots,
        EvidenceLimits::default(),
        &Interrupt::default(),
    )
    .unwrap();
    let mut left = Vec::new();
    let mut right = Vec::new();
    let a = encode_steps(
        &original,
        &mut left,
        EvidenceLimits::default(),
        &Interrupt::default(),
    )
    .unwrap();
    let b = encode_steps(
        &restored,
        &mut right,
        EvidenceLimits::default(),
        &Interrupt::default(),
    )
    .unwrap();
    assert_eq!(left, right);
    assert_eq!(
        serde_json::to_value(a).unwrap(),
        serde_json::to_value(b).unwrap()
    );
}

#[test]
fn malformed_rules_forward_steps_and_missing_roots_cannot_forge_a_derivation() {
    let source = om_parse::parse_expr("x^2==2", om_parse::Dialect::Wolfram).unwrap();
    let original = om_solve::solve(
        &source,
        &[Expr::symbol("x")],
        &Default::default(),
        &Interrupt::default(),
    )
    .unwrap()
    .steps
    .unwrap();
    let mut roots = Vec::new();
    let data = encode_steps(
        &original,
        &mut roots,
        EvidenceLimits::default(),
        &Interrupt::default(),
    )
    .unwrap();
    let value = serde_json::to_value(data).unwrap();
    for change in 0..4 {
        let mut bad = value.clone();
        match change {
            0 => bad["version"] = serde_json::json!(2),
            1 => bad["nodes"][0]["rule"] = serde_json::json!("unregistered-rule"),
            2 => bad["nodes"][0]["children"] = serde_json::json!([u32::MAX]),
            _ => bad["roots"] = serde_json::json!([]),
        }
        let data = serde_json::from_value(bad).unwrap();
        assert!(
            decode_steps(
                data,
                &roots,
                EvidenceLimits::default(),
                &Interrupt::default()
            )
            .is_err()
        );
    }
}
#[test]
fn actual_exact_numeric_and_region_solution_sets_preserve_full_metadata() {
    let ctx = Interrupt::default();
    for source in ["x^2==2", "(x-1)^3==0", "x^2>1"] {
        let expr = om_parse::parse_expr(source, om_parse::Dialect::Wolfram).unwrap();
        let actual = om_solve::solve(&expr, &[Expr::symbol("x")], &Default::default(), &ctx)
            .unwrap()
            .set;
        let mut roots = Vec::new();
        let data = om_solve::checkpoint::encode_solutions(
            &actual,
            &mut roots,
            EvidenceLimits::default(),
            &ctx,
        )
        .unwrap();
        let saved = serde_json::to_value(&data).unwrap();
        let restored =
            om_solve::checkpoint::decode_solutions(data, &roots, EvidenceLimits::default(), &ctx)
                .unwrap();
        let mut again = Vec::new();
        let data = om_solve::checkpoint::encode_solutions(
            &restored,
            &mut again,
            EvidenceLimits::default(),
            &ctx,
        )
        .unwrap();
        assert_eq!(roots, again);
        assert_eq!(saved, serde_json::to_value(data).unwrap());
    }
}
