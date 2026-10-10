//! Dedicated immutable-result owner. No data parsing/evaluation/formatting runs on source/UI locks.
mod queries;
use super::*;
use crate::protocol::{generated::*, validation};
use om_num::ctx::Clock;
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, VecDeque},
    sync::{Mutex, OnceLock, atomic::Ordering, mpsc},
    thread::{self, JoinHandle},
    time::Instant,
};
struct HostClock(Instant);
impl Clock for HostClock {
    fn now_ms(&self) -> f64 {
        self.0.elapsed().as_secs_f64() * 1000.0
    }
}
enum Message {
    Accepted(Box<AcceptedKernelState>),
    Read {
        command: Box<NativeResultHostCommand>,
        scope: Box<ReferenceScope>,
        token: Arc<AtomicBool>,
    },
}
struct Entry {
    fingerprint: String,
    reply: NativeResultHostReply,
    cancel: Arc<AtomicBool>,
    scope: ReferenceScope,
}
struct State {
    entries: BTreeMap<String, Entry>,
    completed: VecDeque<String>,
}
/// Trusted fixture pause observes an actual running inspection; it grants no model/UI write access.
#[derive(Clone, Default)]
pub struct ResultWorkerProbes {
    /// Runs only on the real result thread after it marks the inspection running.
    pub before_query: Option<Arc<dyn Fn() + Send + Sync>>,
}
/// Bounded result worker plus compact original-ID request facts. Cancel is direct and independent.
pub struct ResultRuntime {
    sender: Mutex<Option<mpsc::SyncSender<Message>>>,
    state: Arc<Mutex<State>>,
    closing: Arc<AtomicBool>,
    owner: Mutex<Option<JoinHandle<()>>>,
}
fn request_id(command: &NativeResultHostCommand) -> &str {
    match command {
        NativeResultHostCommand::ResultManifest(b) => &b.request_id,
        NativeResultHostCommand::ResultInspect(b) => &b.request_id,
        NativeResultHostCommand::ResultStatus(b) => &b.request_id,
        NativeResultHostCommand::ResultRevoke(b) => &b.request_id,
    }
}
fn terminal(phase: &NativeResultHostReplyPhase) -> bool {
    matches!(
        phase,
        NativeResultHostReplyPhase::Completed
            | NativeResultHostReplyPhase::Cancelled
            | NativeResultHostReplyPhase::Failed
    )
}
fn base(scope: &ReferenceScope, id: &str) -> Result<NativeResultHostReply, String> {
    Ok(NativeResultHostReply {
        protocol_version: 1,
        request_id: id.into(),
        runtime_instance_id: scope.runtime.clone(),
        document_id: scope.document.clone(),
        document_generation: Serial::new(scope.generation)?,
        document_revision: Serial::new(scope.revision)?,
        execution_epoch: Serial::new(scope.execution_epoch)?,
        kernel_state_revision: Serial::new(scope.definition_revision)?,
        phase: NativeResultHostReplyPhase::Queued,
        binding: Nullable(None),
        payload: json!({}),
        error_code: Nullable(None),
    })
}
fn finish(state: &Mutex<State>, id: &str, value: Result<(Option<ResultBinding>, Value), String>) {
    let Ok(mut state) = state.lock() else { return };
    let Some(entry) = state.entries.get_mut(id) else {
        return;
    };
    if entry.cancel.load(Ordering::Acquire) {
        entry.reply.phase = NativeResultHostReplyPhase::Cancelled;
        entry.reply.error_code = Nullable(Some("CANCELLED".into()));
    } else {
        match value {
            Ok((binding, payload)) => {
                entry.reply.phase = NativeResultHostReplyPhase::Completed;
                entry.reply.binding = Nullable(binding);
                entry.reply.payload = payload;
            }
            Err(code) => {
                entry.reply.phase = NativeResultHostReplyPhase::Failed;
                entry.reply.error_code = Nullable(Some(code));
            }
        }
    }
    state.completed.push_back(id.into());
    while state.completed.len() > 32 {
        if let Some(id) = state.completed.pop_front()
            && let Some(old) = state.entries.get_mut(&id)
        {
            old.reply.binding = Nullable(None);
            old.reply.payload = json!({});
            old.reply.phase = NativeResultHostReplyPhase::Failed;
            old.reply.error_code = Nullable(Some("RESULT_DETAILS_EXPIRED".into()));
        }
    }
}
impl ResultRuntime {
    /// Known request identity across shared native operation namespaces, without queueing work.
    pub fn has_request(&self, id: &str) -> bool {
        self.state
            .lock()
            .is_ok_and(|state| state.entries.contains_key(id))
    }
    /// Create a dedicated result owner with host entropy, four queued messages and 32 requests.
    pub fn new() -> Result<Self, String> {
        Self::with_probes(ResultWorkerProbes::default())
    }
    /// Trusted test hook; no serialized request can configure this callback or pause behavior.
    pub fn with_probes(probes: ResultWorkerProbes) -> Result<Self, String> {
        let mut key = [0; 32];
        getrandom::fill(&mut key).map_err(|_| "ENTROPY_UNAVAILABLE")?;
        let mut store =
            ResultStore::new(key, 64 * 1024 * 1024, 32).map_err(|_| "BUDGET_EXCEEDED")?;
        let state = Arc::new(Mutex::new(State {
            entries: BTreeMap::new(),
            completed: VecDeque::new(),
        }));
        let closing = Arc::new(AtomicBool::new(false));
        let (sender, receiver) = mpsc::sync_channel::<Message>(4);
        let replies = state.clone();
        let closed = closing.clone();
        let owner = thread::Builder::new()
            .name("openmath-result-worker".into())
            .spawn(move || {
                let clock: Arc<dyn Clock> = Arc::new(HostClock(Instant::now()));
                while let Ok(message) = receiver.recv() {
                    match message {
                        Message::Accepted(accepted) => {
                            if closed.load(Ordering::Acquire) {
                                continue;
                            }
                            let context = Interrupt {
                                clock: Some(clock.clone()),
                                deadline_ms: Some(clock.now_ms() + 5000.),
                                ..Interrupt::default()
                            };
                            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                                // Cache capacity/codec failure changes availability only. Durable main math
                                // acceptance already exists and cannot be undone or replayed by this worker.
                                let _ = store.set_current(&accepted, &context);
                                if accepted.receipt().result_id.0.is_some()
                                    && let Ok(record) = StoredResult::capture(
                                        &accepted,
                                        CheckpointLimits::default(),
                                        &context,
                                    )
                                {
                                    let _ = store.register_recent(record);
                                }
                            }));
                        }
                        Message::Read {
                            command,
                            scope,
                            token,
                        } => {
                            let id = request_id(&command).to_owned();
                            if let Ok(mut entries) = replies.lock()
                                && let Some(entry) = entries.entries.get_mut(&id)
                            {
                                entry.reply.phase = NativeResultHostReplyPhase::Running;
                            }
                            let context = Interrupt {
                                flag: token,
                                clock: Some(clock.clone()),
                                deadline_ms: Some(clock.now_ms() + 5000.),
                                steps_left: std::cell::Cell::new(4 * 1024 * 1024),
                            };
                            let result =
                                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                                    if let Some(probe) = &probes.before_query {
                                        probe();
                                    }
                                    queries::execute(
                                        &mut store,
                                        &command,
                                        &scope,
                                        clock.now_ms() as u64,
                                        &context,
                                    )
                                }))
                                .unwrap_or_else(|_| Err("RESULT_WORKER_PANIC".into()));
                            finish(&replies, &id, result);
                        }
                    }
                }
            })
            .map_err(|_| "INTERNAL_ERROR")?;
        Ok(Self {
            sender: Mutex::new(Some(sender)),
            state,
            closing,
            owner: Mutex::new(Some(owner)),
        })
    }
    /// Retain an actual just-accepted snapshot on the independent worker; no source replay/IO here.
    pub fn accepted(&self, accepted: AcceptedKernelState) -> Result<(), String> {
        if self.closing.load(Ordering::Acquire) {
            return Err("HOST_CLOSING".into());
        }
        self.sender
            .lock()
            .map_err(|_| "INTERNAL_ERROR")?
            .as_ref()
            .ok_or("HOST_CLOSING")?
            .try_send(Message::Accepted(Box::new(accepted)))
            .map_err(|_| "BUDGET_EXCEEDED".into())
    }
    /// Queue a strictly validated readonly command or read its original bounded status.
    pub fn command(
        &self,
        command: NativeResultHostCommand,
        scope: &ReferenceScope,
    ) -> Result<NativeResultHostReply, String> {
        scope.validate().map_err(|_| "STALE_DOCUMENT")?;
        if !scope.can_read {
            return Err("PERMISSION_DENIED".into());
        }
        let id = request_id(&command);
        let mut state = self.state.lock().map_err(|_| "INTERNAL_ERROR")?;
        if let NativeResultHostCommand::ResultStatus(_) = command {
            let entry = state.entries.get(id).ok_or("INVALID_REFERENCE")?;
            let original = &entry.scope;
            if original.runtime != scope.runtime
                || original.document != scope.document
                || original.generation != scope.generation
                || original.task != scope.task
                || original.task_generation != scope.task_generation
                || original.grant_revision != scope.grant_revision
                || original.metadata_revision != scope.metadata_revision
            {
                return Err("PERMISSION_DENIED".into());
            }
            return Ok(entry.reply.clone());
        }
        if self.closing.load(Ordering::Acquire) {
            return Err("HOST_CLOSING".into());
        }
        let encoded = serde_json::to_vec(&command).map_err(|_| "INVALID_ARGUMENT")?;
        let mut hasher = sha2::Sha256::new();
        use sha2::Digest;
        hasher.update(encoded);
        hasher.update(serde_json::to_vec(scope).map_err(|_| "INVALID_ARGUMENT")?);
        let fingerprint = format!("{:x}", hasher.finalize());
        if let Some(entry) = state.entries.get(id) {
            if entry.fingerprint != fingerprint {
                return Err("DUPLICATE_OPERATION".into());
            }
            return Ok(entry.reply.clone());
        }
        if state.entries.len() >= 4096
            || state
                .entries
                .values()
                .filter(|e| !terminal(&e.reply.phase))
                .count()
                >= 32
        {
            return Err("BUDGET_EXCEEDED".into());
        }
        let reply = base(scope, id)?;
        let token = Arc::new(AtomicBool::new(false));
        self.sender
            .lock()
            .map_err(|_| "INTERNAL_ERROR")?
            .as_ref()
            .ok_or("HOST_CLOSING")?
            .try_send(Message::Read {
                command: Box::new(command.clone()),
                scope: Box::new(scope.clone()),
                token: token.clone(),
            })
            .map_err(|_| "BUDGET_EXCEEDED")?;
        state.entries.insert(
            id.into(),
            Entry {
                fingerprint,
                reply: reply.clone(),
                cancel: token,
                scope: scope.clone(),
            },
        );
        Ok(reply)
    }
    /// Cancel a real pending inspection, independent from main computation and event queues.
    pub fn cancel(&self, id: &str) -> Option<bool> {
        let state = self.state.lock().ok()?;
        let entry = state.entries.get(id)?;
        if terminal(&entry.reply.phase) {
            return Some(false);
        }
        entry.cancel.store(true, Ordering::Release);
        Some(true)
    }
    /// Close admission/cancel first. Background caller joins the extracted owner after lock release.
    pub fn begin_close(&self) {
        self.closing.store(true, Ordering::Release);
        if let Ok(state) = self.state.lock() {
            for entry in state.entries.values().filter(|e| !terminal(&e.reply.phase)) {
                entry.cancel.store(true, Ordering::Release);
            }
        }
        if let Ok(mut sender) = self.sender.lock() {
            sender.take();
        }
    }
    /// Extract worker for background join only; this method performs no blocking wait.
    pub fn take_thread(&self) -> Option<JoinHandle<()>> {
        self.begin_close();
        self.owner.lock().ok()?.take()
    }
}
impl Drop for ResultRuntime {
    fn drop(&mut self) {
        self.begin_close();
    }
}
/// Strict host-only result control DTO. It does not register an Agent tool or grant permissions.
pub fn decode_command(bytes: &[u8]) -> Result<NativeResultHostCommand, String> {
    static ROOT: OnceLock<Value> = OnceLock::new();
    let root = ROOT.get_or_init(|| {
        serde_json::from_str(include_str!(
            "../../../../docs/design/native-result-store.schema.json"
        ))
        .expect("checked result schema")
    });
    validation::decode(bytes, &root["$defs"]["NativeResultHostCommand"], root)
}
