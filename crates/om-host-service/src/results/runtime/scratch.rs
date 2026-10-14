use super::*;
/// Trusted fixture pause at a real independently scheduled scratch job; never wire-configurable.
#[derive(Default)]
pub struct ScratchWorkerProbes {
    /// Called on scratch thread before evaluation, after its real running status is published.
    pub before_evaluate: Option<Arc<dyn Fn() + Send + Sync>>,
}
pub(super) struct Job {
    pub request: FrozenScratch,
    pub scope: Box<ReferenceScope>,
    pub token: Arc<AtomicBool>,
}
pub(super) fn spawn(
    delivery: mpsc::SyncSender<Message>,
    state: Arc<Mutex<State>>,
    probes: ScratchWorkerProbes,
) -> Result<(mpsc::SyncSender<Job>, JoinHandle<()>), String> {
    let (sender, receiver) = mpsc::sync_channel::<Job>(4);
    let owner = thread::Builder::new()
        .name("openmath-isolated-scratch-worker".into())
        .spawn(move || {
            let clock: Arc<dyn Clock> = Arc::new(HostClock(Instant::now()));
            while let Ok(job) = receiver.recv() {
                let id = job.request.id().to_owned();
                if let Ok(mut state) = state.lock()
                    && let Some(entry) = state.entries.get_mut(&id)
                {
                    entry.reply.phase = NativeResultHostReplyPhase::Running;
                }
                let context = Interrupt {
                    flag: job.token.clone(),
                    clock: Some(clock.clone()),
                    deadline_ms: Some(clock.now_ms() + 5000.),
                    steps_left: std::cell::Cell::new(4 * 1024 * 1024),
                };
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    if let Some(probe) = &probes.before_evaluate {
                        probe();
                    }
                    job.request.evaluate(&context)
                }))
                .unwrap_or_else(|_| Err("SCRATCH_WORKER_PANIC".into()));
                if delivery
                    .try_send(Message::ScratchReady {
                        id: id.clone(),
                        scope: job.scope,
                        result: result.map(Box::new),
                        token: job.token,
                    })
                    .is_err()
                {
                    finish(&state, &id, Err("BUDGET_EXCEEDED".into()));
                }
            }
        })
        .map_err(|_| "INTERNAL_ERROR")?;
    Ok((sender, owner))
}
