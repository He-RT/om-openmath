//! Bounded dedicated main math thread; registry lookup and direct stop never wait behind CAS.
use super::*;
use std::{
    collections::BTreeMap,
    sync::{Weak, mpsc},
    thread::{self, JoinHandle},
    time::Instant,
};
struct HostClock(Instant);
impl Clock for HostClock {
    fn now_ms(&self) -> f64 {
        self.0.elapsed().as_secs_f64() * 1000.0
    }
}
struct Message {
    job: KernelJob,
    parent: Arc<KernelState>,
    reply: mpsc::SyncSender<Result<KernelCandidate, KernelWorkerError>>,
    lifecycle: Arc<Lifecycle>,
}
struct Admission {
    token: Weak<AtomicBool>,
    lifecycle: Weak<Lifecycle>,
}
/// A dedicated worker with no active-state decision. Close is two-phase; join off the UI thread.
pub struct KernelWorker {
    sender: Mutex<Option<mpsc::SyncSender<Message>>>,
    pool: Arc<Mutex<KernelStatePool>>,
    admissions: Mutex<BTreeMap<String, Admission>>,
    closed: Arc<AtomicBool>,
    owner: Mutex<Option<JoinHandle<()>>>,
}
impl KernelWorker {
    /// Start one main CAS owner with a two-job queue and bounded immutable state registry.
    pub fn new(
        max_bytes: usize,
        max_states: usize,
        limits: CheckpointLimits,
    ) -> Result<Self, KernelWorkerError> {
        Self::with_clock(
            max_bytes,
            max_states,
            limits,
            Arc::new(HostClock(Instant::now())),
        )
    }
    /// Trusted host/fixture clock, never a model-configurable execution callback.
    pub fn with_clock(
        max_bytes: usize,
        max_states: usize,
        limits: CheckpointLimits,
        clock: Arc<dyn Clock>,
    ) -> Result<Self, KernelWorkerError> {
        let pool = Arc::new(Mutex::new(KernelStatePool::new(max_bytes, max_states)?));
        let closed = Arc::new(AtomicBool::new(false));
        let (sender, receiver) = mpsc::sync_channel::<Message>(2);
        let states = pool.clone();
        let closing = closed.clone();
        let owner = thread::Builder::new()
            .name("openmath-main-kernel-worker".into())
            .spawn(move || {
                while let Ok(message) = receiver.recv() {
                    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        message.lifecycle.start();
                        execute(
                            message.job,
                            message.lifecycle,
                            message.parent,
                            &states,
                            limits,
                            clock.clone(),
                            &closing,
                        )
                    }))
                    .unwrap_or(Err(KernelWorkerError::Internal));
                    if let Err(mpsc::SendError(Ok(candidate))) = message.reply.send(outcome) {
                        // Lost recipient cannot keep a candidate lookup alive as an accidental head.
                        if let Ok(mut pool) = states.lock() {
                            let _ = pool.revoke(candidate.checkpoint_ref());
                        }
                    }
                }
            })
            .map_err(|_| KernelWorkerError::Internal)?;
        Ok(Self {
            sender: Mutex::new(Some(sender)),
            pool,
            admissions: Mutex::new(BTreeMap::new()),
            closed,
            owner: Mutex::new(Some(owner)),
        })
    }
    /// Register externally frozen initial/restored bytes on a short lock. This grants no active role.
    pub fn register(&self, state: KernelState) -> Result<String, KernelWorkerError> {
        if self.closed.load(Ordering::Acquire) {
            return Err(KernelWorkerError::Closing);
        }
        Ok(self
            .pool
            .lock()
            .map_err(|_| KernelWorkerError::Internal)?
            .register(state)?)
    }
    /// Actual scoped state lookup without queueing behind a running mathematical job.
    pub fn state(&self, id: &str) -> Result<Arc<KernelState>, KernelWorkerError> {
        Ok(self
            .pool
            .lock()
            .map_err(|_| KernelWorkerError::Internal)?
            .state(id)?)
    }
    /// Actual pool reservation after reclaiming genuinely unpinned retired state envelopes.
    pub fn reserved_bytes(&self) -> Result<usize, KernelWorkerError> {
        let mut pool = self.pool.lock().map_err(|_| KernelWorkerError::Internal)?;
        pool.reap();
        Ok(pool.reserved_bytes())
    }
    /// Revoke unaccepted/obsolete lookup. Queued/running/reader pins keep their reservations.
    pub fn discard(&self, id: &str) -> Result<(), KernelWorkerError> {
        Ok(self
            .pool
            .lock()
            .map_err(|_| KernelWorkerError::Internal)?
            .revoke(id)?)
    }
    /// Queue only the explicit selected parent. Receiver delivers actual candidate or real failure.
    pub fn submit(
        &self,
        job: KernelJob,
    ) -> Result<mpsc::Receiver<Result<KernelCandidate, KernelWorkerError>>, KernelWorkerError> {
        let mut admissions = self
            .admissions
            .lock()
            .map_err(|_| KernelWorkerError::Internal)?;
        if self.closed.load(Ordering::Acquire) {
            return Err(KernelWorkerError::Closing);
        }
        if !identity(&job.operation_id) {
            return Err(KernelWorkerError::Invalid);
        }
        if admissions.contains_key(&job.operation_id) {
            return Err(KernelWorkerError::DuplicateOperation);
        }
        if admissions
            .values()
            .any(|entry| entry.token.ptr_eq(&Arc::downgrade(&job.cancel)))
        {
            return Err(KernelWorkerError::ReusedToken);
        }
        if admissions.len() >= 4096 {
            return Err(KernelWorkerError::Limit);
        }
        let parent = self.state(&job.parent_checkpoint_ref)?;
        let (reply, receiver) = mpsc::sync_channel(1);
        let id = job.operation_id.clone();
        let token = Arc::downgrade(&job.cancel);
        let lifecycle = Arc::new(Lifecycle::new(job.cancel.clone()));
        let sender = self
            .sender
            .lock()
            .map_err(|_| KernelWorkerError::Internal)?;
        let sender = sender.as_ref().ok_or(KernelWorkerError::Closing)?;
        sender
            .try_send(Message {
                job,
                parent,
                reply,
                lifecycle: lifecycle.clone(),
            })
            .map_err(|e| match e {
                mpsc::TrySendError::Full(_) => KernelWorkerError::QueueFull,
                mpsc::TrySendError::Disconnected(_) => KernelWorkerError::Closing,
            })?;
        admissions.insert(
            id,
            Admission {
                token,
                lifecycle: Arc::downgrade(&lifecycle),
            },
        );
        Ok(receiver)
    }
    /// Direct stop also reaches a frozen candidate that is awaiting coordinator acceptance.
    pub fn cancel(&self, operation: &str) -> Result<bool, KernelWorkerError> {
        let admissions = self
            .admissions
            .lock()
            .map_err(|_| KernelWorkerError::Internal)?;
        let entry = admissions
            .get(operation)
            .ok_or(KernelWorkerError::Invalid)?;
        if let Some(lifecycle) = entry.lifecycle.upgrade() {
            lifecycle.cancel()
        } else {
            Ok(false)
        }
    }
    /// Actual worker start fact, independent from admission and from control polling times.
    pub fn started(&self, operation: &str) -> bool {
        self.admissions
            .lock()
            .ok()
            .and_then(|entries| {
                entries
                    .get(operation)
                    .and_then(|entry| entry.lifecycle.upgrade())
            })
            .is_some_and(|life| life.started())
    }
    /// Close admission/signal tokens/drop queue sender now; caller joins returned thread off UI.
    pub fn begin_close(&self) -> Result<Option<JoinHandle<()>>, KernelWorkerError> {
        let admissions = self
            .admissions
            .lock()
            .map_err(|_| KernelWorkerError::Internal)?;
        self.closed.store(true, Ordering::Release);
        for lifecycle in admissions
            .values()
            .filter_map(|entry| entry.lifecycle.upgrade())
        {
            let _ = lifecycle.cancel();
        }
        self.sender
            .lock()
            .map_err(|_| KernelWorkerError::Internal)?
            .take();
        Ok(self
            .owner
            .lock()
            .map_err(|_| KernelWorkerError::Internal)?
            .take())
    }
}
impl Drop for KernelWorker {
    fn drop(&mut self) {
        let _ = self.begin_close();
    }
}
