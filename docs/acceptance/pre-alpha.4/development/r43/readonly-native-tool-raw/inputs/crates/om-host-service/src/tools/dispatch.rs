//! Closed native readonly results. The peer's Pi-converted argument object is never input here.
use super::{
    catalog,
    read::{ReadError, ReadNotebookService},
};
use crate::{
    agent::ingress::ToolIntent,
    document::{commit::DocumentCommitController, coordinator::SourceCoordinator},
    kernel::acceptance::AcceptedKernelState,
    protocol::generated::AgentToolName,
    references::ReferenceScope,
};
use serde_json::{Value, json};
/// Actual owner/checkpoint facts. This struct is internal native authority, never deserialized.
pub struct ReadonlySource<'a> {
    /// Confirmed source owner and its real preview snapshot issuer.
    pub controller: &'a mut DocumentCommitController,
    /// Bounded ephemeral read retention for this actual task.
    pub reads: &'a mut ReadNotebookService,
    /// Actual accepted parser/ownership facts.
    pub coordinator: &'a SourceCoordinator,
    /// Actual accepted checkpoint; None does not invent user bindings.
    pub accepted: Option<&'a AcceptedKernelState>,
    /// Trusted current document/task/grants/config/definition binding.
    pub scope: &'a ReferenceScope,
    /// Native monotonic time.
    pub now: u64,
}
fn read_error(error: ReadError) -> (&'static str, &'static str) {
    match error {
        ReadError::InvalidArgument => ("INVALID_ARGUMENT", "none"),
        ReadError::InvalidReference => ("INVALID_REFERENCE", "refresh_snapshot"),
        ReadError::PermissionDenied => ("PERMISSION_DENIED", "none"),
        ReadError::StaleSnapshot => ("STALE_SNAPSHOT", "refresh_snapshot"),
        ReadError::NotFound => ("INVALID_REFERENCE", "refresh_snapshot"),
        ReadError::LimitExceeded => ("LIMIT_EXCEEDED", "reduce_request"),
        ReadError::NotAvailable => ("NOT_AVAILABLE", "none"),
    }
}
fn catalog_error(error: catalog::CatalogError) -> (&'static str, &'static str) {
    match error {
        catalog::CatalogError::InvalidArgument => ("INVALID_ARGUMENT", "none"),
        catalog::CatalogError::StaleMetadata => ("STALE_SNAPSHOT", "reprepare"),
        catalog::CatalogError::NotFound => ("NOT_AVAILABLE", "none"),
        catalog::CatalogError::LimitExceeded => ("LIMIT_EXCEEDED", "reduce_request"),
    }
}
/// Execute exactly one original, ACKed readonly proposal. No CAS request, file IO or source write.
pub fn execute(
    intent: &ToolIntent,
    args: &[u8],
    source: ReadonlySource<'_>,
) -> Result<(String, bool), String> {
    let current_revision = source.controller.owner().snapshot().revision.get();
    let outcome = match intent.name {
        AgentToolName::ReadNotebook => source
            .reads
            .read(
                args,
                source.controller,
                source.coordinator,
                source.accepted,
                source.scope,
                source.now,
            )
            .map(|p| (p.data, Some(p.pagination), p.source_revision))
            .map_err(read_error),
        AgentToolName::SearchFunctions => catalog::search(args)
            .map(|p| (p, None, current_revision))
            .map_err(catalog_error),
        AgentToolName::GetFunctionDocs => {
            let empty = om_kernel::source::SourceExecutionContext {
                known_functions: vec![],
                owned: vec![],
            };
            let stale = source.accepted.is_some_and(|a| {
                a.state().binding().source_snapshot_hash != source.scope.snapshot_hash
                    || a.state().binding().execution_epoch != source.scope.execution_epoch
                    || a.receipt().producer.config_revision.get() != source.scope.config_revision
            });
            let by_id = om_agent_contract::json::strict_json_bounded(args, 65536)
                .ok()
                .is_some_and(|v| v.get("function_id").is_some());
            if stale && !by_id {
                Err(("CONTEXT_NOT_READY", "reprepare"))
            } else {
                catalog::docs(
                    args,
                    source
                        .accepted
                        .map_or(&empty, |a| a.state().execution_context()),
                )
                .map(|p| (p, None, current_revision))
                .map_err(catalog_error)
            }
        }
        _ => return Err("NATIVE_TOOL_UNAVAILABLE".into()),
    };
    let success = outcome.is_ok();
    let (data, pagination, revision, error) = match outcome {
        Ok((data, page, revision)) => (data, page, revision, Value::Null),
        Err((code, recovery)) => (
            Value::Null,
            if intent.name == AgentToolName::ReadNotebook {
                Some(json!({"truncated":false,"next_cursor":null}))
            } else {
                None
            },
            current_revision,
            json!({"code":code,"message":code,"recovery":recovery,"details":{}}),
        ),
    };
    let mut envelope = json!({"ok":success,"status":if success {"completed"} else {"failed"},"request_ref":intent.operation_id,"operation_ref":null,"document":{"id":source.scope.document,"generation":source.scope.generation,"revision":revision},"data":data,"error":error});
    envelope["pagination"] = pagination.unwrap_or(json!({"truncated":false,"next_cursor":null}));
    let definition = match intent.name {
        AgentToolName::ReadNotebook => "read_notebook_result",
        AgentToolName::SearchFunctions => "search_functions_result",
        AgentToolName::GetFunctionDocs => "get_function_docs_result",
        _ => unreachable!(),
    };
    let envelope =
        super::checked_data(definition, envelope).map_err(|_| "TOOL_RESULT_LIMIT".to_owned())?;
    serde_json::to_string(&envelope)
        .map(|value| (value, success))
        .map_err(|_| "TOOL_RESULT_ENCODING".into())
}
