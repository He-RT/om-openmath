//! Native storage observations are data only. This bridge has no computation or source executor.
use super::*;
use crate::{
    agent::ingress::ToolIntent,
    tools::operations::{self, Projection},
};
use serde_json::json;
pub(super) struct PendingOperationQuery {
    lookup: NativeOperationLookup,
    scope: ReferenceScope,
    projection: Option<Projection>,
    observation_hash: Option<String>,
    result_request: Option<String>,
}
impl SourceEndpoint {
    pub(super) fn query_agent_operation(
        &mut self,
        c: NativeAgentQueryOperation,
    ) -> Result<NativeAgentHostReply, String> {
        let binding = self.binding();
        let agent = self.agent.as_ref().ok_or("AGENT_NOT_ACTIVE")?;
        agent.check_native_scope(&c.scope, &binding)?;
        let current = agent.readonly_scope(self.current_source_scope());
        let (intent, args, cached) = agent.readonly_input(
            &NativeAgentReadTool {
                r#type: NativeAgentReadToolType::AgentReadTool,
                scope: c.scope,
                operation_id: c.operation_id.clone(),
            },
            &binding,
            &current,
        )?;
        if intent.name != AgentToolName::GetOperationStatus {
            return Err("INVALID_AGENT_TOOL".into());
        }
        if let Some(cached) = cached {
            return Ok(agent.readonly_reply(cached));
        }
        if !self.operation_queries.contains_key(&c.operation_id) {
            if self.operation_queries.len() >= 32 {
                return Err("IPC_TASK_BUDGET".into());
            }
            let lookup = match operations::prepare(&intent, args.as_bytes(), &current, &self.store)
            {
                Ok(lookup) => lookup,
                Err(code) => return self.operation_query_failure(&intent, &current, &code),
            };
            self.operation_queries.insert(
                c.operation_id.clone(),
                PendingOperationQuery {
                    lookup,
                    scope: current.clone(),
                    projection: None,
                    observation_hash: None,
                    result_request: None,
                },
            );
        }
        let mut pending = self
            .operation_queries
            .remove(&c.operation_id)
            .expect("inserted");
        let result = (|| {
            // Original physical readback may advance source/definition facts. Read-only lookup
            // authority depends on the exact live document/task/grant lifetime, never old defs.
            let original = &pending.scope;
            if original.runtime != current.runtime
                || original.document != current.document
                || original.generation != current.generation
                || original.task != current.task
                || original.task_generation != current.task_generation
                || original.grant_revision != current.grant_revision
                || !current.can_read
            {
                return Err("STALE_SNAPSHOT".into());
            }
            if c.failure_code.0.is_some() {
                if c.observation.0.is_some() || pending.projection.is_some() {
                    return Err("IPC_CALL_SCOPE_MISMATCH".into());
                }
                return self.operation_query_failure(&intent, &current, "NOT_AVAILABLE");
            }
            if let Some(observation) = c.observation.0 {
                let hash = format!(
                    "{:x}",
                    Sha256::digest(
                        serde_json::to_vec(&observation).map_err(|_| "INVALID_ARGUMENT")?
                    )
                );
                if let Some(original) = &pending.observation_hash {
                    if original != &hash {
                        return Err("IPC_LEDGER_RECEIPT_MISMATCH".into());
                    }
                } else {
                    pending.projection = Some(operations::project(&pending.lookup, &observation)?);
                    pending.observation_hash = Some(hash);
                }
            }
            let Some(projection) = &pending.projection else {
                let mut reply = self.agent.as_ref().expect("checked").pending_readonly();
                reply.operation_query = Nullable(Some(pending.lookup.clone()));
                return Ok(reply);
            };
            let mut outputs = json!({});
            if !projection.receipts.is_empty() {
                self.flush_result_state();
                if self.pending_result_state.is_some() {
                    return Ok(self.agent.as_ref().expect("checked").pending_readonly());
                }
                if self.results.is_none() {
                    self.results = Some(crate::results::runtime::ResultRuntime::new()?);
                }
                let results = self.results.as_ref().expect("created");
                let response = if let Some(id) = &pending.result_request {
                    Some(results.command(
                        NativeResultHostCommand::ResultStatus(ResultStatus {
                            r#type: ResultStatusType::ResultStatus,
                            request_id: id.clone(),
                        }),
                        &current,
                    )?)
                } else {
                    let id = format!(
                        "operation-result-{:x}",
                        Sha256::digest(intent.operation_id.as_bytes())
                    );
                    match results.run_outputs(&id, projection.receipts.clone(), &current) {
                        Ok(response) => {
                            pending.result_request = Some(id);
                            Some(response)
                        }
                        Err(code) if code == "RESULT_QUEUE_BUSY" => {
                            return Ok(self.agent.as_ref().expect("checked").pending_readonly());
                        }
                        Err(code) if code == "RESULT_NOT_AVAILABLE" => None,
                        Err(code) => return Err(code),
                    }
                };
                if let Some(response) = response {
                    match response.phase {
                        NativeResultHostReplyPhase::Queued
                        | NativeResultHostReplyPhase::Running => {
                            return Ok(self.agent.as_ref().expect("checked").pending_readonly());
                        }
                        NativeResultHostReplyPhase::Completed => outputs = response.payload,
                        _ => (),
                    }
                }
            }
            let bytes = operations::envelope(&pending.lookup, &current, projection, &outputs)?;
            self.agent
                .as_mut()
                .expect("checked")
                .register_readonly(&intent, &current, bytes, true)
        })();
        self.operation_queries.insert(c.operation_id, pending);
        result
    }
    fn operation_query_failure(
        &mut self,
        intent: &ToolIntent,
        current: &ReferenceScope,
        code: &str,
    ) -> Result<NativeAgentHostReply, String> {
        let bytes = operations::failure(intent, current, code)?;
        self.agent
            .as_mut()
            .ok_or("AGENT_NOT_ACTIVE")?
            .register_readonly(intent, current, bytes, false)
    }
}
