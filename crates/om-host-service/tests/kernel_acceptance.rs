//! Trusted lifecycle/contract tests. Synthetic receipt values here are never physical IO evidence.
use om_host_service::{
    document::{SourceDocument, snapshot_hash},
    kernel::{
        acceptance::{
            AcceptanceContext, AcceptedKernelState, FrozenKernelPlan, KernelRecovery,
            decode_commit, decode_receipt, request_hash, validate_commit, validate_receipt,
        },
        worker::{KernelJob, KernelWorker, KernelWorkerError},
    },
    protocol::{Nullable, Serial, generated::*},
};
use om_kernel::{checkpoint::CheckpointLimits, config::GeneralConfig};
use om_num::ctx::Interrupt;
use std::{
    sync::{Arc, atomic::AtomicBool},
    time::Duration,
};
fn serial(n: u64) -> Serial {
    Serial::new(n).unwrap()
}
fn context() -> AcceptanceContext {
    let source = SourceDocument::new(
        "accept-doc".into(),
        serial(1),
        NativeSourceFile {
            version: 1,
            title: "before".into(),
            cells: vec![NativeSourceCell {
                id: "a".into(),
                kind: NativeCellKind::Math,
                source: "let a=2".into(),
                dialect: Dialect::Modern,
            }],
        },
    )
    .unwrap()
    .snapshot()
    .clone();
    AcceptanceContext {
        runtime_instance_id: "runtime".into(),
        store_id: "store".into(),
        document_generation: serial(1),
        source,
        general: GeneralConfig::default(),
        config_revision: serial(0),
        build: "accept-build".into(),
    }
}
fn synthetic_receipt(p: &NativeKernelCommit) -> NativeKernelReceipt {
    NativeKernelReceipt {
        protocol_version: 1,
        store_id: p.store_id.clone(),
        document_id: p.producer.document_id.clone(),
        operation_id: p.operation_id.clone(),
        request_hash: p.request_hash.clone(),
        checkpoint_id: p.checkpoint_id.clone(),
        checkpoint_blob_hash: p.checkpoint_blob_hash.clone(),
        checkpoint_byte_length: p.checkpoint_byte_length,
        codec_version: 1,
        producer: p.producer.clone(),
        accepted_source_revision: p.acceptance_source.revision,
        accepted_snapshot_hash: p.acceptance_source.snapshot_hash.clone(),
        kernel_state_revision: p.producer.kernel_state_revision,
        result_id: p.result_id.clone(),
        outbox_event_id: p.outbox_event_id.clone(),
        committed_at: "2026-10-11T00:00:00.123Z".into(),
    }
}
fn bootstrap(worker: &KernelWorker, context: &AcceptanceContext) -> AcceptedKernelState {
    let plan = FrozenKernelPlan::bootstrap(
        context,
        "boot",
        worker,
        CheckpointLimits::default(),
        &Interrupt::default(),
    )
    .unwrap();
    plan.enter_commit(context, None).unwrap();
    plan.accept_receipt(&synthetic_receipt(plan.plan()))
        .unwrap()
}
fn produce(
    worker: &KernelWorker,
    parent: &AcceptedKernelState,
    context: &AcceptanceContext,
    id: &str,
) -> om_host_service::kernel::worker::KernelCandidate {
    worker
        .submit(KernelJob {
            runtime_instance_id: context.runtime_instance_id.clone(),
            document_generation: context.document_generation,
            operation_id: id.into(),
            parent_checkpoint_ref: parent.registry_ref().into(),
            source: context.source.clone(),
            general: context.general.clone(),
            config_revision: context.config_revision,
            cell_id: "a".into(),
            cancel: Arc::new(AtomicBool::new(false)),
        })
        .unwrap()
        .recv_timeout(Duration::from_secs(10))
        .unwrap()
        .unwrap()
}
fn close(worker: &KernelWorker) {
    worker.begin_close().unwrap().unwrap().join().unwrap();
}
#[test]
fn only_entered_original_plan_and_matching_receipt_yield_an_accepted_parent() {
    let worker = KernelWorker::new(64 * 1024 * 1024, 16, CheckpointLimits::default()).unwrap();
    let ctx = context();
    let boot = FrozenKernelPlan::bootstrap(
        &ctx,
        "boot",
        &worker,
        CheckpointLimits::default(),
        &Interrupt::default(),
    )
    .unwrap();
    let receipt = synthetic_receipt(boot.plan());
    assert!(boot.accept_receipt(&receipt).is_err());
    boot.enter_commit(&ctx, None).unwrap();
    let mut wrong = receipt.clone();
    wrong.checkpoint_blob_hash = "0".repeat(64);
    assert!(boot.accept_receipt(&wrong).is_err());
    boot.mark_unknown().unwrap();
    assert!(boot.discard(&worker).is_err());
    boot.cancel().unwrap();
    let parent = boot.accept_receipt(&receipt).unwrap();
    assert!(!boot.cancel().unwrap());
    let frozen = FrozenKernelPlan::candidate(
        produce(&worker, &parent, &ctx, "real-a"),
        &parent,
        &ctx,
        &worker,
    )
    .unwrap();
    frozen.enter_commit(&ctx, Some(&parent)).unwrap();
    assert!(worker.cancel("real-a").unwrap());
    frozen.mark_unknown().unwrap();
    let accepted = frozen
        .accept_receipt(&synthetic_receipt(frozen.plan()))
        .unwrap();
    assert_eq!(accepted.receipt().kernel_state_revision.get(), 1);
    assert!(!worker.cancel("real-a").unwrap());
    close(&worker);
}
#[test]
fn title_change_accepts_original_producer_but_math_epoch_change_discards_candidate() {
    let worker = KernelWorker::new(64 * 1024 * 1024, 16, CheckpointLimits::default()).unwrap();
    let mut ctx = context();
    let parent = bootstrap(&worker, &ctx);
    let candidate = produce(&worker, &parent, &ctx, "title");
    ctx.source.revision = serial(1);
    ctx.source.file.title = "changed after CAS".into();
    ctx.source.snapshot_hash = snapshot_hash(&ctx.source);
    let frozen = FrozenKernelPlan::candidate(candidate, &parent, &ctx, &worker).unwrap();
    assert_eq!(frozen.plan().producer.source_revision.get(), 0);
    assert_eq!(frozen.plan().acceptance_source.revision.get(), 1);
    assert_eq!(frozen.plan().source.file.title, "before");
    assert_eq!(
        frozen.plan().acceptance_source.file.title,
        "changed after CAS"
    );
    frozen.enter_commit(&ctx, Some(&parent)).unwrap();
    frozen
        .accept_receipt(&synthetic_receipt(frozen.plan()))
        .unwrap();
    let candidate = produce(&worker, &parent, &ctx, "stale");
    let reference = candidate.checkpoint_ref().to_owned();
    ctx.source.revision = serial(2);
    ctx.source.execution_epoch = serial(1);
    ctx.source.file.cells[0].source = "let a=5".into();
    ctx.source.cell_revisions[0].revision = serial(1);
    ctx.source.snapshot_hash = snapshot_hash(&ctx.source);
    assert!(FrozenKernelPlan::candidate(candidate, &parent, &ctx, &worker).is_err());
    assert!(worker.state(&reference).is_err());
    close(&worker);
}
#[test]
fn early_cancel_final_source_config_or_generation_change_prevents_commit() {
    let worker = KernelWorker::new(64 * 1024 * 1024, 16, CheckpointLimits::default()).unwrap();
    let mut ctx = context();
    let parent = bootstrap(&worker, &ctx);
    let frozen = FrozenKernelPlan::candidate(
        produce(&worker, &parent, &ctx, "cancel-before"),
        &parent,
        &ctx,
        &worker,
    )
    .unwrap();
    worker.cancel("cancel-before").unwrap();
    assert!(matches!(
        frozen.enter_commit(&ctx, Some(&parent)),
        Err(KernelWorkerError::Cancelled)
    ));
    frozen.discard(&worker).unwrap();
    let frozen = FrozenKernelPlan::candidate(
        produce(&worker, &parent, &ctx, "final-scope"),
        &parent,
        &ctx,
        &worker,
    )
    .unwrap();
    ctx.document_generation = serial(2);
    assert!(frozen.enter_commit(&ctx, Some(&parent)).is_err());
    ctx.document_generation = serial(1);
    ctx.config_revision = serial(1);
    assert!(frozen.enter_commit(&ctx, Some(&parent)).is_err());
    ctx.config_revision = serial(0);
    ctx.source.revision = serial(1);
    ctx.source.file.title = "new after freezing".into();
    ctx.source.snapshot_hash = snapshot_hash(&ctx.source);
    assert!(frozen.enter_commit(&ctx, Some(&parent)).is_err());
    frozen.discard(&worker).unwrap();
    close(&worker);
}
#[test]
fn closed_contract_hashes_roles_unknown_fields_and_receipt_producer_are_checked() {
    let worker = KernelWorker::new(64 * 1024 * 1024, 16, CheckpointLimits::default()).unwrap();
    let ctx = context();
    let parent = bootstrap(&worker, &ctx);
    let frozen = FrozenKernelPlan::candidate(
        produce(&worker, &parent, &ctx, "shape"),
        &parent,
        &ctx,
        &worker,
    )
    .unwrap();
    let plan = frozen.plan();
    decode_commit(&serde_json::to_vec(plan).unwrap()).unwrap();
    let mut malformed = plan.clone();
    malformed.expected_parent_checkpoint_id = Nullable(None);
    malformed.request_hash = request_hash(&malformed);
    assert!(validate_commit(&malformed).is_err());
    let mut malformed = serde_json::to_value(plan).unwrap();
    malformed["producer"]["unknown"] = true.into();
    assert!(decode_commit(&serde_json::to_vec(&malformed).unwrap()).is_err());
    let mut malformed = plan.clone();
    malformed.producer.cell_source_hash = Nullable(Some("0".repeat(64)));
    malformed.request_hash = request_hash(&malformed);
    assert!(validate_commit(&malformed).is_err());
    let mut receipt = synthetic_receipt(plan);
    receipt.producer.kernel_build = "other".into();
    assert!(validate_receipt(plan, &receipt).is_err());
    let mut receipt = serde_json::to_value(synthetic_receipt(plan)).unwrap();
    receipt.as_object_mut().unwrap().remove("result_id");
    assert!(decode_receipt(&serde_json::to_vec(&receipt).unwrap()).is_err());
    frozen.discard(&worker).unwrap();
    close(&worker);
}
#[test]
fn data_only_recovery_rejects_wrong_blob_and_current_build_instead_of_replaying_source() {
    let worker = KernelWorker::new(64 * 1024 * 1024, 16, CheckpointLimits::default()).unwrap();
    let ctx = context();
    let parent = bootstrap(&worker, &ctx);
    let frozen = FrozenKernelPlan::candidate(
        produce(&worker, &parent, &ctx, "restore"),
        &parent,
        &ctx,
        &worker,
    )
    .unwrap();
    let plan = frozen.plan();
    let receipt = synthetic_receipt(plan);
    let mut bytes = frozen.state().bytes().to_vec();
    bytes[0] ^= 1;
    assert!(
        AcceptedKernelState::recover(
            KernelRecovery {
                plan,
                receipt: &receipt,
                bytes,
                general: ctx.general.clone(),
                expected_build: &ctx.build
            },
            &worker,
            CheckpointLimits::default(),
            &Interrupt::default()
        )
        .is_err()
    );
    assert!(
        AcceptedKernelState::recover(
            KernelRecovery {
                plan,
                receipt: &receipt,
                bytes: frozen.state().bytes().to_vec(),
                general: ctx.general.clone(),
                expected_build: "other-build"
            },
            &worker,
            CheckpointLimits::default(),
            &Interrupt::default()
        )
        .is_err()
    );
    let recovered = AcceptedKernelState::recover(
        KernelRecovery {
            plan,
            receipt: &receipt,
            bytes: frozen.state().bytes().to_vec(),
            general: ctx.general.clone(),
            expected_build: &ctx.build,
        },
        &worker,
        CheckpointLimits::default(),
        &Interrupt::default(),
    )
    .unwrap();
    assert_eq!(recovered.receipt().kernel_state_revision.get(), 1);
    assert_eq!(recovered.state().bytes(), frozen.state().bytes());
    frozen.discard(&worker).unwrap();
    close(&worker);
}
