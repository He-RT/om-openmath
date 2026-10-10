//! Restore actual persistent mathematics as data; delayed definitions never run during decoding.
use om_core::{Expr, Interrupt, Symbol};
use om_eval::{Evaluator, state::EvalStateLimits};
use om_parse::{Dialect, parse_expr};
fn run(ev: &mut Evaluator, s: &str) -> Expr {
    ev.evaluate_statement(
        &parse_expr(s, Dialect::Wolfram).unwrap(),
        &Interrupt::default(),
    )
    .unwrap()
}
fn decode(ev: &Evaluator) -> Evaluator {
    let bytes = ev
        .encode_persistent(
            "fixture-build-1",
            EvalStateLimits::default(),
            &Interrupt::default(),
        )
        .unwrap();
    Evaluator::decode_persistent(
        &bytes,
        "fixture-build-1",
        EvalStateLimits::default(),
        &Interrupt::default(),
    )
    .unwrap()
}
#[test]
fn definitions_rules_attributes_changes_history_and_random_continue_after_data_only_restore() {
    let mut original = Evaluator::new();
    run(&mut original, "a=2");
    run(&mut original, "f[x_]:=x+a");
    run(
        &mut original,
        "r=12345678901234567890123456789/100000000000000000000000000003",
    );
    run(&mut original, "SeedRandom[71]");
    run(&mut original, "RandomNormal[Count->3]");
    let before = original.history.clone();
    let mut restored = decode(&original);
    assert_eq!(restored.history, before);
    assert_eq!(
        restored.defs.take_changed_symbols(),
        original.defs.take_changed_symbols()
    );
    assert_eq!(
        restored.defs.own_value(Symbol::intern("r")),
        original.defs.own_value(Symbol::intern("r"))
    );
    assert_eq!(
        run(&mut restored, "RandomUniform[Count->7]"),
        run(&mut original, "RandomUniform[Count->7]")
    );
    assert_eq!(
        run(&mut restored, "f[{1,2}]"),
        run(&mut original, "f[{1,2}]")
    );
    run(&mut restored, "a=5");
    assert_eq!(run(&mut restored, "f[1]"), Expr::int(6));
    assert_eq!(
        original.defs.own_value(Symbol::intern("a")),
        Some(&Expr::int(2))
    );
}
#[test]
fn delayed_rhs_is_not_replayed_and_out_is_restored_without_history_reexecution() {
    let mut original = Evaluator::new();
    run(&mut original, "counter=0");
    run(
        &mut original,
        "later:=CompoundExpression[counter=counter+1,counter]",
    );
    run(&mut original, "41+1");
    let before = original.history.clone();
    let mut restored = decode(&original);
    assert_eq!(
        restored.defs.own_value(Symbol::intern("counter")),
        Some(&Expr::int(0))
    );
    assert_eq!(restored.history, before);
    assert_eq!(
        restored
            .evaluate(
                &parse_expr("Out[-1]", Dialect::Wolfram).unwrap(),
                &Interrupt::default()
            )
            .unwrap(),
        Expr::int(42)
    );
    assert_eq!(run(&mut restored, "later"), Expr::int(1));
    assert_eq!(
        original.defs.own_value(Symbol::intern("counter")),
        Some(&Expr::int(0))
    );
}
#[test]
fn state_codec_rejects_wrong_build_readonly_truncation_limits_and_cancel() {
    let original = Evaluator::new();
    let ctx = Interrupt::default();
    let limits = EvalStateLimits::default();
    let bytes = original
        .encode_persistent("fixture-build-1", limits, &ctx)
        .unwrap();
    assert!(Evaluator::decode_persistent(&bytes, "different-build", limits, &ctx).is_err());
    assert!(
        original
            .fork_readonly()
            .encode_persistent("fixture-build-1", limits, &ctx)
            .is_err()
    );
    assert!(
        Evaluator::decode_persistent(&bytes[..bytes.len() - 1], "fixture-build-1", limits, &ctx)
            .is_err()
    );
    assert!(
        Evaluator::decode_persistent(
            &bytes,
            "fixture-build-1",
            EvalStateLimits {
                max_bytes: 8,
                ..limits
            },
            &ctx
        )
        .is_err()
    );
    ctx.flag.store(true, std::sync::atomic::Ordering::Relaxed);
    assert!(Evaluator::decode_persistent(&bytes, "fixture-build-1", limits, &ctx).is_err());
}

fn rewrite_metadata(bytes: &[u8], change: impl FnOnce(&mut serde_json::Value)) -> Vec<u8> {
    let size = u32::from_le_bytes(bytes[5..9].try_into().unwrap()) as usize;
    let mut metadata: serde_json::Value = serde_json::from_slice(&bytes[13..13 + size]).unwrap();
    change(&mut metadata);
    let encoded = serde_json::to_vec(&metadata).unwrap();
    let graph = &bytes[13 + size..];
    let mut output = b"OMES\x01".to_vec();
    output.extend((encoded.len() as u32).to_le_bytes());
    output.extend((graph.len() as u32).to_le_bytes());
    output.extend(encoded);
    output.extend(graph);
    output
}
#[test]
fn registry_metadata_indices_and_duplicate_ownership_are_not_trusted_from_input() {
    let mut original = Evaluator::new();
    run(&mut original, "a=2");
    let bytes = original
        .encode_persistent(
            "fixture-build-1",
            EvalStateLimits::default(),
            &Interrupt::default(),
        )
        .unwrap();
    let bad = [
        rewrite_metadata(&bytes, |m| m["unknown_field"] = serde_json::json!(true)),
        rewrite_metadata(&bytes, |m| {
            m["registry"][0]["id"] = serde_json::json!("unregistered-callback")
        }),
        rewrite_metadata(&bytes, |m| m["metadata_version"] = serde_json::json!(0)),
        rewrite_metadata(&bytes, |m| {
            m["settings"]["unknown_field"] = serde_json::json!(3)
        }),
        rewrite_metadata(&bytes, |m| {
            m["own"][0]["value"] = serde_json::json!(u32::MAX)
        }),
        rewrite_metadata(&bytes, |m| m["changed"][0] = m["own"][0]["symbol"].clone()),
        rewrite_metadata(&bytes, |m| {
            let duplicate = m["own"][0].clone();
            m["own"].as_array_mut().unwrap().push(duplicate);
        }),
    ];
    for bytes in bad {
        assert!(
            Evaluator::decode_persistent(
                &bytes,
                "fixture-build-1",
                EvalStateLimits::default(),
                &Interrupt::default()
            )
            .is_err()
        );
    }
    assert_eq!(
        original.defs.own_value(Symbol::intern("a")),
        Some(&Expr::int(2))
    );
}
#[test]
fn actual_root_high_precision_interpolation_and_fit_values_restore_without_recomputation() {
    let mut original = Evaluator::new();
    for s in [
        "let roots=solve(x^5-x-1=0,x)",
        "let precise=decimal(\"0.12345678901234567890123456789\",precision:80)",
        "let interpolation=interpolate([[0,0],[1,2],[2,4]])",
        "let line=fit([[0,1],[1,3],[2,5]],model:a*x+b,parameters:{a:1,b:0})",
        "let curve=ode(fn(t,y)=>y,initial:1,t:0..1)",
    ] {
        let parsed = parse_expr(s, Dialect::Modern).unwrap();
        let actual = original
            .evaluate_statement(&parsed, &Interrupt::default())
            .unwrap();
        assert!(
            original
                .messages
                .take()
                .iter()
                .all(|m| !matches!(m.level, om_core::MsgLevel::Error)),
            "{s}"
        );
        assert_ne!(actual, parsed, "{s}");
    }
    let mut restored = decode(&original);
    for name in ["roots", "precise", "interpolation", "line", "curve"] {
        assert_eq!(
            restored.defs.own_value(Symbol::intern(name)),
            original.defs.own_value(Symbol::intern(name)),
            "{name}"
        );
    }
    for source in [
        "interpolation(0.37)",
        "line.model(7)",
        "curve.solution(0.37)",
        "numeric(precise,precision:80)",
    ] {
        let parsed = parse_expr(source, Dialect::Modern).unwrap();
        let first = restored.evaluate(&parsed, &Interrupt::default()).unwrap();
        let second = original.evaluate(&parsed, &Interrupt::default()).unwrap();
        assert_eq!(first, second, "{source}");
    }
}

#[test]
fn metadata_budget_history_limits_order_and_packet_lengths_reject_malformed_states() {
    let mut original = Evaluator::new();
    run(&mut original, "z=1");
    run(&mut original, "a=2");
    let limits = EvalStateLimits::default();
    let ctx = Interrupt::default();
    let bytes = original
        .encode_persistent("fixture-build-1", limits, &ctx)
        .unwrap();
    assert!(
        original
            .encode_persistent(
                "fixture-build-1",
                EvalStateLimits {
                    max_history: 1,
                    ..limits
                },
                &ctx
            )
            .is_err()
    );
    assert!(
        Evaluator::decode_persistent(
            &bytes,
            "fixture-build-1",
            EvalStateLimits {
                max_bindings: 1,
                ..limits
            },
            &ctx
        )
        .is_err()
    );
    assert!(
        Evaluator::decode_persistent(
            &bytes,
            "fixture-build-1",
            EvalStateLimits {
                max_metadata_bytes: 1,
                ..limits
            },
            &ctx
        )
        .is_err()
    );
    let wrong_order = rewrite_metadata(&bytes, |m| m["own"].as_array_mut().unwrap().reverse());
    assert!(Evaluator::decode_persistent(&wrong_order, "fixture-build-1", limits, &ctx).is_err());
    let no_history = rewrite_metadata(&bytes, |m| m["history"] = serde_json::json!([]));
    assert!(Evaluator::decode_persistent(&no_history, "fixture-build-1", limits, &ctx).is_err());
    for n in [0, 4, 12, bytes.len() - 1] {
        assert!(
            Evaluator::decode_persistent(&bytes[..n], "fixture-build-1", limits, &ctx).is_err()
        );
    }
    let mut long = bytes.clone();
    long[5..9].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(Evaluator::decode_persistent(&long, "fixture-build-1", limits, &ctx).is_err());
    let mut trailing = bytes.clone();
    trailing.push(0);
    assert!(Evaluator::decode_persistent(&trailing, "fixture-build-1", limits, &ctx).is_err());
}

#[test]
fn capture_restore_recapture_is_byte_stable_and_keeps_shared_history_values() {
    let mut original = Evaluator::new();
    run(
        &mut original,
        "value={1/3,2/7,12345678901234567890123456789}",
    );
    run(&mut original, "alias=value");
    let ctx = Interrupt::default();
    let limits = EvalStateLimits::default();
    let bytes = original
        .encode_persistent("fixture-build-1", limits, &ctx)
        .unwrap();
    let restored = Evaluator::decode_persistent(&bytes, "fixture-build-1", limits, &ctx).unwrap();
    assert_eq!(
        restored
            .encode_persistent("fixture-build-1", limits, &ctx)
            .unwrap(),
        bytes
    );
}
