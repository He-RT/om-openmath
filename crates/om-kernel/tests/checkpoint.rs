//! Whole-session byte snapshots retain actual computation records without replaying definitions.
use om_kernel::{
    KernelConfig, Session,
    checkpoint::{CheckpointBinding, CheckpointLimits, CheckpointRestore},
    protocol::*,
};
use std::sync::{Arc, atomic::AtomicBool};
fn eval(session: &mut Session, id: &str, source: &str) -> CellOutput {
    let reply = session
        .handle(Request::Evaluate {
            cell_id: id.into(),
            source: source.into(),
            dialect: Dialect::Modern,
        })
        .0;
    let Response::Evaluated { output, .. } = reply else {
        panic!("{reply:?}")
    };
    output
}
fn file(session: &mut Session) -> NotebookFile {
    let Response::NotebookState { state } = session.handle(Request::GetNotebookState).0 else {
        panic!("missing actual state")
    };
    state.file
}
fn binding() -> CheckpointBinding {
    CheckpointBinding {
        document_id: "checkpoint-fixture".into(),
        document_generation: 1,
        source_revision: 5,
        execution_epoch: 5,
        kernel_state_revision: 2,
        source_snapshot_hash: "0".repeat(64),
        build: "fixture-exact-build".into(),
    }
}
#[test]
fn actual_definitions_owners_out_steps_and_readonly_explore_survive_session_bytes() {
    let mut original = Session::new(KernelConfig::default(), None);
    eval(&mut original, "main", "let a=99;let rate=2;seed_random(31)");
    let solved = eval(&mut original, "solve", "solve(x^2=2,x)");
    let explored = eval(
        &mut original,
        "explore",
        "explore([[a,a^2],[rate,a+rate]],controls:{a:0..4})",
    );
    let expected_file = file(&mut original);
    let expected_general = original.config.general.clone();
    let binding = binding();
    original.config.llm.enabled = true;
    original.config.llm.profiles[0].api_key = Some("synthetic-do-not-capture".into());
    let bytes = original
        .encode_checkpoint(
            &binding,
            CheckpointLimits::default(),
            &om_core::Interrupt::default(),
        )
        .unwrap();
    assert!(!String::from_utf8_lossy(&bytes).contains("synthetic-do-not-capture"));
    let mut restored = Session::decode_checkpoint(
        &bytes,
        CheckpointRestore {
            binding: &binding,
            source: &expected_file,
            general: &expected_general,
            clock: None,
            cancel: Arc::new(AtomicBool::new(false)),
        },
        CheckpointLimits::default(),
        &om_core::Interrupt::default(),
    )
    .unwrap()
    .into_session();
    assert!(!restored.config.llm.enabled);
    assert!(restored.config.llm.profiles.is_empty());
    assert_eq!(
        serde_json::to_value(restored.notebook.cells[1].output.as_ref().unwrap()).unwrap(),
        serde_json::to_value(&solved).unwrap()
    );
    let OutputItem::Solutions { out_index, .. } = &solved.items[0] else {
        panic!("no real solver output")
    };
    assert_eq!(
        restored.notebook.cells[1]
            .steps(*out_index)
            .unwrap()
            .root
            .len(),
        original.notebook.cells[1]
            .steps(*out_index)
            .unwrap()
            .root
            .len()
    );
    let OutputItem::Explore {
        out_index, view_id, ..
    } = &explored.items[0]
    else {
        panic!("no real explore output")
    };
    let query = ExploreQuery {
        cell_id: "explore".into(),
        out_index: *out_index,
        view_id: view_id.clone(),
    };
    let params = std::collections::BTreeMap::from([("a".into(), 3.)]);
    let a = restored
        .handle(Request::SampleExplore {
            query: query.clone(),
            values: params.clone(),
            revision: 1,
        })
        .0;
    let b = original
        .handle(Request::SampleExplore {
            query,
            values: params,
            revision: 1,
        })
        .0;
    assert_eq!(
        serde_json::to_value(a).unwrap(),
        serde_json::to_value(b).unwrap()
    );
    let a = eval(&mut restored, "next", "random_uniform(count:5)");
    let b = eval(&mut original, "next", "random_uniform(count:5)");
    assert_eq!(
        serde_json::to_value(a).unwrap(),
        serde_json::to_value(b).unwrap()
    );
    let output = eval(&mut restored, "probe", "a+1");
    let OutputItem::Expr { input_form, .. } = &output.items[0] else {
        panic!("not math")
    };
    assert_eq!(input_form, "100");
}
#[test]
fn wrong_source_config_identity_version_trailing_limits_and_cancel_do_not_restore() {
    let mut original = Session::new(Default::default(), None);
    eval(&mut original, "cell", "let value=2;value");
    let expected = file(&mut original);
    let config = original.config.general.clone();
    let binding = binding();
    let limits = CheckpointLimits::default();
    let ctx = om_core::Interrupt::default();
    let bytes = original.encode_checkpoint(&binding, limits, &ctx).unwrap();
    let load = |bytes: &[u8],
                b: &CheckpointBinding,
                f: &NotebookFile,
                g: &om_kernel::config::GeneralConfig,
                l: CheckpointLimits| {
        Session::decode_checkpoint(
            bytes,
            CheckpointRestore {
                binding: b,
                source: f,
                general: g,
                clock: None,
                cancel: Arc::new(AtomicBool::new(false)),
            },
            l,
            &ctx,
        )
    };
    let mut wrong = binding.clone();
    wrong.execution_epoch += 1;
    assert!(load(&bytes, &wrong, &expected, &config, limits).is_err());
    let mut source = expected.clone();
    source.cells[0].source = "different source".into();
    assert!(load(&bytes, &binding, &source, &config, limits).is_err());
    let mut config2 = config.clone();
    config2.show_steps = !config2.show_steps;
    assert!(load(&bytes, &binding, &expected, &config2, limits).is_err());
    let mut future = bytes.clone();
    future[4] = 2;
    assert!(load(&future, &binding, &expected, &config, limits).is_err());
    let mut tail = bytes.clone();
    tail.push(0);
    assert!(load(&tail, &binding, &expected, &config, limits).is_err());
    assert!(
        load(
            &bytes,
            &binding,
            &expected,
            &config,
            CheckpointLimits {
                max_bytes: 8,
                ..limits
            }
        )
        .is_err()
    );
    assert!(
        load(
            &bytes[..bytes.len() - 1],
            &binding,
            &expected,
            &config,
            limits
        )
        .is_err()
    );
    ctx.flag.store(true, std::sync::atomic::Ordering::Relaxed);
    assert!(load(&bytes, &binding, &expected, &config, limits).is_err());
}

#[test]
fn actual_precision_models_interpolation_and_ode_provenance_are_retained_without_reexecution() {
    let mut original = Session::new(Default::default(), None);
    let examples = [
        ("root", "let roots=solve(x^5-x-1=0,x);roots"),
        (
            "precise",
            "let precise=decimal(\"0.12345678901234567890123456789\",precision:80);precise",
        ),
        (
            "model",
            "let model=fit([[0,1],[1,3],[2,5]],model:a*x+b,parameters:{a:1,b:0});model",
        ),
        ("curve", "let curve=ode(fn(t,y)=>y,initial:1,t:0..1);curve"),
        (
            "interpolation",
            "let interpolation=interpolate([[0,0],[1,2],[2,4]]);interpolation",
        ),
    ];
    let mut saved = Vec::new();
    for (id, source) in examples {
        saved.push(serde_json::to_value(eval(&mut original, id, source)).unwrap());
    }
    let expected = file(&mut original);
    let config = original.config.general.clone();
    let binding = binding();
    let bytes = original
        .encode_checkpoint(
            &binding,
            CheckpointLimits::default(),
            &om_core::Interrupt::default(),
        )
        .unwrap();
    let mut restored = Session::decode_checkpoint(
        &bytes,
        CheckpointRestore {
            binding: &binding,
            source: &expected,
            general: &config,
            clock: None,
            cancel: Arc::new(AtomicBool::new(false)),
        },
        CheckpointLimits::default(),
        &om_core::Interrupt::default(),
    )
    .unwrap()
    .into_session();
    for (index, output) in saved.iter().enumerate() {
        assert_eq!(
            serde_json::to_value(restored.notebook.cells[index].output.as_ref().unwrap()).unwrap(),
            *output
        );
    }
    for (n, source) in [
        "model.model(7)",
        "curve.solution(0.37)",
        "interpolation(0.25)",
        "numeric(precise,precision:80)",
    ]
    .iter()
    .enumerate()
    {
        let first = eval(&mut restored, &format!("next-{n}"), source);
        let second = eval(&mut original, &format!("next-{n}"), source);
        assert_eq!(
            serde_json::to_value(first).unwrap(),
            serde_json::to_value(second).unwrap()
        );
    }
}

fn rewrite_metadata(bytes: &[u8], change: impl FnOnce(&mut serde_json::Value)) -> Vec<u8> {
    let metadata_len = u32::from_le_bytes(bytes[5..9].try_into().unwrap()) as usize;
    let mut metadata: serde_json::Value =
        serde_json::from_slice(&bytes[21..21 + metadata_len]).unwrap();
    change(&mut metadata);
    let encoded = serde_json::to_vec(&metadata).unwrap();
    let mut packet = bytes[..21].to_vec();
    packet[5..9].copy_from_slice(&(encoded.len() as u32).to_le_bytes());
    packet.extend(encoded);
    packet.extend_from_slice(&bytes[21 + metadata_len..]);
    packet
}
#[test]
fn malformed_owners_record_history_provenance_and_unknown_nested_fields_are_rejected() {
    let mut original = Session::new(Default::default(), None);
    eval(&mut original, "main", "let a=2");
    eval(&mut original, "result", "solve(x^2=2,x)");
    let expected = file(&mut original);
    let config = original.config.general.clone();
    let binding = binding();
    let limits = CheckpointLimits::default();
    let ctx = om_core::Interrupt::default();
    let bytes = original.encode_checkpoint(&binding, limits, &ctx).unwrap();
    let changes = [
        rewrite_metadata(&bytes, |m| m["unknown"] = serde_json::json!(true)),
        rewrite_metadata(&bytes, |m| {
            m["cells"][0]["input"]["unknown"] = serde_json::json!(3)
        }),
        rewrite_metadata(&bytes, |m| {
            m["owners"][0][1] = serde_json::json!("nonexistent-cell")
        }),
        rewrite_metadata(&bytes, |m| {
            m["cells"][1]["records"][0]["value"] = m["cells"][0]["records"][0]["value"].clone()
        }),
        rewrite_metadata(&bytes, |m| {
            m["cells"][1]["records"][0]["out_index"] = serde_json::json!(u32::MAX)
        }),
        rewrite_metadata(&bytes, |m| {
            m["cells"][1]["records"][0]["solver"]["name"] = serde_json::json!("unregistered-solver")
        }),
        rewrite_metadata(&bytes, |m| {
            m["cells"][0]["status"] = serde_json::json!("Running")
        }),
    ];
    for bad in changes {
        assert!(
            Session::decode_checkpoint(
                &bad,
                CheckpointRestore {
                    binding: &binding,
                    source: &expected,
                    general: &config,
                    clock: None,
                    cancel: Arc::new(AtomicBool::new(false))
                },
                limits,
                &ctx
            )
            .is_err()
        );
    }
    assert_eq!(original.notebook.cells[0].source, "let a=2");
}

#[test]
fn restoring_a_session_never_reexecutes_delayed_source_or_resets_out_history() {
    let mut original = Session::new(Default::default(), None);
    let reply = original
        .handle(Request::Evaluate {
            cell_id: "delayed".into(),
            source: "counter=0; later:=CompoundExpression[counter=counter+1,counter]; 42".into(),
            dialect: Dialect::Wolfram,
        })
        .0;
    let Response::Evaluated { output, .. } = reply else {
        panic!("missing actual delayed output")
    };
    assert!(output.messages.is_empty());
    let source = file(&mut original);
    let general = original.config.general.clone();
    let binding = binding();
    let bytes = original
        .encode_checkpoint(
            &binding,
            CheckpointLimits::default(),
            &om_core::Interrupt::default(),
        )
        .unwrap();
    let mut restored = Session::decode_checkpoint(
        &bytes,
        CheckpointRestore {
            binding: &binding,
            source: &source,
            general: &general,
            clock: None,
            cancel: Arc::new(AtomicBool::new(false)),
        },
        CheckpointLimits::default(),
        &om_core::Interrupt::default(),
    )
    .unwrap()
    .into_session();
    let value = eval(&mut restored, "counter-probe", "counter");
    let OutputItem::Expr { input_form, .. } = &value.items[0] else {
        panic!("not scalar")
    };
    assert_eq!(input_form, "0");
    let value = eval(&mut restored, "actual-later", "later");
    let OutputItem::Expr { input_form, .. } = &value.items[0] else {
        panic!("not scalar")
    };
    assert_eq!(input_form, "1");
    let value = eval(&mut original, "parent-counter", "counter");
    let OutputItem::Expr { input_form, .. } = &value.items[0] else {
        panic!("not scalar")
    };
    assert_eq!(input_form, "0");
}
