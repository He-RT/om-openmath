//! Real checkpoint bytes in a bounded host pool are scoped/pinned and restore owned candidates.
use om_host_service::kernel::{KernelStateError, KernelStatePool};
use om_kernel::{
    Session,
    checkpoint::{CheckpointBinding, CheckpointLimits, CheckpointRestore},
    protocol::*,
};
use om_num::ctx::Interrupt;
use std::sync::{Arc, atomic::AtomicBool};
fn binding() -> CheckpointBinding {
    CheckpointBinding {
        document_id: "pool-doc".into(),
        document_generation: 1,
        source_revision: 1,
        execution_epoch: 1,
        kernel_state_revision: 1,
        source_snapshot_hash: "0".repeat(64),
        build: "host-pool-fixture".into(),
    }
}
fn evaluate(session: &mut Session, id: &str, source: &str) -> String {
    let Response::Evaluated { output, .. } = session
        .handle(Request::Evaluate {
            cell_id: id.into(),
            source: source.into(),
            dialect: Dialect::Modern,
        })
        .0
    else {
        panic!("no actual output")
    };
    let OutputItem::Expr { input_form, .. } = output.items.last().unwrap() else {
        panic!("not scalar")
    };
    input_form.clone()
}
#[test]
fn original_scoped_bytes_restore_actual_defs_and_old_parent_stays_independent() {
    let mut parent = Session::new(Default::default(), None);
    assert_eq!(evaluate(&mut parent, "a", "let a=2; a"), "2");
    let Response::NotebookState { state } = parent.handle(Request::GetNotebookState).0 else {
        panic!("missing source")
    };
    let config = parent.config.general.clone();
    let binding = binding();
    let mut pool = KernelStatePool::new(64 * 1024 * 1024, 2).unwrap();
    let id = pool
        .capture(
            &mut parent,
            binding.clone(),
            CheckpointLimits::default(),
            &Interrupt::default(),
        )
        .unwrap();
    let saved = pool.state(&id).unwrap();
    assert_eq!(saved.binding(), &binding);
    assert_eq!(saved.hash().len(), 64);
    assert!(saved.bytes().starts_with(b"OMKS\x01"));
    let mut working = saved
        .restore(
            CheckpointRestore {
                binding: &binding,
                source: &state.file,
                general: &config,
                clock: None,
                cancel: Arc::new(AtomicBool::new(false)),
            },
            CheckpointLimits::default(),
            &Interrupt::default(),
        )
        .unwrap();
    assert_eq!(evaluate(working.session_mut(), "a", "let a=5; a"), "5");
    assert_eq!(evaluate(&mut parent, "probe", "a"), "2");
    let mut wrong = binding.clone();
    wrong.execution_epoch += 1;
    assert!(
        saved
            .restore(
                CheckpointRestore {
                    binding: &wrong,
                    source: &state.file,
                    general: &config,
                    clock: None,
                    cancel: Arc::new(AtomicBool::new(false))
                },
                CheckpointLimits::default(),
                &Interrupt::default()
            )
            .is_err()
    );
}
#[test]
fn revoked_external_pins_keep_byte_and_count_reservations_until_reaped() {
    let mut parent = Session::new(Default::default(), None);
    evaluate(&mut parent, "cell", "2+2");
    let mut pool = KernelStatePool::new(64 * 1024 * 1024, 1).unwrap();
    let limits = CheckpointLimits::default();
    let ctx = Interrupt::default();
    let id = pool.capture(&mut parent, binding(), limits, &ctx).unwrap();
    let pin = pool.state(&id).unwrap();
    let reserved = pool.reserved_bytes();
    pool.revoke(&id).unwrap();
    assert!(pool.state(&id).is_err());
    assert_eq!(pool.reserved_bytes(), reserved);
    assert!(matches!(
        pool.capture(&mut parent, binding(), limits, &ctx),
        Err(KernelStateError::Limit)
    ));
    drop(pin);
    pool.reap();
    assert_eq!(pool.reserved_bytes(), 0);
    assert!(pool.capture(&mut parent, binding(), limits, &ctx).is_ok());
    let mut too_small = KernelStatePool::new(16, 1).unwrap();
    assert!(
        too_small
            .capture(&mut parent, binding(), limits, &ctx)
            .is_err()
    );
    assert_eq!(too_small.reserved_bytes(), 0);
}
