//! All original authority inputs pass through real kernel compute -> bytes -> data-only restore.
#[path = "../../../tests/corpus/solve_support.rs"]
mod support;
use om_kernel::{
    KernelConfig, Session,
    checkpoint::{CheckpointBinding, CheckpointLimits, CheckpointRestore},
    protocol::*,
};
use std::sync::{Arc, atomic::AtomicBool};
#[test]
fn all_53_original_authority_results_and_evidence_survive_session_checkpoint() {
    let cases = support::corpus();
    assert_eq!(cases.len(), 53);
    for case in cases {
        let mut config = KernelConfig::default();
        config.general.eval_timeout_ms = 120000;
        let mut original = Session::new(config, None);
        let reply = original
            .handle(Request::Evaluate {
                cell_id: "authority".into(),
                source: case.input.clone(),
                dialect: Dialect::Wolfram,
            })
            .0;
        let Response::Evaluated { output, .. } = reply else {
            panic!("{} {}: {reply:?}", case.id, case.authority)
        };
        let saved = serde_json::to_value(&output).unwrap();
        let input_form = saved["items"].as_array().unwrap().last().unwrap()["input_form"]
            .as_str()
            .unwrap();
        let actual = support::e(input_form);
        if case.check == "=" {
            assert_eq!(
                actual,
                support::e(case.expected.as_ref().unwrap()),
                "{} {}",
                case.id,
                case.notes
            );
        } else {
            let input = om_parse::parse_expr(&case.input, om_parse::Dialect::Wolfram).unwrap();
            support::numeric_expr(&case, &actual, &input.args()[0]).unwrap();
        }
        let Response::NotebookState { state } = original.handle(Request::GetNotebookState).0 else {
            panic!("state missing")
        };
        let binding = CheckpointBinding {
            document_id: format!("authority-{}", case.id),
            document_generation: 1,
            source_revision: 1,
            execution_epoch: 1,
            kernel_state_revision: 1,
            source_snapshot_hash: "0".repeat(64),
            build: "authority-fixture-build".into(),
        };
        let bytes = original
            .encode_checkpoint(
                &binding,
                CheckpointLimits::default(),
                &om_core::Interrupt::default(),
            )
            .unwrap_or_else(|e| panic!("encode {}: {e}", case.id));
        let restored = Session::decode_checkpoint(
            &bytes,
            CheckpointRestore {
                binding: &binding,
                source: &state.file,
                general: &original.config.general,
                clock: None,
                cancel: Arc::new(AtomicBool::new(false)),
            },
            CheckpointLimits::default(),
            &om_core::Interrupt::default(),
        )
        .unwrap_or_else(|e| panic!("decode {}: {e}", case.id))
        .into_session();
        assert_eq!(
            serde_json::to_value(restored.notebook.cells[0].output.as_ref().unwrap()).unwrap(),
            saved,
            "{} original source/evidence",
            case.id
        );
        let again = restored
            .encode_checkpoint(
                &binding,
                CheckpointLimits::default(),
                &om_core::Interrupt::default(),
            )
            .unwrap();
        assert_eq!(bytes, again, "{} byte recapture", case.id);
    }
}
