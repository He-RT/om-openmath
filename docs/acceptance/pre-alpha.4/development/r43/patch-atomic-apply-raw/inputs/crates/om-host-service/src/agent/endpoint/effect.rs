//! Completed/uncertain effect data is issued from the actual source owner, never a peer claim.
use super::*;
use crate::references::ReferenceScope;

pub(super) struct EffectEntry {
    scope: ReferenceScope,
    result: NativeAgentEffectResult,
    receipt: Option<(String, u64, String)>,
}
impl AgentEndpoint {
    /// Resolve original ACKed apply arguments under the currently live, write-enabled task.
    pub fn apply_input(
        &self,
        operation: &str,
        binding: &DocumentBinding,
    ) -> Result<(ToolIntent, String), String> {
        self.check_native_scope(&self.scope, binding)?;
        let (intent, args) = self
            .ingress
            .original_tool(operation)
            .map_err(str::to_owned)?;
        if intent.name != AgentToolName::ApplyNotebookPatch {
            return Err("INVALID_AGENT_TOOL".into());
        }
        Ok((intent, args))
    }
    /// A final physical barrier can use only this live activation's actual grants.
    pub fn apply_scope(
        &self,
        operation: &str,
        binding: &DocumentBinding,
        current: ReferenceScope,
    ) -> Result<ReferenceScope, String> {
        self.apply_input(operation, binding)?;
        let scope = self.ingress.read_scope(current);
        if !scope.execution_mode || !scope.can_write {
            return Err("PERMISSION_DENIED".into());
        }
        Ok(scope)
    }
    /// A frozen native result can be re-read but cannot be recomputed by a resumed callback.
    pub fn cached_effect(
        &self,
        operation: &str,
        scope: &ReferenceScope,
    ) -> Result<Option<NativeAgentHostReply>, String> {
        match self.effects.get(operation) {
            Some(entry) if entry.scope != *scope => Err("STALE_SNAPSHOT".into()),
            Some(entry) => Ok(Some(self.effect_reply(entry.result.clone()))),
            None => Ok(None),
        }
    }
    /// Freeze a result only after the source owner has interpreted its real original-ID state.
    pub fn register_effect(
        &mut self,
        intent: &ToolIntent,
        scope: &ReferenceScope,
        operation: Option<String>,
        json: String,
    ) -> Result<NativeAgentHostReply, String> {
        let (original, _) = self
            .ingress
            .original_tool(&intent.operation_id)
            .map_err(str::to_owned)?;
        if original.arguments_hash != intent.arguments_hash
            || self.effects.len() >= 32
            || self.effects.contains_key(&intent.operation_id)
        {
            return Err("IPC_CALL_SCOPE_MISMATCH".into());
        }
        let envelope: Value = serde_json::from_str(&json).map_err(|_| "INVALID_TOOL_RESULT")?;
        crate::tools::checked_data("apply_notebook_patch_result", envelope.clone())
            .map_err(|_| "INVALID_TOOL_RESULT")?;
        let outcome = serde_json::from_value(envelope["status"].clone())
            .map_err(|_| "INVALID_TOOL_RESULT")?;
        let result = NativeAgentEffectResult {
            operation_id: intent.operation_id.clone(),
            agent_turn_id: intent.turn_id.clone(),
            tool_call_id: intent.call_id.clone(),
            name: intent.name.clone(),
            result_ref: identity().map_err(str::to_owned)?,
            result_hash: format!("{:x}", Sha256::digest(json.as_bytes())),
            result_json: json,
            operation_ref: Nullable(operation),
            outcome,
        };
        self.effects.insert(
            intent.operation_id.clone(),
            EffectEntry {
                scope: scope.clone(),
                result: result.clone(),
                receipt: None,
            },
        );
        Ok(self.effect_reply(result))
    }
    fn effect_reply(&self, result: NativeAgentEffectResult) -> NativeAgentHostReply {
        let mut reply = self.reply(NativeAgentHostReplyDisposition::Data);
        reply.effect_result = Nullable(Some(result));
        reply
    }
    pub(super) fn confirm_effect(
        &mut self,
        c: NativeAgentConfirmTool,
        scope: &ReferenceScope,
    ) -> Result<NativeAgentHostReply, String> {
        self.ingress
            .original_tool(&c.operation_id)
            .map_err(str::to_owned)?;
        let entry = self
            .effects
            .get_mut(&c.operation_id)
            .ok_or("EFFECT_PRODUCER_REQUIRED")?;
        if entry.scope != *scope
            || entry.result.result_ref != c.result_ref
            || entry.result.result_hash != c.result_hash
            || c.event_id != format!("effect-{}", c.operation_id)
            || c.event_sequence.get() == 0
        {
            return Err("IPC_LEDGER_RECEIPT_MISMATCH".into());
        }
        let receipt = (c.event_id, c.event_sequence.get(), c.receipt_hash);
        if entry.receipt.as_ref().is_some_and(|r| r != &receipt) {
            return Err("IPC_LEDGER_RECEIPT_MISMATCH".into());
        }
        entry.receipt = Some(receipt);
        let result = entry.result.clone();
        Ok(self.effect_reply(result))
    }
    pub(super) fn check_tool_result(
        &self,
        payload: &AgentToolResultPayload,
        scope: &ReferenceScope,
    ) -> Result<(), &'static str> {
        let Some(entry) = self
            .effects
            .values()
            .find(|e| e.result.tool_call_id == payload.tool_call_id)
        else {
            return self.check_readonly_result(payload, scope);
        };
        let result = &entry.result;
        let outcome: AgentToolResultPayloadOutcome = serde_json::from_value(
            serde_json::to_value(&result.outcome).map_err(|_| "IPC_TOOL_RECEIPT_REQUIRED")?,
        )
        .map_err(|_| "IPC_TOOL_RECEIPT_REQUIRED")?;
        if entry.scope != *scope
            || entry.receipt.is_none()
            || payload.agent_turn_id != result.agent_turn_id
            || payload.operation_ref.0 != result.operation_ref.0
            || payload.result_ref.0.as_ref() != Some(&result.result_ref)
            || payload.result_json.0.as_ref() != Some(&result.result_json)
            || payload.outcome != outcome
        {
            return Err("IPC_TOOL_RECEIPT_REQUIRED");
        }
        Ok(())
    }
}
