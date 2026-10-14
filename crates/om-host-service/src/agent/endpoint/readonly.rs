//! Native-produced readonly results. A peer frame cannot supply arguments, data or a receipt.
use super::*;
use crate::references::ReferenceScope;

pub(super) struct ReadonlyEntry {
    scope: ReferenceScope,
    result: NativeAgentReadonlyResult,
    receipt: Option<(String, u64, String)>,
    issued: std::time::Instant,
}
impl AgentEndpoint {
    /// Host-only source/history callbacks still require this exact live activation.
    pub fn check_native_scope(
        &self,
        scope: &NativeAgentControlScope,
        binding: &DocumentBinding,
    ) -> Result<(), String> {
        self.check(scope, binding)?;
        if self.ingress.phase() != ConnectionPhase::Ready {
            return Err("IPC_TASK_NOT_READY".into());
        }
        Ok(())
    }
    /// Work is queued on the independent immutable-result owner; poll the same request identity.
    pub fn pending_readonly(&self) -> NativeAgentHostReply {
        self.reply(NativeAgentHostReplyDisposition::Pending)
    }
    /// Derive task scope solely from the actual activation and current source owner facts.
    pub fn readonly_scope(&self, current: ReferenceScope) -> ReferenceScope {
        self.ingress.read_scope(current)
    }
    /// Resolve only an already acknowledged proposal; no serialized command supplies raw args.
    pub fn readonly_input(
        &self,
        command: &NativeAgentReadTool,
        binding: &DocumentBinding,
        current: &ReferenceScope,
    ) -> Result<(ToolIntent, String, Option<NativeAgentReadonlyResult>), String> {
        self.check(&command.scope, binding)?;
        let (intent, args) = self
            .ingress
            .original_tool(&command.operation_id)
            .map_err(str::to_owned)?;
        if !matches!(
            intent.name,
            AgentToolName::ReadNotebook
                | AgentToolName::SearchFunctions
                | AgentToolName::GetFunctionDocs
        ) {
            return Err("NATIVE_TOOL_UNAVAILABLE".into());
        }
        let cached = if let Some(entry) = self.readonly.get(&intent.operation_id) {
            if entry.scope != *current || entry.issued.elapsed().as_secs() >= 600 {
                return Err("STALE_SNAPSHOT".into());
            }
            Some(entry.result.clone())
        } else {
            None
        };
        Ok((intent, args, cached))
    }
    /// Register actual data after the source-owned producer has completed. IDs remain immutable.
    pub fn register_readonly(
        &mut self,
        intent: &ToolIntent,
        current: &ReferenceScope,
        result_json: String,
        success: bool,
    ) -> Result<NativeAgentHostReply, String> {
        let (actual, _) = self
            .ingress
            .original_tool(&intent.operation_id)
            .map_err(str::to_owned)?;
        if actual.arguments_hash != intent.arguments_hash
            || actual.call_id != intent.call_id
            || result_json.len() > 262144
        {
            return Err("IPC_CALL_SCOPE_MISMATCH".into());
        }
        if self.readonly.contains_key(&intent.operation_id) || self.readonly.len() >= 32 {
            return Err("IPC_TASK_BUDGET".into());
        }
        let result = NativeAgentReadonlyResult {
            operation_id: intent.operation_id.clone(),
            agent_turn_id: intent.turn_id.clone(),
            tool_call_id: intent.call_id.clone(),
            name: intent.name.clone(),
            result_ref: identity().map_err(str::to_owned)?,
            result_hash: format!("{:x}", Sha256::digest(result_json.as_bytes())),
            result_json,
            outcome: if success {
                NativeAgentReadonlyResultOutcome::Completed
            } else {
                NativeAgentReadonlyResultOutcome::Failed
            },
        };
        self.readonly.insert(
            intent.operation_id.clone(),
            ReadonlyEntry {
                scope: current.clone(),
                result: result.clone(),
                receipt: None,
                issued: std::time::Instant::now(),
            },
        );
        Ok(self.readonly_reply(result))
    }
    /// Exact original-ID readback; this does not rerun the tool or widen source exposure.
    pub fn readonly_reply(&self, result: NativeAgentReadonlyResult) -> NativeAgentHostReply {
        let mut reply = self.reply(NativeAgentHostReplyDisposition::Data);
        reply.readonly_result = Nullable(Some(result));
        reply
    }
    pub(super) fn confirm_readonly(
        &mut self,
        c: NativeAgentConfirmTool,
        current: &ReferenceScope,
    ) -> Result<NativeAgentHostReply, String> {
        self.ingress
            .original_tool(&c.operation_id)
            .map_err(str::to_owned)?;
        let entry = self
            .readonly
            .get_mut(&c.operation_id)
            .ok_or("READONLY_PRODUCER_REQUIRED")?;
        if entry.scope != *current
            || entry.issued.elapsed().as_secs() >= 600
            || entry.result.result_ref != c.result_ref
            || entry.result.result_hash != c.result_hash
            || c.event_id != format!("readonly-{}", c.operation_id)
            || c.event_sequence.get() == 0
        {
            return Err("IPC_LEDGER_RECEIPT_MISMATCH".into());
        }
        let receipt = (c.event_id, c.event_sequence.get(), c.receipt_hash);
        if entry.receipt.as_ref().is_some_and(|old| old != &receipt) {
            return Err("IPC_LEDGER_RECEIPT_MISMATCH".into());
        }
        entry.receipt = Some(receipt);
        let result = entry.result.clone();
        Ok(self.readonly_reply(result))
    }
    pub(super) fn check_readonly_result(
        &self,
        payload: &AgentToolResultPayload,
        current: &ReferenceScope,
    ) -> Result<(), &'static str> {
        if payload.outcome == AgentToolResultPayloadOutcome::Unavailable {
            return self.ingress.unavailable_tool(payload);
        }
        let entry = self
            .readonly
            .values()
            .find(|e| e.result.tool_call_id == payload.tool_call_id)
            .ok_or("IPC_TOOL_RECEIPT_REQUIRED")?;
        let result = &entry.result;
        let outcome = match result.outcome {
            NativeAgentReadonlyResultOutcome::Completed => AgentToolResultPayloadOutcome::Completed,
            NativeAgentReadonlyResultOutcome::Failed => AgentToolResultPayloadOutcome::Failed,
        };
        if entry.scope != *current
            || entry.issued.elapsed().as_secs() >= 600
            || entry.receipt.is_none()
            || payload.agent_turn_id != result.agent_turn_id
            || payload.operation_ref.0.is_some()
            || payload.result_ref.0.as_ref() != Some(&result.result_ref)
            || payload.result_json.0.as_ref() != Some(&result.result_json)
            || payload.outcome != outcome
        {
            return Err("IPC_TOOL_RECEIPT_REQUIRED");
        }
        Ok(())
    }
}
