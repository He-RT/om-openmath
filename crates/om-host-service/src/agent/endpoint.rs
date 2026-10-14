//! Trusted document-owned native control port. No transcript can recreate this authority.
mod effect;
mod readonly;
use super::{framing, ingress::*};
use crate::protocol::{Nullable, generated::*, validation};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, sync::OnceLock};

/// Strict closed native command shape; complete peer frame bytes are preserved inside the string.
pub fn decode_command(bytes: &[u8]) -> Result<NativeAgentHostCommand, String> {
    static ROOT: OnceLock<Value> = OnceLock::new();
    let root = ROOT.get_or_init(|| {
        serde_json::from_str(include_str!(
            "../../../../docs/design/native-agent.schema.json"
        ))
        .expect("checked agent schema")
    });
    let value = framing::strict_json_bounded(bytes, crate::protocol::MAX_CONTROL_BYTES)
        .map_err(str::to_owned)?;
    validation::validate(&value, &root["$defs"]["NativeAgentHostCommand"], root)?;
    serde_json::from_value(value).map_err(|_| "INVALID_AGENT_COMMAND".into())
}
/// Authority lives with the actual source owner; replies describe intents, never completed effects.
pub struct AgentEndpoint {
    owner: OwnerScope,
    scope: NativeAgentControlScope,
    ingress: AgentIngress,
    outgoing_sequence: u64,
    replies: BTreeMap<String, NativeAgentHostReply>,
    sent: BTreeMap<String, (String, NativeAgentHostReply)>,
    last_receipt: Option<(String, String, String, u64, String)>,
    readonly: BTreeMap<String, readonly::ReadonlyEntry>,
    effects: BTreeMap<String, effect::EffectEntry>,
}
impl AgentEndpoint {
    /// Actual runtime/document facts are supplied by SourceEndpoint, not a helper frame.
    pub fn activate(
        command: NativeAgentActivate,
        runtime: String,
        binding: DocumentBinding,
    ) -> Result<(Self, NativeAgentHostReply), String> {
        if command.scope.document_generation != binding.generation {
            return Err("STALE_AGENT_SCOPE".into());
        }
        let s = &command.scope;
        let owner = OwnerScope {
            runtime,
            session: s.agent_session_id.clone(),
            document: binding.document_id,
            document_generation: binding.generation,
            task: s.task_id.clone(),
            task_generation: s.task_generation,
            connection: s.connection_id.clone(),
            connection_generation: s.connection_generation,
        };
        let ingress = AgentIngress::new(
            owner.clone(),
            command.grants,
            command.readonly,
            command.hello_id.clone(),
            command.core_version.clone(),
            command.adapter_version.clone(),
        )
        .map_err(str::to_owned)?;
        let mut endpoint = Self {
            owner,
            scope: command.scope,
            ingress,
            outgoing_sequence: 0,
            replies: BTreeMap::new(),
            last_receipt: None,
            sent: BTreeMap::new(),
            readonly: BTreeMap::new(),
            effects: BTreeMap::new(),
        };
        let frame = endpoint.outbound("hello", &command.hello_id, None, json!({"core_version":command.core_version,"adapter_version":command.adapter_version,"schema_hash":format!("{:x}",Sha256::digest(include_bytes!("../../../../docs/design/native-agent.schema.json")))}))?;
        let mut reply = endpoint.reply(NativeAgentHostReplyDisposition::Hello);
        reply.outbound_json = Nullable(Some(frame));
        Ok((endpoint, reply))
    }
    /// An active task can only be replaced after explicit native close. Generations never regress.
    pub fn can_replace(&self, scope: &NativeAgentControlScope) -> bool {
        self.ingress.phase() == ConnectionPhase::Closed
            && scope.task_generation.get() > self.scope.task_generation.get()
            && scope.connection_generation.get() > self.scope.connection_generation.get()
    }
    fn check(
        &self,
        scope: &NativeAgentControlScope,
        binding: &DocumentBinding,
    ) -> Result<(), String> {
        if self.owner.document != binding.document_id
            || self.owner.document_generation != binding.generation
            || serde_json::to_value(scope).ok() != serde_json::to_value(&self.scope).ok()
        {
            return Err("STALE_AGENT_SCOPE".into());
        }
        Ok(())
    }
    fn reply(&self, disposition: NativeAgentHostReplyDisposition) -> NativeAgentHostReply {
        let phase = match self.ingress.phase() {
            ConnectionPhase::Starting => NativeAgentHostReplyPhase::Starting,
            ConnectionPhase::Ready => NativeAgentHostReplyPhase::Ready,
            ConnectionPhase::Cancelling => NativeAgentHostReplyPhase::Cancelling,
            ConnectionPhase::AwaitingSettlement => NativeAgentHostReplyPhase::AwaitingSettlement,
            ConnectionPhase::Failed => NativeAgentHostReplyPhase::Failed,
            ConnectionPhase::Closed => NativeAgentHostReplyPhase::Closed,
        };
        NativeAgentHostReply {
            protocol_version: 1,
            runtime_instance_id: self.owner.runtime.clone(),
            document_id: self.owner.document.clone(),
            document_generation: self.owner.document_generation,
            scope: self.scope.clone(),
            phase,
            disposition,
            outbound_json: Nullable(None),
            canonical_json: Nullable(None),
            peer_message_id: Nullable(None),
            event_id: Nullable(None),
            source_sequence: Nullable(None),
            tool_intent: Nullable(None),
            request_intent: Nullable(None),
            readonly_result: Nullable(None),
            effect_result: Nullable(None),
        }
    }
    fn outbound(
        &mut self,
        kind: &str,
        id: &str,
        reply_to: Option<&str>,
        payload: Value,
    ) -> Result<String, String> {
        let sequence = self
            .outgoing_sequence
            .checked_add(1)
            .filter(|s| *s <= crate::protocol::MAX_SERIAL)
            .ok_or("IPC_TASK_BUDGET")?;
        let s = &self.owner;
        let frame = json!({"protocol_version":1,"connection_id":s.connection,"connection_generation":s.connection_generation,"runtime_instance_id":s.runtime,"agent_session_id":s.session,"document_id":s.document,"document_generation":s.document_generation,"task_id":s.task,"task_generation":s.task_generation,"message_id":id,"sequence":sequence,"reply_to":reply_to,"type":kind,"payload":payload});
        let bytes = serde_json::to_vec(&frame).map_err(|_| "IPC_INVALID_SHAPE")?;
        framing::decode(&bytes).map_err(str::to_owned)?;
        self.outgoing_sequence = sequence;
        String::from_utf8(bytes).map_err(|_| "IPC_INVALID_UTF8".into())
    }
    /// Short synchronous owner steps. Disk/network/process waits happen outside this port.
    pub fn command(
        &mut self,
        command: NativeAgentHostCommand,
        binding: &DocumentBinding,
        read_scope: &crate::references::ReferenceScope,
    ) -> Result<NativeAgentHostReply, String> {
        let scope = match &command {
            NativeAgentHostCommand::NativeAgentActivate(_) => {
                return Err("AGENT_TASK_ACTIVE".into());
            }
            NativeAgentHostCommand::NativeAgentReceive(c) => &c.scope,
            NativeAgentHostCommand::NativeAgentConfirm(c) => &c.scope,
            NativeAgentHostCommand::NativeAgentSend(c) => &c.scope,
            NativeAgentHostCommand::NativeAgentCancel(c) => &c.scope,
            NativeAgentHostCommand::NativeAgentClose(c) => &c.scope,
            NativeAgentHostCommand::NativeAgentStatus(c) => &c.scope,
            NativeAgentHostCommand::NativeAgentReadTool(c) => &c.scope,
            NativeAgentHostCommand::NativeAgentConfirmTool(c) => &c.scope,
            NativeAgentHostCommand::NativeAgentBindHistory(c) => &c.scope,
            NativeAgentHostCommand::NativeAgentEffectTool(c) => &c.scope,
        };
        self.check(scope, binding)?;
        match command {
            NativeAgentHostCommand::NativeAgentReceive(c) => {
                let disposition = self
                    .ingress
                    .accept(c.frame_json.as_bytes(), &self.owner)
                    .map_err(str::to_owned)?;
                let value = framing::strict_json(c.frame_json.as_bytes()).map_err(str::to_owned)?;
                let id = value["message_id"]
                    .as_str()
                    .ok_or("IPC_INVALID_SHAPE")?
                    .to_owned();
                if matches!(disposition, IngressDisposition::Duplicate) {
                    let mut reply = self
                        .replies
                        .get(&id)
                        .cloned()
                        .ok_or("IPC_REPLAY_UNAVAILABLE")?;
                    reply.phase = self.reply(NativeAgentHostReplyDisposition::Duplicate).phase;
                    // Pending transcript is returned again for real unknown-commit reconciliation.
                    if reply.disposition != NativeAgentHostReplyDisposition::Transcript {
                        reply.disposition = NativeAgentHostReplyDisposition::Duplicate;
                    }
                    return Ok(reply);
                }
                let mut reply = self.reply(NativeAgentHostReplyDisposition::Data);
                reply.peer_message_id = Nullable(Some(id.clone()));
                match disposition {
                    IngressDisposition::Ready => {
                        reply.disposition = NativeAgentHostReplyDisposition::Ready
                    }
                    IngressDisposition::Transcript(event) => {
                        reply.disposition = NativeAgentHostReplyDisposition::Transcript;
                        reply.canonical_json = Nullable(Some(
                            serde_json::to_string(&event.payload.message)
                                .map_err(|_| "IPC_INVALID_TRANSCRIPT")?,
                        ));
                        reply.event_id = Nullable(Some(event.payload.event_id));
                        reply.source_sequence = Nullable(Some(event.payload.source_sequence));
                    }
                    IngressDisposition::ToolIntent(i) => {
                        reply.disposition = NativeAgentHostReplyDisposition::ToolIntent;
                        reply.tool_intent = Nullable(Some(NativeAgentToolIntent {
                            operation_id: i.operation_id,
                            agent_turn_id: i.turn_id,
                            tool_call_id: i.call_id,
                            proposal_event_ref: i.proposal_event,
                            name: i.name,
                            arguments_hash: i.arguments_hash,
                        }));
                    }
                    IngressDisposition::RequestIntent(i) => {
                        reply.disposition = NativeAgentHostReplyDisposition::RequestIntent;
                        reply.request_intent = Nullable(Some(NativeAgentRequestIntent {
                            operation_id: i.operation_id,
                            agent_turn_id: i.turn_id,
                            request_id: i.request_id,
                        }));
                    }
                    IngressDisposition::AwaitingSettlement | IngressDisposition::Data(_) => (),
                    IngressDisposition::Duplicate => unreachable!(),
                }
                self.replies.insert(id, reply.clone());
                Ok(reply)
            }
            NativeAgentHostCommand::NativeAgentConfirm(c) => {
                let claim = (
                    c.message_id.clone(),
                    c.event_id.clone(),
                    c.canonical_hash.clone(),
                    c.event_sequence.get(),
                    c.receipt_hash.clone(),
                );
                if self.last_receipt.as_ref() == Some(&claim) {
                    return self
                        .replies
                        .get(&c.message_id)
                        .cloned()
                        .ok_or_else(|| "IPC_REPLAY_UNAVAILABLE".into());
                }
                if c.event_sequence.get() == 0 {
                    return Err("IPC_LEDGER_RECEIPT_MISMATCH".into());
                }
                self.ingress
                    .confirm_transcript(&c.message_id, &c.event_id, &c.canonical_hash)
                    .map_err(str::to_owned)?;
                let ack_id = identity().map_err(str::to_owned)?;
                let frame=self.outbound("ack",&ack_id,Some(&c.message_id),json!({"event_id":c.event_id,"event_sequence":c.event_sequence,"receipt_hash":c.receipt_hash}))?;
                let mut reply = self.reply(NativeAgentHostReplyDisposition::Ack);
                reply.outbound_json = Nullable(Some(frame));
                reply.peer_message_id = Nullable(Some(c.message_id.clone()));
                reply.event_id = Nullable(Some(c.event_id));
                self.replies.insert(c.message_id, reply.clone());
                self.last_receipt = Some(claim);
                Ok(reply)
            }
            NativeAgentHostCommand::NativeAgentSend(c) => {
                if self.ingress.phase() != ConnectionPhase::Ready {
                    return Err("IPC_TASK_NOT_READY".into());
                }
                let frame = framing::decode(c.frame_json.as_bytes()).map_err(str::to_owned)?;
                let value = serde_json::to_value(&frame).map_err(|_| "IPC_INVALID_SHAPE")?;
                let id = value["message_id"]
                    .as_str()
                    .ok_or("IPC_INVALID_SHAPE")?
                    .to_owned();
                let digest = format!("{:x}", Sha256::digest(c.frame_json.as_bytes()));
                if let Some((original, reply)) = self.sent.get(&id) {
                    if original != &digest {
                        return Err("IPC_ID_CONFLICT".into());
                    }
                    if let AgentIPCFrame::AgentToolResultFrame(f) = &frame {
                        self.check_tool_result(&f.payload, read_scope)
                            .map_err(str::to_owned)?;
                    }
                    return Ok(reply.clone());
                }
                if self.sent.len() >= 4096 {
                    return Err("IPC_TASK_BUDGET".into());
                }
                let supplied = value["sequence"].as_u64().ok_or("IPC_INVALID_SHAPE")?;
                let mut expected = framing::strict_json(
                    self.outbound(
                        value["type"].as_str().ok_or("IPC_INVALID_SHAPE")?,
                        value["message_id"].as_str().ok_or("IPC_INVALID_SHAPE")?,
                        value["reply_to"].as_str(),
                        value["payload"].clone(),
                    )?
                    .as_bytes(),
                )
                .map_err(str::to_owned)?;
                // Compare full scope, not just IDs and sequence. Native old callbacks cannot send on a new task.
                expected["sequence"] = json!(supplied);
                if expected != value || supplied != self.outgoing_sequence {
                    self.outgoing_sequence -= 1;
                    return Err("STALE_AGENT_SCOPE".into());
                }
                let accepted = match &frame {
                    AgentIPCFrame::AgentUserInputFrame(f) => self.user_message(&f.payload.message),
                    AgentIPCFrame::AgentSteerFrame(f) => self.user_message(&f.payload.message),
                    AgentIPCFrame::AgentFollowUpFrame(f) => self.user_message(&f.payload.message),
                    AgentIPCFrame::AgentModelRequestFrame(f) => {
                        self.ingress.bind_request(&f.payload)
                    }
                    AgentIPCFrame::AgentModelEventFrame(f) => self.ingress.model_event(&f.payload),
                    AgentIPCFrame::AgentToolResultFrame(f) => {
                        self.check_tool_result(&f.payload, read_scope)
                    }
                    AgentIPCFrame::AgentErrorFrame(_) => Ok(()),
                    _ => Err("IPC_WRONG_DIRECTION"),
                };
                if let Err(code) = accepted {
                    self.outgoing_sequence -= 1;
                    return Err(code.into());
                }
                let mut reply = self.reply(NativeAgentHostReplyDisposition::Sent);
                reply.outbound_json = Nullable(Some(c.frame_json));
                self.sent.insert(id, (digest, reply.clone()));
                Ok(reply)
            }
            NativeAgentHostCommand::NativeAgentCancel(c) => {
                self.ingress.cancel();
                self.readonly.clear();
                self.effects.clear();
                let id = identity().map_err(str::to_owned)?;
                let frame = self.outbound("cancel", &id, None, json!({"reason":c.reason}))?;
                let mut reply = self.reply(NativeAgentHostReplyDisposition::Cancel);
                reply.outbound_json = Nullable(Some(frame));
                Ok(reply)
            }
            NativeAgentHostCommand::NativeAgentClose(_) => {
                self.close();
                Ok(self.reply(NativeAgentHostReplyDisposition::Closed))
            }
            NativeAgentHostCommand::NativeAgentStatus(_) => {
                Ok(self.reply(NativeAgentHostReplyDisposition::Status))
            }
            NativeAgentHostCommand::NativeAgentConfirmTool(c) => {
                if self.effects.contains_key(&c.operation_id) {
                    self.confirm_effect(c, read_scope)
                } else {
                    self.confirm_readonly(c, read_scope)
                }
            }
            NativeAgentHostCommand::NativeAgentReadTool(_) => {
                Err("READONLY_PRODUCER_REQUIRED".into())
            }
            NativeAgentHostCommand::NativeAgentBindHistory(_) => {
                Err("SOURCE_PRODUCER_REQUIRED".into())
            }
            NativeAgentHostCommand::NativeAgentEffectTool(_) => {
                Err("EFFECT_PRODUCER_REQUIRED".into())
            }
            NativeAgentHostCommand::NativeAgentActivate(_) => unreachable!(),
        }
    }
    fn user_message(&self, message: &AgentCanonicalMessage) -> Result<(), &'static str> {
        if message.role != AgentCanonicalMessageRole::User
            || message.task_id.0.as_ref() != Some(&self.owner.task)
            || message.parts.iter().any(|p| {
                matches!(
                    p,
                    AgentMessagePart::AgentToolProposalPart(_)
                        | AgentMessagePart::AgentToolResultPart(_)
                )
            })
        {
            return Err("IPC_INVALID_USER_INPUT");
        }
        Ok(())
    }
    /// Source rebind/close revokes ephemeral grants before physical owner joins.
    pub fn close(&mut self) {
        self.ingress.close();
        self.readonly.clear();
        self.effects.clear();
        self.replies.clear();
        self.sent.clear();
        self.last_receipt = None;
    }
}
