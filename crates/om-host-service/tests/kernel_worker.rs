//! Real main CAS source reconciliation and independent candidates; no fake durable acceptance.
use om_host_service::{
    document::{SourceDocument, coordinator::kernel_file, snapshot_hash},
    kernel::{
        KernelState,
        worker::{KernelCandidate, KernelJob, KernelWorker, KernelWorkerError},
    },
    protocol::{Serial, generated::*},
};
use om_kernel::{
    KernelConfig, Session,
    checkpoint::{CheckpointBinding, CheckpointLimits, CheckpointRestore},
    protocol::{CellStatus, OutputItem, Request, Response},
};
use om_num::ctx::{Clock, Interrupt};
use std::{
    sync::{
        Arc, Condvar, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};
fn serial(n: u64) -> Serial {
    Serial::new(n).unwrap()
}
fn snapshot(cells: &[(&str, &str)]) -> NativeSourceSnapshot {
    SourceDocument::new(
        "worker-doc".into(),
        serial(1),
        NativeSourceFile {
            version: 1,
            title: "实际候选🙂".into(),
            cells: cells
                .iter()
                .map(|(id, source)| NativeSourceCell {
                    id: (*id).into(),
                    kind: NativeCellKind::Math,
                    source: (*source).into(),
                    dialect: Dialect::Modern,
                })
                .collect(),
        },
    )
    .unwrap()
    .snapshot()
    .clone()
}
fn binding(s: &NativeSourceSnapshot) -> CheckpointBinding {
    CheckpointBinding {
        document_id: s.document_id.clone(),
        document_generation: 1,
        source_revision: s.revision.get(),
        execution_epoch: s.execution_epoch.get(),
        kernel_state_revision: 0,
        source_snapshot_hash: s.snapshot_hash.clone(),
        build: "main-worker-real-fixture".into(),
    }
}
fn evaluate(s: &mut Session, id: &str, source: &str) -> String {
    let Response::Evaluated { output, .. } = s
        .handle(Request::Evaluate {
            cell_id: id.into(),
            source: source.into(),
            dialect: om_kernel::protocol::Dialect::Modern,
        })
        .0
    else {
        panic!("no output")
    };
    let OutputItem::Expr { input_form, .. } = output.items.last().unwrap() else {
        panic!("not expr: {output:?}")
    };
    input_form.clone()
}
fn state(source: &NativeSourceSnapshot, evaluate_ids: &[&str]) -> KernelState {
    let mut config = KernelConfig::default();
    config.general.auto_run_dependents = false;
    let mut session = Session::new(config, None);
    session
        .apply_source_file_without_evaluation(kernel_file(&source.file))
        .unwrap();
    for id in evaluate_ids {
        let cell = source.file.cells.iter().find(|c| c.id == *id).unwrap();
        evaluate(&mut session, id, &cell.source);
    }
    KernelState::capture(
        &mut session,
        binding(source),
        CheckpointLimits::default(),
        &Interrupt::default(),
    )
    .unwrap()
}
fn job(
    worker: &KernelWorker,
    parent: &str,
    source: NativeSourceSnapshot,
    operation: &str,
    cell: &str,
) -> KernelJob {
    KernelJob {
        runtime_instance_id: "runtime-new".into(),
        document_generation: serial(2),
        operation_id: operation.into(),
        parent_checkpoint_ref: parent.into(),
        general: worker.state(parent).unwrap().general().clone(),
        config_revision: serial(0),
        source,
        cell_id: cell.into(),
        cancel: Arc::new(AtomicBool::new(false)),
    }
}
fn receive(worker: &KernelWorker, job: KernelJob) -> Result<KernelCandidate, KernelWorkerError> {
    worker
        .submit(job)
        .unwrap()
        .recv_timeout(Duration::from_secs(10))
        .unwrap()
}
fn probe(state: &KernelState, source: &str) -> String {
    let mut work = state
        .restore(
            CheckpointRestore {
                binding: state.binding(),
                source: state.source(),
                general: state.general(),
                clock: None,
                cancel: Arc::new(AtomicBool::new(false)),
            },
            CheckpointLimits::default(),
            &Interrupt::default(),
        )
        .unwrap();
    evaluate(work.session_mut(), "independent-probe", source)
}
fn change(
    mut source: NativeSourceSnapshot,
    id: &str,
    text: &str,
    revision: u64,
    epoch: u64,
) -> NativeSourceSnapshot {
    source
        .file
        .cells
        .iter_mut()
        .find(|c| c.id == id)
        .unwrap()
        .source = text.into();
    source.revision = serial(revision);
    source.execution_epoch = serial(epoch);
    source
        .cell_revisions
        .iter_mut()
        .find(|c| c.cell_id == id)
        .unwrap()
        .revision = serial(revision);
    source.snapshot_hash = snapshot_hash(&source);
    source
}
fn close(worker: &KernelWorker) {
    if let Some(join) = worker.begin_close().unwrap() {
        join.join().unwrap();
    }
}
#[test]
fn explicit_parent_a2_then_a5_reconcile_retires_old_definitions_and_no_cascade_runs() {
    let source = snapshot(&[("a", "let a=2"), ("b", "let b=a+1")]);
    let worker = KernelWorker::new(64 * 1024 * 1024, 12, CheckpointLimits::default()).unwrap();
    let parent = worker.register(state(&source, &["a", "b"])).unwrap();
    let old = worker.state(&parent).unwrap();
    let old_bytes = old.bytes().to_vec();
    assert_eq!(probe(&old, "b"), "3");
    let source = change(source, "a", "let a=5", 1, 1);
    assert!(
        matches!(receive(&worker,job(&worker,&parent,source.clone(),"blocked-b","b")),Err(KernelWorkerError::Kernel(code)) if code=="DEPENDENCY_NOT_READY")
    );
    let mut run_a = job(&worker, &parent, source.clone(), "run-a5", "a");
    // The real kernel's single-cell boundary must remain one boundary even with auto-run enabled.
    run_a.general.auto_run_dependents = true;
    let candidate = receive(&worker, run_a).unwrap();
    assert_eq!(candidate.boundary().status, CellStatus::Done);
    assert_eq!(probe(candidate.state(), "a"), "5");
    assert_eq!(probe(candidate.state(), "b"), "b");
    assert_eq!(candidate.state().source().title, "实际候选🙂");
    assert_eq!(candidate.state().binding().document_generation, 2);
    assert_eq!(candidate.state().binding().kernel_state_revision, 1);
    // In this worker-only fixture the caller explicitly selects the candidate; no acceptance claim.
    let mut run_b = job(&worker, candidate.checkpoint_ref(), source, "run-b6", "b");
    run_b.config_revision = serial(1);
    let b = receive(&worker, run_b).unwrap();
    assert_eq!(probe(b.state(), "b"), "6");
    assert_eq!(old.bytes(), old_bytes);
    assert_eq!(probe(&old, "a"), "2");
    assert_eq!(probe(&old, "b"), "3");
    worker.discard(candidate.checkpoint_ref()).unwrap();
    worker.discard(b.checkpoint_ref()).unwrap();
    close(&worker);
}
#[test]
fn unchanged_math_after_title_change_keeps_definitions_and_skipped_epoch_retires_them() {
    let source = snapshot(&[("a", "let a=2"), ("b", "a+1")]);
    let worker = KernelWorker::new(64 * 1024 * 1024, 12, CheckpointLimits::default()).unwrap();
    let parent = worker.register(state(&source, &["a"])).unwrap();
    let mut renamed = source.clone();
    renamed.revision = serial(1);
    renamed.file.title = "只改标题".into();
    renamed.snapshot_hash = snapshot_hash(&renamed);
    let candidate = receive(&worker, job(&worker, &parent, renamed, "title-b", "b")).unwrap();
    assert_eq!(probe(candidate.state(), "a"), "2");
    assert_eq!(candidate.state().source().title, "只改标题");
    let reverted = change(source, "a", "let a=2", 2, 2);
    assert!(
        matches!(receive(&worker,job(&worker,&parent,reverted,"skipped-epoch","b")),Err(KernelWorkerError::Kernel(code)) if code=="DEPENDENCY_NOT_READY")
    );
    close(&worker);
}
#[test]
fn real_error_keeps_actual_partial_definition_and_history_only_in_the_candidate() {
    let mut source = snapshot(&[("effect", "partial=7; loop:=loop; loop; never=8")]);
    source.file.cells[0].dialect = Dialect::Wolfram;
    source.snapshot_hash = snapshot_hash(&source);
    let worker = KernelWorker::new(64 * 1024 * 1024, 12, CheckpointLimits::default()).unwrap();
    let parent = worker.register(state(&source, &[])).unwrap();
    let candidate = receive(
        &worker,
        job(&worker, &parent, source, "partial-error", "effect"),
    )
    .unwrap();
    assert_eq!(candidate.boundary().status, CellStatus::Error);
    // Wolfram semicolons are one CompoundExpression statement. Failed evaluation adds no history,
    // while the original evaluator retains the earlier actual assignment effects.
    assert_eq!(candidate.boundary().successful_statements, 0);
    assert_eq!(candidate.boundary().out_index, None);
    assert_eq!(probe(candidate.state(), "partial"), "7");
    assert_eq!(probe(&worker.state(&parent).unwrap(), "partial"), "partial");
    assert_eq!(probe(candidate.state(), "never"), "never");
    close(&worker);
}
struct GateClock {
    calls: AtomicUsize,
    at: usize,
    gate: Mutex<(bool, bool)>,
    signal: Condvar,
}
impl GateClock {
    fn new(at: usize) -> Self {
        Self {
            calls: AtomicUsize::new(0),
            at,
            gate: Mutex::new((false, false)),
            signal: Condvar::new(),
        }
    }
    fn entered(&self) {
        let lock = self.gate.lock().unwrap();
        let (lock, timeout) = self
            .signal
            .wait_timeout_while(lock, Duration::from_secs(10), |s| !s.0)
            .unwrap();
        assert!(lock.0 && !timeout.timed_out());
    }
    fn release(&self) {
        self.gate.lock().unwrap().1 = true;
        self.signal.notify_all();
    }
}
impl Clock for GateClock {
    fn now_ms(&self) -> f64 {
        if self.calls.fetch_add(1, Ordering::AcqRel) + 1 == self.at {
            let mut lock = self.gate.lock().unwrap();
            lock.0 = true;
            self.signal.notify_all();
            let _ = self
                .signal
                .wait_timeout_while(lock, Duration::from_secs(10), |s| !s.1)
                .unwrap();
        }
        0.0
    }
}
#[test]
fn direct_stop_and_parent_lookup_do_not_wait_for_cas_and_queue_tokens_stay_independent() {
    let source = snapshot(&[("long", "let partial=7; map(fn(k)=>sin(k),range(1,100000))")]);
    let clock = Arc::new(GateClock::new(3));
    let worker = KernelWorker::with_clock(
        64 * 1024 * 1024,
        12,
        CheckpointLimits::default(),
        clock.clone(),
    )
    .unwrap();
    let parent = worker.register(state(&source, &[])).unwrap();
    let long = job(&worker, &parent, source.clone(), "running", "long");
    let token = long.cancel.clone();
    let result = worker.submit(long).unwrap();
    clock.entered();
    let start = Instant::now();
    assert!(worker.state(&parent).is_ok());
    assert!(worker.cancel("running").unwrap());
    assert!(start.elapsed() < Duration::from_secs(1));
    let second = job(&worker, &parent, source.clone(), "queued", "long");
    let queued = worker.submit(second).unwrap();
    let third = job(&worker, &parent, source.clone(), "queued2", "long");
    let queued2 = worker.submit(third).unwrap();
    assert!(matches!(
        worker.submit(job(&worker, &parent, source.clone(), "full", "long")),
        Err(KernelWorkerError::QueueFull)
    ));
    let mut reused = job(&worker, &parent, source.clone(), "reused", "long");
    reused.cancel = token;
    assert!(matches!(
        worker.submit(reused),
        Err(KernelWorkerError::ReusedToken)
    ));
    worker.cancel("queued").unwrap();
    worker.cancel("queued2").unwrap();
    clock.release();
    let candidate = result
        .recv_timeout(Duration::from_secs(10))
        .unwrap()
        .unwrap();
    assert!(candidate.is_cancelled());
    assert_eq!(candidate.boundary().status, CellStatus::Error);
    assert_eq!(candidate.boundary().successful_statements, 1);
    assert_eq!(probe(candidate.state(), "partial"), "7");
    assert!(matches!(
        queued.recv_timeout(Duration::from_secs(10)).unwrap(),
        Err(KernelWorkerError::Cancelled)
    ));
    assert!(matches!(
        queued2.recv_timeout(Duration::from_secs(10)).unwrap(),
        Err(KernelWorkerError::Cancelled)
    ));
    assert_eq!(probe(&worker.state(&parent).unwrap(), "partial"), "partial");
    let later = receive(
        &worker,
        job(
            &worker,
            &parent,
            change(source, "long", "2+2", 1, 1),
            "later-independent",
            "long",
        ),
    )
    .unwrap();
    assert!(!later.is_cancelled());
    assert_eq!(probe(later.state(), "Out(1)"), "4");
    close(&worker);
}
#[test]
fn wrong_source_scope_epoch_and_frozen_cancel_are_rejected_or_preserved_as_actual_facts() {
    let source = snapshot(&[("a", "let a=2")]);
    let worker = KernelWorker::new(64 * 1024 * 1024, 12, CheckpointLimits::default()).unwrap();
    let parent = worker.register(state(&source, &[])).unwrap();
    let mut bad = job(&worker, &parent, source.clone(), "wrong-hash", "a");
    bad.source.snapshot_hash = "0".repeat(64);
    assert!(matches!(
        receive(&worker, bad),
        Err(KernelWorkerError::Invalid)
    ));
    let mut bad = job(
        &worker,
        &parent,
        change(source.clone(), "a", "let a=5", 1, 0),
        "wrong-epoch",
        "a",
    );
    assert!(matches!(
        receive(&worker, bad),
        Err(KernelWorkerError::Invalid)
    ));
    bad = job(&worker, &parent, source.clone(), "wrong-document", "a");
    bad.source.document_id = "different".into();
    bad.source.snapshot_hash = snapshot_hash(&bad.source);
    assert!(matches!(
        receive(&worker, bad),
        Err(KernelWorkerError::Invalid)
    ));
    let candidate = receive(
        &worker,
        job(&worker, &parent, source.clone(), "frozen", "a"),
    )
    .unwrap();
    assert!(!candidate.is_cancelled());
    assert!(worker.cancel("frozen").unwrap());
    assert!(candidate.is_cancelled());
    assert!(matches!(
        worker.submit(job(&worker, &parent, source, "frozen", "a")),
        Err(KernelWorkerError::DuplicateOperation)
    ));
    worker.discard(candidate.checkpoint_ref()).unwrap();
    assert!(worker.state(candidate.checkpoint_ref()).is_err());
    assert_eq!(probe(candidate.state(), "a"), "2");
    close(&worker);
}

#[test]
fn discarded_random_and_out_state_never_become_the_next_implicit_parent() {
    let source = snapshot(&[("seed", "seed_random(42)"), ("draw", "random_uniform()")]);
    let worker = KernelWorker::new(64 * 1024 * 1024, 12, CheckpointLimits::default()).unwrap();
    let parent = worker.register(state(&source, &["seed"])).unwrap();
    let original = worker.state(&parent).unwrap();
    let next_from_parent = probe(&original, "random_uniform()");
    let first = receive(
        &worker,
        job(&worker, &parent, source.clone(), "first-draw", "draw"),
    )
    .unwrap();
    let first_draw = probe(first.state(), "Out(2)");
    assert_eq!(first_draw, next_from_parent);
    let next_from_first = probe(first.state(), "random_uniform()");
    assert_ne!(next_from_first, next_from_parent);
    worker.discard(first.checkpoint_ref()).unwrap();
    drop(first);
    let second = receive(
        &worker,
        job(&worker, &parent, source, "second-draw", "draw"),
    )
    .unwrap();
    assert_eq!(probe(second.state(), "Out(2)"), first_draw);
    assert_eq!(probe(second.state(), "random_uniform()"), next_from_first);
    assert_eq!(probe(&original, "random_uniform()"), next_from_parent);
    close(&worker);
}

#[test]
fn registry_capacity_failure_and_close_do_not_publish_or_overwrite_the_parent() {
    let source = snapshot(&[("a", "let a=2")]);
    let worker = KernelWorker::new(64 * 1024 * 1024, 1, CheckpointLimits::default()).unwrap();
    let parent = worker.register(state(&source, &[])).unwrap();
    let original = worker.state(&parent).unwrap();
    let hash = original.hash().to_owned();
    assert!(matches!(
        receive(
            &worker,
            job(&worker, &parent, source.clone(), "full-registry", "a")
        ),
        Err(KernelWorkerError::State(
            om_host_service::kernel::KernelStateError::Limit
        ))
    ));
    assert_eq!(worker.state(&parent).unwrap().hash(), hash);
    assert_eq!(probe(&original, "a"), "a");
    close(&worker);
    assert!(matches!(
        worker.submit(job(&worker, &parent, source, "after-close", "a")),
        Err(KernelWorkerError::Closing)
    ));
    assert!(worker.begin_close().unwrap().is_none());
}

#[test]
fn explicit_nonreactive_mode_keeps_original_reassignment_semantics() {
    let source = snapshot(&[("first", "let a=2"), ("second", "let a=3")]);
    let mut config = KernelConfig::default();
    config.general.reactive = false;
    let mut session = Session::new(config, None);
    session
        .apply_source_file_without_evaluation(kernel_file(&source.file))
        .unwrap();
    assert_eq!(evaluate(&mut session, "first", "let a=2"), "2");
    let frozen = KernelState::capture(
        &mut session,
        binding(&source),
        CheckpointLimits::default(),
        &Interrupt::default(),
    )
    .unwrap();
    let worker = KernelWorker::new(64 * 1024 * 1024, 12, CheckpointLimits::default()).unwrap();
    let parent = worker.register(frozen).unwrap();
    let candidate = receive(
        &worker,
        job(&worker, &parent, source, "nonreactive-reassign", "second"),
    )
    .unwrap();
    assert_eq!(probe(candidate.state(), "a"), "3");
    assert_eq!(probe(&worker.state(&parent).unwrap(), "a"), "2");
    close(&worker);
}
