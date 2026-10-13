//! Connection state is ephemeral authority. Persisted transcript data cannot recreate grants.
use super::framing;
use crate::protocol::{Serial, generated::*};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

/// Trusted native application facts, never constructed from a helper payload.
#[derive(Clone, PartialEq, Eq)]
pub struct OwnerScope {
    /// Current native runtime.
    pub runtime: String,
    /// Explicit canonical session activation.
    pub session: String,
    /// Human-selected open document.
    pub document: String,
    /// Trusted document lifetime generation.
    pub document_generation: Serial,
    /// Host task identity.
    pub task: String,
    /// Host task activation generation.
    pub task_generation: Serial,
    /// Private helper connection identity.
    pub connection: String,
    /// Actual private process/pipe generation.
    pub connection_generation: Serial,
}
/// Shape/replay status only. A helper cannot establish mathematical or durable task completion.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConnectionPhase {
    /// Waiting for the exact reply to the host handshake.
    Starting,
    /// Private peer handshake verified.
    Ready,
    /// Host cancellation is authoritative and prevents new intents.
    Cancelling,
    /// Loop reports stopped; host effects still need independent settlement.
    AwaitingSettlement,
    /// This connection needs a new handshake.
    Failed,
    /// Host has closed this connection.
    Closed,
}
/// Identity mapping is a pending native intent, never an execution/completion receipt.
#[derive(Clone, Debug)]
pub struct ToolIntent {
    /// Host entropy-derived ID, distinct from all provider/tool IDs.
    pub operation_id: String,
    /// Original provider-decoded turn ID within this trusted task.
    pub turn_id: String,
    /// Canonical call identity.
    pub call_id: String,
    /// Original durable proposal event.
    pub proposal_event: String,
    /// Actual canonical tool name.
    pub name: AgentToolName,
    /// Complete original data-only argument bytes; validated again by the actual tool handler.
    pub arguments_hash: String,
}
/// Provider request identity mapped to one host pending intent, never a network receipt.
#[derive(Clone, Debug)]
pub struct RequestIntent {
    /// Entropy-derived native intent identity.
    pub operation_id: String,
    /// Original turn within the active task.
    pub turn_id: String,
    /// Original helper request identity.
    pub request_id: String,
}
/// An accepted frame has no automatic evaluator/model dispatch side effect.
pub enum IngressDisposition {
    /// Peer version/connection handshake is ready.
    Ready,
    /// Exact replay of an original byte-identical frame.
    Duplicate,
    /// Must persist the canonical event and confirm its receipt before proceeding.
    Transcript(Box<AgentTranscriptEventFrame>),
    /// A native identity is available; actual tool admission is a separate step.
    ToolIntent(ToolIntent),
    /// Other known data frames remain typed for the next service layer.
    Data(AgentIPCFrame),
    /// A model request is correlated to its original native pending intent.
    RequestIntent(RequestIntent),
    /// Loop termination report awaiting native effect reconciliation.
    AwaitingSettlement,
}
struct RequestState {
    intent: RequestIntent,
    refs: Option<(String, String)>,
    requested: bool,
    offset: u64,
    terminal: bool,
}
struct Seen {
    hash: [u8; 32],
    sequence: u64,
}
struct Proposal {
    event: String,
    turn: String,
    call: AgentToolProposalPart,
}
/// One current explicit task activation; never serializable as recovered authorization.
pub struct AgentIngress {
    scope: OwnerScope,
    grants: Vec<AgentToolName>,
    readonly: bool,
    hello_id: String,
    core_version: String,
    adapter_version: String,
    schema_hash: String,
    phase: ConnectionPhase,
    sequence: u64,
    bytes: usize,
    seen: BTreeMap<String, Seen>,
    awaiting_transcript: Option<AgentTranscriptEventFrame>,
    source_sequence: u64,
    proposals: BTreeMap<String, Proposal>,
    mapped: BTreeMap<String, ToolIntent>,
    requests: BTreeMap<String, RequestState>,
}
fn hash(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}
fn hash_text(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub(super) fn identity() -> Result<String, &'static str> {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).map_err(|_| "HOST_ENTROPY_FAILED")?;
    Ok(format!(
        "agent-operation-{}",
        bytes.iter().map(|b| format!("{b:02x}")).collect::<String>()
    ))
}
impl AgentIngress {
    /// The native caller supplies scope/grants and version pins from explicit activation.
    /// Transcript restoration must create a fresh activation and connection instead.
    pub fn new(
        scope: OwnerScope,
        grants: Vec<AgentToolName>,
        readonly: bool,
        hello_id: String,
        core_version: String,
        adapter_version: String,
    ) -> Result<Self, &'static str> {
        if grants.len() > 12
            || scope.connection_generation.get() == 0
            || scope.task_generation.get() == 0
        {
            return Err("INVALID_AGENT_ACTIVATION");
        }
        Ok(Self {
            scope,
            grants,
            readonly,
            hello_id,
            core_version,
            adapter_version,
            schema_hash: hash_text(include_bytes!(
                "../../../../docs/design/native-agent.schema.json"
            )),
            phase: ConnectionPhase::Starting,
            sequence: 0,
            bytes: 0,
            seen: BTreeMap::new(),
            awaiting_transcript: None,
            source_sequence: 0,
            proposals: BTreeMap::new(),
            mapped: BTreeMap::new(),
            requests: BTreeMap::new(),
        })
    }
    /// Current shape/replay state, not task success.
    pub fn phase(&self) -> ConnectionPhase {
        self.phase
    }
    fn scope_matches(&self, value: &Value) -> bool {
        let s = &self.scope;
        value["runtime_instance_id"] == s.runtime
            && value["agent_session_id"] == s.session
            && value["document_id"] == s.document
            && value["document_generation"].as_u64() == Some(s.document_generation.get())
            && value["task_id"] == s.task
            && value["task_generation"].as_u64() == Some(s.task_generation.get())
            && value["connection_id"] == s.connection
            && value["connection_generation"].as_u64() == Some(s.connection_generation.get())
    }
    /// Decode and bind a real raw frame. Compare current trusted native facts before replay.
    pub fn accept(
        &mut self,
        bytes: &[u8],
        current: &OwnerScope,
    ) -> Result<IngressDisposition, &'static str> {
        if current != &self.scope
            || matches!(
                self.phase,
                ConnectionPhase::Failed | ConnectionPhase::Closed
            )
        {
            return Err("STALE_AGENT_SCOPE");
        }
        let frame = framing::decode(bytes)?;
        let value = serde_json::to_value(&frame).map_err(|_| "IPC_INVALID_SHAPE")?;
        if !self.scope_matches(&value) {
            return Err("STALE_AGENT_SCOPE");
        }
        let id = value["message_id"].as_str().ok_or("IPC_INVALID_SHAPE")?;
        let sequence = value["sequence"].as_u64().ok_or("IPC_INVALID_SHAPE")?;
        let digest = hash(bytes);
        if let Some(previous) = self.seen.get(id) {
            return if previous.hash == digest && previous.sequence == sequence {
                Ok(IngressDisposition::Duplicate)
            } else {
                Err("IPC_ID_CONFLICT")
            };
        }
        if sequence != self.sequence + 1 {
            self.phase = ConnectionPhase::Failed;
            return Err("IPC_SEQUENCE_GAP");
        }
        if self.seen.len() >= 4096
            || self
                .bytes
                .checked_add(bytes.len())
                .is_none_or(|n| n > 8 * 1024 * 1024)
        {
            return Err("IPC_TASK_BUDGET");
        }
        if self.awaiting_transcript.is_some() {
            return Err("IPC_LEDGER_ACK_REQUIRED");
        }
        let result = match frame {
            AgentIPCFrame::AgentReadyFrame(ready) => {
                if self.phase != ConnectionPhase::Starting
                    || ready.reply_to.0.as_ref() != Some(&self.hello_id)
                    || ready.payload.core_version != self.core_version
                    || ready.payload.adapter_version != self.adapter_version
                    || ready.payload.schema_hash != self.schema_hash
                {
                    return Err("IPC_HANDSHAKE_MISMATCH");
                }
                self.phase = ConnectionPhase::Ready;
                IngressDisposition::Ready
            }
            AgentIPCFrame::AgentTranscriptEventFrame(event) => {
                if self.phase != ConnectionPhase::Ready
                    || event.payload.source_sequence.get() != self.source_sequence + 1
                {
                    return Err("IPC_TRANSCRIPT_SEQUENCE");
                }
                let message = &event.payload.message;
                if message.role != AgentCanonicalMessageRole::Assistant
                    || message.task_id.0.as_ref() != Some(&self.scope.task)
                    || message.agent_turn_id.0.is_none()
                {
                    return Err("IPC_INVALID_TRANSCRIPT");
                }
                let mut parts = BTreeSet::new();
                let mut calls = BTreeSet::new();
                for part in &message.parts {
                    let p = serde_json::to_value(part).map_err(|_| "IPC_INVALID_TRANSCRIPT")?;
                    if !parts.insert(
                        p["part_id"]
                            .as_str()
                            .ok_or("IPC_INVALID_TRANSCRIPT")?
                            .to_owned(),
                    ) {
                        return Err("IPC_PART_ID_CONFLICT");
                    }
                    if let AgentMessagePart::AgentToolProposalPart(call) = part {
                        if calls.len() >= 32
                            || !calls.insert(call.tool_call_id.clone())
                            || self.proposals.contains_key(&call.tool_call_id)
                        {
                            return Err("IPC_CALL_ID_CONFLICT");
                        }
                        let arguments = framing::strict_json(call.arguments_json.as_bytes())?;
                        if !arguments.is_object() {
                            return Err("IPC_INVALID_TOOL_ARGUMENTS");
                        }
                    } else if matches!(part, AgentMessagePart::AgentToolResultPart(_)) {
                        return Err("IPC_FORGED_TOOL_RESULT");
                    }
                }
                if self.proposals.len() + calls.len() > 32 {
                    return Err("IPC_TASK_BUDGET");
                }
                self.awaiting_transcript = Some((*event).clone());
                IngressDisposition::Transcript(event)
            }
            AgentIPCFrame::AgentToolCallFrame(call) => {
                if self.phase != ConnectionPhase::Ready {
                    return Err("IPC_TASK_NOT_READY");
                }
                let proposal = self
                    .proposals
                    .get(&call.payload.tool_call_id)
                    .ok_or("IPC_PROPOSAL_NOT_ACKED")?;
                if proposal.event != call.payload.proposal_event_ref
                    || proposal.turn != call.payload.agent_turn_id
                {
                    return Err("IPC_CALL_SCOPE_MISMATCH");
                }
                let name = &proposal.call.name;
                if !self.grants.contains(name)
                    || self.readonly
                        && matches!(
                            name,
                            AgentToolName::ApplyNotebookPatch
                                | AgentToolName::RunCells
                                | AgentToolName::UndoTransaction
                        )
                {
                    return Err("PERMISSION_DENIED");
                }
                let intent = if let Some(original) = self.mapped.get(&call.payload.tool_call_id) {
                    original.clone()
                } else {
                    let mapped = ToolIntent {
                        operation_id: identity()?,
                        turn_id: proposal.turn.clone(),
                        call_id: proposal.call.tool_call_id.clone(),
                        proposal_event: proposal.event.clone(),
                        name: name.clone(),
                        arguments_hash: hash_text(proposal.call.arguments_json.as_bytes()),
                    };
                    self.mapped.insert(mapped.call_id.clone(), mapped.clone());
                    mapped
                };
                IngressDisposition::ToolIntent(intent)
            }
            AgentIPCFrame::AgentSettledFrame(_) => {
                if self.phase == ConnectionPhase::Starting {
                    return Err("IPC_TASK_NOT_READY");
                }
                self.phase = ConnectionPhase::AwaitingSettlement;
                IngressDisposition::AwaitingSettlement
            }
            AgentIPCFrame::AgentPrepareRequestFrame(request) => {
                if self.phase != ConnectionPhase::Ready {
                    return Err("IPC_TASK_NOT_READY");
                }
                let p = request.payload;
                if let Some(state) = self.requests.get(&p.request_id) {
                    let original = &state.intent;
                    if original.turn_id != p.agent_turn_id {
                        return Err("IPC_REQUEST_SCOPE_MISMATCH");
                    }
                    IngressDisposition::RequestIntent(original.clone())
                } else {
                    if self.requests.len() >= 64 {
                        return Err("IPC_TASK_BUDGET");
                    }
                    let intent = RequestIntent {
                        operation_id: identity()?,
                        turn_id: p.agent_turn_id,
                        request_id: p.request_id,
                    };
                    self.requests.insert(
                        intent.request_id.clone(),
                        RequestState {
                            intent: intent.clone(),
                            refs: None,
                            requested: false,
                            offset: 0,
                            terminal: false,
                        },
                    );
                    IngressDisposition::RequestIntent(intent)
                }
            }
            AgentIPCFrame::AgentModelRequestFrame(request) => {
                if self.phase != ConnectionPhase::Ready {
                    return Err("IPC_TASK_NOT_READY");
                }
                let p = request.payload;
                let state = self
                    .requests
                    .get_mut(&p.request_id)
                    .ok_or("IPC_REQUEST_NOT_PREPARED")?;
                if state.intent.turn_id != p.agent_turn_id {
                    return Err("IPC_REQUEST_SCOPE_MISMATCH");
                }
                if state.terminal {
                    return Err("IPC_REQUEST_FINISHED");
                }
                let supplied = (p.context_ref, p.model_config_ref);
                if state.refs.as_ref() != Some(&supplied) {
                    return Err("IPC_REQUEST_NOT_BOUND");
                }
                state.requested = true;
                IngressDisposition::RequestIntent(state.intent.clone())
            }
            AgentIPCFrame::AgentErrorFrame(_) => {
                if self.phase != ConnectionPhase::Ready {
                    return Err("IPC_TASK_NOT_READY");
                }
                IngressDisposition::Data(frame)
            }
            _ => return Err("IPC_WRONG_DIRECTION"),
        };
        self.seen.insert(
            id.to_owned(),
            Seen {
                hash: digest,
                sequence,
            },
        );
        self.sequence = sequence;
        self.bytes += bytes.len();
        Ok(result)
    }
    /// Only the native context/model preparer binds references; a helper cannot choose them.
    pub fn bind_request(&mut self, payload: &AgentModelRequestPayload) -> Result<(), &'static str> {
        let state = self
            .requests
            .get_mut(&payload.request_id)
            .ok_or("IPC_REQUEST_NOT_PREPARED")?;
        if state.intent.turn_id != payload.agent_turn_id || state.terminal {
            return Err("IPC_REQUEST_SCOPE_MISMATCH");
        }
        let refs = (
            payload.context_ref.clone(),
            payload.model_config_ref.clone(),
        );
        if state
            .refs
            .as_ref()
            .is_some_and(|previous| previous != &refs)
        {
            return Err("IPC_REQUEST_ID_CONFLICT");
        }
        state.refs = Some(refs);
        Ok(())
    }
    /// Actual native stream direction, correlated bytes and terminal canonical model message.
    pub fn model_event(&mut self, payload: &AgentModelEventPayload) -> Result<(), &'static str> {
        let state = self
            .requests
            .get_mut(&payload.request_id)
            .ok_or("IPC_REQUEST_NOT_PREPARED")?;
        if !state.requested
            || state.terminal
            || state.intent.turn_id != payload.agent_turn_id
            || payload.byte_offset.get() != state.offset
        {
            return Err("IPC_MODEL_STREAM_MISMATCH");
        }
        let finished = payload.phase == AgentModelEventPayloadPhase::Finished;
        if finished {
            let message = payload
                .canonical_message
                .0
                .as_ref()
                .ok_or("IPC_INVALID_TRANSCRIPT")?;
            if message.role != AgentCanonicalMessageRole::Assistant
                || message.task_id.0.as_ref() != Some(&self.scope.task)
                || message.agent_turn_id.0.as_ref() != Some(&state.intent.turn_id)
                || message
                    .parts
                    .iter()
                    .any(|p| matches!(p, AgentMessagePart::AgentToolResultPart(_)))
            {
                return Err("IPC_INVALID_TRANSCRIPT");
            }
        } else if payload.canonical_message.0.is_some() {
            return Err("IPC_INVALID_TRANSCRIPT");
        }
        state.offset = state
            .offset
            .checked_add(payload.text.len() as u64)
            .filter(|n| *n <= crate::protocol::MAX_SERIAL)
            .ok_or("IPC_TASK_BUDGET")?;
        state.terminal = payload.phase != AgentModelEventPayloadPhase::Delta;
        Ok(())
    }
    /// Missing native handlers produce unavailable only; this is not an admitted operation/effect.
    pub fn unavailable_tool(&self, payload: &AgentToolResultPayload) -> Result<(), &'static str> {
        let intent = self
            .mapped
            .get(&payload.tool_call_id)
            .ok_or("IPC_PROPOSAL_NOT_ACKED")?;
        if intent.turn_id != payload.agent_turn_id
            || payload.operation_ref.0.is_some()
            || payload.result_ref.0.is_some()
            || payload.outcome != AgentToolResultPayloadOutcome::Unavailable
        {
            return Err("IPC_TOOL_RECEIPT_REQUIRED");
        }
        Ok(())
    }
    /// Trusted native ledger readback, not a peer ACK. Only after a matching durable event
    /// can the helper's canonical tool proposals become eligible for intent-ID mapping.
    pub fn confirm_transcript(
        &mut self,
        message_id: &str,
        event_id: &str,
        canonical_hash: &str,
    ) -> Result<(), &'static str> {
        let pending = self
            .awaiting_transcript
            .as_ref()
            .ok_or("IPC_NO_PENDING_TRANSCRIPT")?;
        let bytes =
            serde_json::to_vec(&pending.payload.message).map_err(|_| "IPC_INVALID_TRANSCRIPT")?;
        if pending.message_id != message_id
            || pending.payload.event_id != event_id
            || hash_text(&bytes) != canonical_hash
        {
            return Err("IPC_LEDGER_RECEIPT_MISMATCH");
        }
        let event = self
            .awaiting_transcript
            .take()
            .ok_or("IPC_NO_PENDING_TRANSCRIPT")?;
        let turn = event
            .payload
            .message
            .agent_turn_id
            .0
            .ok_or("IPC_INVALID_TRANSCRIPT")?;
        for part in event.payload.message.parts {
            if let AgentMessagePart::AgentToolProposalPart(call) = part {
                self.proposals.insert(
                    call.tool_call_id.clone(),
                    Proposal {
                        event: event.payload.event_id.clone(),
                        turn: turn.clone(),
                        call,
                    },
                );
            }
        }
        self.source_sequence = event.payload.source_sequence.get();
        Ok(())
    }
    /// Direct native cancellation closes admission before any helper checkpoint is processed.
    pub fn cancel(&mut self) {
        if matches!(
            self.phase,
            ConnectionPhase::Starting | ConnectionPhase::Ready
        ) {
            self.phase = ConnectionPhase::Cancelling;
        }
    }
    /// Document/application close drops ephemeral grants and pending call mappings.
    pub fn close(&mut self) {
        self.phase = ConnectionPhase::Closed;
        self.awaiting_transcript = None;
        self.proposals.clear();
        self.mapped.clear();
        self.requests.clear();
        self.grants.clear();
    }
}
