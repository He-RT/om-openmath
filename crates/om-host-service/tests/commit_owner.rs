//! Actual source gate linearizes original-ID admission/fence/cancel without running CAS or IO.
use om_host_service::{
    document::{
        SourceDocument,
        commit::{CommitError, DocumentCommitController},
        coordinator::SourceCoordinator,
    },
    protocol::{Nullable, Serial, generated::*},
    references::ReferenceScope,
};
use sha2::{Digest, Sha256};
fn setup() -> (DocumentCommitController, ReferenceScope, String) {
    let owner = SourceDocument::new(
        "doc".into(),
        Serial::new(1).unwrap(),
        NativeSourceFile {
            version: 1,
            title: "".into(),
            cells: vec![NativeSourceCell {
                id: "cell".into(),
                kind: NativeCellKind::Math,
                source: "2+2".into(),
                dialect: Dialect::Modern,
            }],
        },
    )
    .unwrap();
    let scope = ReferenceScope {
        runtime: "runtime".into(),
        document: "doc".into(),
        generation: 1,
        revision: 0,
        execution_epoch: 0,
        snapshot_hash: owner.snapshot().snapshot_hash.clone(),
        task: None,
        task_generation: 0,
        grant_revision: 1,
        config_revision: 0,
        definition_revision: 0,
        metadata_revision: 26,
        editor_state_hash: String::new(),
        execution_mode: true,
        can_read: true,
        can_preview: true,
        can_write: true,
    };
    let mut controller = DocumentCommitController::new(owner, "runtime".into(), [8; 32]);
    let reference = controller
        .snapshot(
            SourceCoordinator::default(),
            ["cell".to_owned()].into_iter().collect(),
            &scope,
            0,
        )
        .unwrap();
    let args=serde_json::to_vec(&serde_json::json!({"snapshot_ref":reference,"input":{"kind":"patch","operations":[{"type":"update_cell","target":{"cell_id":"cell"},"expected_source_hash":format!("{:x}",Sha256::digest("2+2".as_bytes())),"source":"3+3"}]}})).unwrap();
    let preview = controller.preview(&args, &scope, 1, "time".into()).unwrap();
    (controller, scope, preview.preview_ref.0.unwrap())
}
fn admit(plan: &NativeSourceCommit) -> NativeSourceAdmission {
    NativeSourceAdmission {
        protocol_version: 1,
        document_id: "doc".into(),
        operation_id: plan.commit.operation_id.clone(),
        request_hash: plan.commit.request_hash.clone(),
        phase: NativeSourceAdmissionPhase::Accepted,
        created_by_runtime: "runtime".into(),
        receipt: Nullable(None),
    }
}
fn fence(plan: &NativeSourceCommit) -> NativeEditorFence {
    NativeEditorFence {
        protocol_version: 1,
        fence_id: "fence".into(),
        runtime_instance_id: "runtime".into(),
        document_id: "doc".into(),
        document_generation: Serial::new(1).unwrap(),
        source_revision: plan.before.revision,
        source_snapshot_hash: plan.before.snapshot_hash.clone(),
        issued_ms: Serial::new(10).unwrap(),
        expires_ms: Serial::new(100).unwrap(),
        targets: vec![NativeEditorTarget {
            cell_id: "cell".into(),
            editor_generation: Serial::new(1).unwrap(),
            draft_sequence: Serial::new(0).unwrap(),
            base_cell_revision: Serial::new(0).unwrap(),
            source_hash: format!("{:x}", Sha256::digest("2+2".as_bytes())),
            is_composing: false,
            is_dirty: false,
        }],
    }
}
#[test]
fn stop_before_barrier_never_enters_commit_and_does_not_change_confirmed_source() {
    let (mut c, scope, reference) = setup();
    let plan = c.begin(&reference, &scope, 2).unwrap().unwrap();
    let id = &plan.commit.commit.operation_id;
    c.admitted(id, &admit(&plan.commit)).unwrap();
    c.fenced(id, fence(&plan.commit), 11).unwrap();
    assert_eq!(
        c.cancel(id).unwrap().phase,
        NativeCommitStatePhase::Cancelled
    );
    assert_eq!(
        c.enter_commit(id, "fence", &scope, 12).unwrap_err(),
        CommitError::Cancelled
    );
    assert_eq!(c.owner().snapshot().revision.get(), 0);
}
#[test]
fn actual_kernel_gate_blocks_source_writes_but_keeps_frozen_preview_and_reads_available() {
    let (mut c, scope, reference) = setup();
    c.hold_kernel_gate("kernel-one").unwrap();
    assert_eq!(c.kernel_gate(), Some("kernel-one"));
    assert!(matches!(
        c.begin(&reference, &scope, 2),
        Err(CommitError::Busy)
    ));
    let update = NativeSourceOperation::UpdateSourceCell(UpdateSourceCell {
        kind: UpdateSourceCellKind::UpdateCell,
        cell: NativeSourceCell {
            id: "cell".into(),
            kind: NativeCellKind::Math,
            source: "let a=5".into(),
            dialect: Dialect::Modern,
        },
    });
    assert!(matches!(
        c.manual(
            &[update],
            &scope,
            &SourceCoordinator::default(),
            None,
            "time".into()
        ),
        Err(CommitError::Busy)
    ));
    assert_eq!(c.owner().snapshot().revision.get(), 0);
    assert!(
        c.snapshot(
            SourceCoordinator::default(),
            ["cell".to_owned()].into_iter().collect(),
            &scope,
            3
        )
        .is_ok()
    );
    assert!(c.release_kernel_gate("different").is_err());
    assert!(c.hold_kernel_gate("kernel-two").is_err());
    c.release_kernel_gate("kernel-one").unwrap();
    assert!(c.begin(&reference, &scope, 4).is_ok());
    assert!(c.hold_kernel_gate("kernel-two").is_err());
}
#[test]
fn stop_after_barrier_is_pending_not_undo_and_unknown_holds_the_original_gate() {
    let (mut c, scope, reference) = setup();
    let plan = c.begin(&reference, &scope, 2).unwrap().unwrap();
    let id = &plan.commit.commit.operation_id;
    c.admitted(id, &admit(&plan.commit)).unwrap();
    c.fenced(id, fence(&plan.commit), 11).unwrap();
    c.enter_commit(id, "fence", &scope, 12).unwrap();
    let cancelled = c.cancel(id).unwrap();
    assert_eq!(cancelled.phase, NativeCommitStatePhase::Committing);
    assert!(cancelled.cancel_requested);
    assert_eq!(
        c.unknown(id).unwrap().phase,
        NativeCommitStatePhase::Unknown
    );
    assert_eq!(c.owner().snapshot().revision.get(), 0);
    let same = c.begin(&reference, &scope, 10000).unwrap().unwrap();
    assert_eq!(same.commit.commit.operation_id, *id);
}
#[test]
fn late_fence_dirty_ime_scope_and_unacknowledged_admission_are_rejected() {
    let (mut c, scope, reference) = setup();
    let plan = c.begin(&reference, &scope, 2).unwrap().unwrap();
    let id = &plan.commit.commit.operation_id;
    assert!(c.fenced(id, fence(&plan.commit), 11).is_err());
    c.admitted(id, &admit(&plan.commit)).unwrap();
    let mut ime = fence(&plan.commit);
    ime.targets[0].is_composing = true;
    assert!(c.fenced(id, ime, 11).is_err());
    assert!(c.fenced(id, fence(&plan.commit), 100).is_err());
    c.fenced(id, fence(&plan.commit), 11).unwrap();
    let mut changed = scope;
    changed.grant_revision += 1;
    assert!(c.enter_commit(id, "fence", &changed, 12).is_err());
    assert_eq!(c.owner().snapshot().revision.get(), 0);
}

#[test]
fn repeated_admission_does_not_rewind_writing_committing_or_unknown() {
    let (mut c, scope, reference) = setup();
    let plan = c.begin(&reference, &scope, 2).unwrap().unwrap();
    let id = &plan.commit.commit.operation_id;
    c.admitted(id, &admit(&plan.commit)).unwrap();
    c.fenced(id, fence(&plan.commit), 11).unwrap();
    assert_eq!(
        c.admitted(id, &admit(&plan.commit)).unwrap().phase,
        NativeCommitStatePhase::Writing
    );
    c.enter_commit(id, "fence", &scope, 12).unwrap();
    assert_eq!(
        c.admitted(id, &admit(&plan.commit)).unwrap().phase,
        NativeCommitStatePhase::Committing
    );
    c.unknown(id).unwrap();
    assert_eq!(
        c.admitted(id, &admit(&plan.commit)).unwrap().phase,
        NativeCommitStatePhase::Unknown
    );
}

#[test]
fn durable_terminal_admission_never_becomes_a_new_write() {
    let (mut c, scope, reference) = setup();
    let plan = c.begin(&reference, &scope, 2).unwrap().unwrap();
    let id = &plan.commit.commit.operation_id;
    let mut old = admit(&plan.commit);
    old.phase = NativeSourceAdmissionPhase::Failed;
    assert_eq!(
        c.admitted(id, &old).unwrap().phase,
        NativeCommitStatePhase::Failed
    );
    assert!(c.begin(&reference, &scope, 3).unwrap().is_none());
    assert_eq!(c.preview_operation(&reference).unwrap().operation_id, *id);
    assert_eq!(
        c.admitted(id, &admit(&plan.commit)).unwrap().phase,
        NativeCommitStatePhase::Failed
    );
}

#[test]
fn cancelled_admission_readback_does_not_release_another_active_gate() {
    let (mut c, scope, reference) = setup();
    let plan = c.begin(&reference, &scope, 2).unwrap().unwrap();
    let id = &plan.commit.commit.operation_id;
    c.cancel(id).unwrap();
    c.settle_no_commit(id, true).unwrap();
    let rename =
        serde_json::from_value(serde_json::json!({"kind":"rename_notebook","title":"new"}))
            .unwrap();
    c.manual(
        &[rename],
        &scope,
        &SourceCoordinator::default(),
        None,
        "time".into(),
    )
    .unwrap();
    let mut old = admit(&plan.commit);
    old.phase = NativeSourceAdmissionPhase::Cancelled;
    assert_eq!(
        c.admitted(id, &old).unwrap().phase,
        NativeCommitStatePhase::Cancelled
    );
    assert!(matches!(
        c.manual(
            &[],
            &scope,
            &SourceCoordinator::default(),
            None,
            "time".into()
        ),
        Err(CommitError::Busy)
    ));
}
