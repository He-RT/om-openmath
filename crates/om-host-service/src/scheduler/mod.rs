//! Safe native coordinator with separate bounded control, kernel, editor and auxiliary queues.
mod events;
mod operations;
#[cfg(test)]
mod source_gate_tests;
mod workers;
use crate::protocol::{Nullable, generated::*, wire};
use events::Events;
use operations::Operations;
pub use operations::{OperationStatus, Phase};
use serde_json::{Value, json};
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread::{self, JoinHandle},
};
use workers::{Job, Worker};

pub(crate) enum Control {
    Request(Box<RequestEnvelope>),
    Finished {
        request: String,
        operation: String,
        result: Result<Value, String>,
    },
    Stop,
}
struct Shared {
    source: Mutex<Option<crate::document::endpoint::SourceEndpoint>>,
    source_generation: Mutex<crate::protocol::Serial>,
    admission: Mutex<()>,
    projection: Mutex<HostPhase>,
    closed: AtomicBool,
    operations: Arc<Operations>,
    events: Events,
    runtime: String,
}
/// A native runtime. The control owner never evaluates CAS, waits on network or performs disk IO.
pub struct NativeHost {
    sender: mpsc::SyncSender<Control>,
    shared: Arc<Shared>,
    owner: Mutex<Option<JoinHandle<()>>>,
}
impl NativeHost {
    /// Start independently owned bounded channels from a validated initialization frame.
    pub fn new(init: HostInit) -> Result<Self, String> {
        // Public safe callers cannot bypass bounds by constructing a DTO directly.
        wire::decode_init(&serde_json::to_vec(&init).map_err(|_| "INVALID_ARGUMENT")?)?;
        let shared = Arc::new(Shared {
            source: Mutex::new(None),
            source_generation: Mutex::new(crate::protocol::Serial::new(0)?),
            admission: Mutex::new(()),
            projection: Mutex::new(HostPhase::Starting),
            closed: AtomicBool::new(false),
            operations: Arc::new(Operations::new(init.max_pending_operations as usize)),
            events: Events::new(
                init.runtime_instance_id.clone(),
                init.event_capacity as usize,
            ),
            runtime: init.runtime_instance_id,
        });
        let (sender, receiver) =
            mpsc::sync_channel::<Control>(init.max_pending_operations as usize + 8);
        let kernel = Worker::new(
            "openmath-native-kernel",
            sender.clone(),
            shared.operations.clone(),
        )?;
        let editor = Worker::new(
            "openmath-native-editor",
            sender.clone(),
            shared.operations.clone(),
        )?;
        let auxiliary = Worker::new(
            "openmath-native-auxiliary",
            sender.clone(),
            shared.operations.clone(),
        )?;
        let state = shared.clone();
        let owner = thread::Builder::new()
            .name("openmath-document-coordinator".into())
            .spawn(move || {
                {
                let Ok(mut phase) = state.projection.lock() else { return; };
                *phase = HostPhase::Ready;
                state.events.emit(EventEnvelopeEventKind::HostReady, None, None, json!({
                    "protocol_version":1, "document_storage_ready":false,
                    "registered_requests":["get_state","get_capabilities","get_function_catalog","analyze_editor","evaluate_scratch","get_operation_status"]
                }));
                }
                while let Ok(control) = receiver.recv() {
                    if state.closed.load(Ordering::Acquire) { break; }
                    let Ok(_projection) = state.projection.lock() else { break; };
                    match control {
                        Control::Stop => break,
                        Control::Finished { request, operation, result } => {
                            if let Some(status) = state.operations.finish(&operation, result) {
                                state.events.emit(EventEnvelopeEventKind::OperationFinished, Some(request), Some(operation), serde_json::to_value(status).unwrap_or_else(|_| json!({"error":"INTERNAL_ERROR"})));
                            }
                        }
                        Control::Request(request) => dispatch(*request, &state, &kernel, &editor, &auxiliary),
                    }
                }
                state.operations.stop_all();
                drop(receiver);
                drop(kernel.sender);
                drop(editor.sender);
                drop(auxiliary.sender);
                let _ = kernel.thread.join();
                let _ = editor.thread.join();
                let _ = auxiliary.thread.join();
                for status in state.operations.all() {
                    let Ok(_projection) = state.projection.lock() else { break; };
                    if !matches!(status.phase, Phase::Completed | Phase::Failed | Phase::Cancelled)
                        && let Some(done) = state.operations.finish(&status.operation_ref, Err("CANCELLED".into()))
                    {
                        state.events.emit(EventEnvelopeEventKind::OperationFinished, None, Some(status.operation_ref), serde_json::to_value(done).unwrap_or(Value::Null));
                    }
                }
                if let Ok(mut phase) = state.projection.lock() { *phase = HostPhase::Closed; }
                state.events.close();
            }).map_err(|_| "Cannot start document coordinator".to_owned())?;
        Ok(Self {
            sender,
            shared,
            owner: Mutex::new(Some(owner)),
        })
    }
    /// Decode/validate and enqueue quickly. An admission receipt is not a computed result.
    pub fn submit(&self, bytes: &[u8]) -> Result<AdmissionReceipt, String> {
        if self.shared.closed.load(Ordering::Acquire) {
            return Err("HOST_CLOSING".into());
        }
        let request = wire::decode_request(bytes)?;
        let _admission = self.shared.admission.lock().map_err(|_| "INTERNAL_ERROR")?;
        if self.shared.closed.load(Ordering::Acquire) {
            return Err("HOST_CLOSING".into());
        }
        wire::check_owner_binding(&request, &self.shared.runtime, None).map_err(str::to_owned)?;
        if request.task_binding.0.is_some() {
            return Err("PERMISSION_DENIED".into());
        }
        let operation = request
            .operation_id
            .0
            .clone()
            .unwrap_or_else(|| request.request_ref.clone());
        self.shared
            .operations
            .admit(&operation)
            .map_err(str::to_owned)?;
        let receipt = AdmissionReceipt {
            protocol_version: 1,
            request_ref: request.request_ref.clone(),
            operation_ref: Nullable(Some(operation.clone())),
            accepted: true,
            error: Nullable(None),
        };
        if self
            .sender
            .try_send(Control::Request(Box::new(request)))
            .is_err()
        {
            self.shared
                .operations
                .finish(&operation, Err("BUDGET_EXCEEDED".into()));
            return Err("BUDGET_EXCEEDED".into());
        }
        Ok(receipt)
    }
    /// Direct cancellation is independent of control and CAS queues.
    pub fn cancel(&self, operation: &str) -> Result<bool, String> {
        if let Some(source) = self
            .shared
            .source
            .lock()
            .map_err(|_| "INTERNAL_ERROR")?
            .as_mut()
            && let Some(result) = source.cancel(operation)
        {
            return Ok(result);
        }
        let _projection = self
            .shared
            .projection
            .lock()
            .map_err(|_| "INTERNAL_ERROR")?;
        self.shared
            .operations
            .cancel(operation)
            .map_err(str::to_owned)
    }
    /// Background event pump: bounded wait/bytes and explicit resync when a subscriber falls behind.
    pub fn next_events(&self, wait_ms: u32, max_bytes: usize) -> Result<EventBatch, String> {
        self.shared
            .events
            .poll(wait_ms, max_bytes)
            .map_err(str::to_owned)
    }
    /// Host-internal source port: physical store/editor duties are isolated from model tools.
    pub fn source_command(&self, bytes: &[u8]) -> Result<NativeSourceHostReply, String> {
        if self.shared.closed.load(Ordering::Acquire) {
            return Err("HOST_CLOSING".into());
        }
        let command = crate::document::endpoint::decode_source_command(bytes)?;
        // The physical writer holds the native editor fence lock during this final short call.
        // Never wait there behind parsing/preview; reject so SQLite rolls back and typing resumes.
        let mut slot = if matches!(&command, NativeSourceHostCommand::SourceBarrier(_)) {
            self.shared.source.try_lock().map_err(|error| match error {
                std::sync::TryLockError::WouldBlock => "EDITING_BUSY",
                std::sync::TryLockError::Poisoned(_) => "INTERNAL_ERROR",
            })?
        } else {
            self.shared.source.lock().map_err(|_| "INTERNAL_ERROR")?
        };
        if let NativeSourceHostCommand::SourceOpen(body) = command {
            if slot.is_some() {
                return Err("SOURCE_ALREADY_OPEN".into());
            }
            let mut generation = self
                .shared
                .source_generation
                .lock()
                .map_err(|_| "INTERNAL_ERROR")?;
            *generation = generation.checked_next()?;
            *slot = Some(crate::document::endpoint::SourceEndpoint::open(
                body.snapshot.clone(),
                body.store_id,
                self.shared.runtime.clone(),
                *generation,
                body.calculation.0,
                body.config_revision,
            )?);
            return Ok(NativeSourceHostReply {
                protocol_version: 1,
                kind: NativeSourceHostReplyKind::Opened,
                owner_time_ms: crate::protocol::Serial::new(0)?,
                document_generation: *generation,
                snapshot: Nullable(Some(body.snapshot)),
                plan: Nullable(None),
                preview: Nullable(None),
                reference: Nullable(None),
                operation: Nullable(None),
            });
        }
        slot.as_mut().ok_or("SOURCE_NOT_OPEN")?.command(command)
    }
    /// Trusted main-kernel control port. It uses the same source owner/gate; never CAS or disk IO.
    pub fn kernel_command(&self, bytes: &[u8]) -> Result<NativeKernelHostReply, String> {
        let command = crate::kernel::endpoint::decode_command(bytes)?;
        if self.shared.closed.load(Ordering::Acquire)
            && matches!(
                &command,
                NativeKernelHostCommand::KernelBootstrap(_)
                    | NativeKernelHostCommand::KernelRunCell(_)
                    | NativeKernelHostCommand::KernelBarrier(_)
            )
        {
            return Err("HOST_CLOSING".into());
        }
        let mut source = if matches!(&command, NativeKernelHostCommand::KernelBarrier(_)) {
            self.shared.source.try_lock().map_err(|e| match e {
                std::sync::TryLockError::WouldBlock => "EDITING_BUSY",
                std::sync::TryLockError::Poisoned(_) => "INTERNAL_ERROR",
            })?
        } else {
            self.shared.source.lock().map_err(|_| "INTERNAL_ERROR")?
        };
        let completed = matches!(&command, NativeKernelHostCommand::KernelCompleted(body) if !source.as_ref().is_some_and(|source|source.kernel_operation_completed(&body.operation_id)));
        let reply = source
            .as_mut()
            .ok_or("SOURCE_NOT_OPEN")?
            .kernel_command(command)?;
        if completed
            && reply.phase == NativeKernelHostReplyPhase::Completed
            && reply
                .receipt
                .0
                .as_ref()
                .is_some_and(|receipt| receipt.result_id.0.is_some())
        {
            self.shared.events.emit(EventEnvelopeEventKind::ResultAccepted,None,reply.operation_id.0.clone(),json!({"origin":"main","document_id":reply.document_id,"kernel_state_revision":reply.kernel_state_revision,"checkpoint_id":reply.active_checkpoint_id,"receipt":reply.receipt}));
        }
        Ok(reply)
    }
    /// Read retained real operation facts; unavailable IDs cannot be inferred to have never run.
    pub fn operation(&self, id: &str) -> Option<OperationStatus> {
        self.shared.operations.status(id)
    }
    /// Queue-independent fenced facts for a trusted event consumer. No operation is admitted.
    pub fn read_snapshot(&self, bytes: &[u8]) -> Result<HostSnapshot, String> {
        let query = wire::decode_snapshot_query(bytes)?;
        if query.runtime_instance_id != self.shared.runtime {
            return Err("STALE_RUNTIME".into());
        }
        let phase = self
            .shared
            .projection
            .lock()
            .map_err(|_| "INTERNAL_ERROR")?;
        let mut operations = self.shared.operations.summaries();
        let mut unavailable = Vec::new();
        for id in query.operation_refs {
            if let Some(mut status) = self.shared.operations.status(&id) {
                if !query.include_results {
                    status.result = None;
                }
                if let Some(index) = operations.iter().position(|op| op.operation_ref == id) {
                    operations[index] = status;
                } else {
                    operations.push(status);
                }
            } else {
                unavailable.push(id);
            }
        }
        let operations = operations
            .into_iter()
            .map(|op| HostOperationStatus {
                operation_ref: op.operation_ref,
                phase: match op.phase {
                    Phase::Queued => HostOperationStatusPhase::Queued,
                    Phase::Running => HostOperationStatusPhase::Running,
                    Phase::Cancelling => HostOperationStatusPhase::Cancelling,
                    Phase::Completed => HostOperationStatusPhase::Completed,
                    Phase::Cancelled => HostOperationStatusPhase::Cancelled,
                    Phase::Failed => HostOperationStatusPhase::Failed,
                },
                result: Nullable(op.result),
                error_code: Nullable(op.error_code),
            })
            .collect();
        Ok(HostSnapshot {
            protocol_version: 1,
            runtime_instance_id: self.shared.runtime.clone(),
            host_phase: phase.clone(),
            rust_event_sequence: self.shared.events.sequence().map_err(str::to_owned)?,
            document_binding: Nullable(None),
            document_storage_ready: false,
            operation_history_complete: false,
            operations,
            unavailable_operation_refs: unavailable,
        })
    }
    /// Revoke new admission and signal all operation tokens without joining worker threads.
    pub fn close_begin(&self) {
        let Ok(_admission) = self.shared.admission.lock() else {
            return;
        };
        if !self.shared.closed.swap(true, Ordering::AcqRel) {
            if let Ok(mut phase) = self.shared.projection.lock() {
                *phase = HostPhase::Closing;
            }
            self.shared.operations.stop_all();
            if let Ok(mut source) = self.shared.source.lock()
                && let Some(source) = source.as_mut()
            {
                source.close_kernel_begin();
            }
            let _ = self.sender.try_send(Control::Stop);
        }
    }
    /// Wait for owners only on a background caller; returns after all resources have settled.
    pub fn close_finish(&self) -> Result<(), String> {
        self.close_begin();
        if let Some(owner) = self.owner.lock().map_err(|_| "INTERNAL_ERROR")?.take() {
            owner.join().map_err(|_| "OWNER_PANIC")?;
        }
        let handles = self
            .shared
            .source
            .lock()
            .map_err(|_| "INTERNAL_ERROR")?
            .as_mut()
            .map_or_else(Vec::new, |source| source.take_kernel_threads());
        for handle in handles {
            handle.join().map_err(|_| "OWNER_PANIC")?;
        }
        Ok(())
    }
}
fn dispatch(
    request: RequestEnvelope,
    state: &Arc<Shared>,
    kernel: &Worker,
    editor: &Worker,
    auxiliary: &Worker,
) {
    let operation = request
        .operation_id
        .0
        .clone()
        .unwrap_or_else(|| request.request_ref.clone());
    if state
        .operations
        .token(&operation)
        .is_none_or(|t| t.load(Ordering::Acquire))
    {
        let status = state.operations.finish(&operation, Err("CANCELLED".into()));
        if let Some(status) = status {
            state.events.emit(
                EventEnvelopeEventKind::OperationFinished,
                Some(request.request_ref),
                Some(operation),
                serde_json::to_value(status).unwrap_or(Value::Null),
            );
        }
        return;
    }
    let immediate = match &request.body {
        HostRequestBody::ReadHostState(_) => Some(Ok(
            json!({"runtime_instance_id":state.runtime,"document":null,"document_storage_ready":false,"operations":state.operations.summaries(),"operation_history_complete":false}),
        )),
        HostRequestBody::ReadOperationStatus(body) => Some(
            state
                .operations
                .status(&body.operation_ref)
                .ok_or_else(|| "NOT_AVAILABLE".into())
                .and_then(|s| serde_json::to_value(s).map_err(|_| "INTERNAL_ERROR".into())),
        ),
        HostRequestBody::AcknowledgeIO(_) => Some(Err("NOT_AVAILABLE".into())),
        _ => None,
    };
    if let Some(result) = immediate {
        state.operations.start(&operation);
        if let Some(status) = state.operations.finish(&operation, result) {
            state.events.emit(
                EventEnvelopeEventKind::OperationFinished,
                Some(request.request_ref),
                Some(operation),
                serde_json::to_value(status).unwrap_or(Value::Null),
            );
        }
        return;
    }
    let worker = match &request.body {
        HostRequestBody::AnalyzeEditor(_) => editor,
        HostRequestBody::EvaluateScratch(_) => auxiliary,
        _ => kernel,
    };
    // Registry token exists before worker admission; future jobs never clear it.
    let Some(token) = state.operations.token(&operation) else {
        return;
    };
    if worker
        .sender
        .try_send(Job {
            request: request.clone(),
            token,
        })
        .is_err()
        && let Some(status) = state
            .operations
            .finish(&operation, Err("BUDGET_EXCEEDED".into()))
    {
        state.events.emit(
            EventEnvelopeEventKind::OperationFinished,
            Some(request.request_ref),
            Some(operation),
            serde_json::to_value(status).unwrap_or(Value::Null),
        );
    }
}

impl Drop for NativeHost {
    fn drop(&mut self) {
        self.close_begin();
        if let Ok(owner) = self.owner.get_mut()
            && let Some(owner) = owner.take()
        {
            // A forgotten explicit finish still shuts down; destruction never joins on a UI thread.
            let _ = thread::Builder::new()
                .name("openmath-host-reaper".into())
                .spawn(move || {
                    let _ = owner.join();
                });
        }
    }
}
