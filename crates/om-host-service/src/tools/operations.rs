//! Read-only original operation facts. No evaluator, IO dispatch, retry callback or write grant.
use crate::{agent::ingress::ToolIntent, protocol::generated::*, references::ReferenceScope};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

/// Trusted IO lookup derived exclusively from the original acknowledged argument bytes.
pub(crate) fn prepare(
    intent: &ToolIntent,
    args: &[u8],
    scope: &ReferenceScope,
    store: &str,
) -> Result<NativeOperationLookup, String> {
    scope.validate().map_err(|_| "STALE_DOCUMENT")?;
    if intent.name != AgentToolName::GetOperationStatus || !scope.can_read {
        return Err("PERMISSION_DENIED".into());
    }
    if format!("{:x}", Sha256::digest(args)) != intent.arguments_hash {
        return Err("IPC_CALL_SCOPE_MISMATCH".into());
    }
    let value = om_agent_contract::json::strict_json_bounded(args, 65536)
        .map_err(|_| "INVALID_ARGUMENT")?;
    super::checked_data("get_operation_status_args", value.clone())
        .map_err(|_| "INVALID_ARGUMENT")?;
    Ok(NativeOperationLookup {
        operation_id: intent.operation_id.clone(),
        arguments_hash: intent.arguments_hash.clone(),
        operation_ref: value["operation_ref"]
            .as_str()
            .ok_or("INVALID_ARGUMENT")?
            .into(),
        document_id: scope.document.clone(),
        store_id: store.into(),
    })
}
/// Actual native facts plus original result receipts for the independent immutable-results owner.
pub(crate) struct Projection {
    state: &'static str,
    known: bool,
    committed: Option<bool>,
    transaction: Option<String>,
    run: Option<String>,
    hint: String,
    pub(crate) receipts: Vec<NativeKernelReceipt>,
}
fn kernel(receipt: &NativeKernelReceipt, lookup: &NativeOperationLookup) -> Result<(), String> {
    if receipt.protocol_version != 1
        || receipt.store_id != lookup.store_id
        || receipt.document_id != lookup.document_id
        || receipt.producer.document_id != lookup.document_id
        || receipt.producer.kernel_state_revision != receipt.kernel_state_revision
    {
        return Err("IPC_LEDGER_RECEIPT_MISMATCH".into());
    }
    Ok(())
}
/// A missing row remains unknown. Only a real durable receipt or validated run conclusion is final.
pub(crate) fn project(
    lookup: &NativeOperationLookup,
    observation: &NativeOperationObservation,
) -> Result<Projection, String> {
    if lookup.operation_ref != observation.operation_ref
        || lookup.document_id != observation.document_id
        || lookup.store_id != observation.store_id
    {
        return Err("IPC_LEDGER_RECEIPT_MISMATCH".into());
    }
    let variants = usize::from(observation.source_receipt.0.is_some())
        + usize::from(observation.kernel_receipt.0.is_some())
        + usize::from(observation.run.0.is_some())
        + usize::from(observation.claim.0.is_some());
    if variants > 1 {
        return Err("IPC_LEDGER_RECEIPT_MISMATCH".into());
    }
    let mut value = Projection {
        state: "unknown",
        known: false,
        committed: None,
        transaction: None,
        run: None,
        receipts: vec![],
        hint: "No complete original fact is available; absence never authorizes replay".into(),
    };
    if let Some(source) = &observation.source_receipt.0 {
        let r = &source.receipt;
        if source.protocol_version != 1
            || r.document_id.0.as_ref() != Some(&lookup.document_id)
            || r.store_id != lookup.store_id
            || r.operation_id != lookup.operation_ref
            || r.phase != OperationReceiptPhase::Completed
            || r.transaction_id.0.is_none()
            || r.committed_revision.0.is_none()
        {
            return Err("IPC_LEDGER_RECEIPT_MISMATCH".into());
        }
        value.state = "completed";
        value.known = true;
        value.committed = Some(true);
        value.transaction = r.transaction_id.0.clone();
        value.hint = "Original source receipt found; any undo requires a new current-snapshot conflict check".into();
    } else if let Some(r) = &observation.kernel_receipt.0 {
        kernel(r, lookup)?;
        if r.operation_id != lookup.operation_ref {
            return Err("IPC_LEDGER_RECEIPT_MISMATCH".into());
        }
        value.state = if r.producer.terminal_status == NativeKernelTerminalStatus::Done {
            "completed"
        } else {
            "failed"
        };
        value.known = true;
        value.committed = Some(true);
        value.hint =
            "Original checkpoint receipt found; this query does not rerun or restore computation"
                .into();
        if r.producer.cell_id.0.is_some() {
            value.receipts.push(r.clone());
        }
    } else if let Some(run) = &observation.run.0 {
        if run.event.event_sequence.get() == 0
            || run.accepted_effects < run.accepted_cells
            || run.accepted_effects.get() > 1001
            || run.kernel_receipts.len() > 32
        {
            return Err("IPC_LEDGER_RECEIPT_MISMATCH".into());
        }
        let state = match run.phase {
            NativeOperationRunObservationPhase::Queued => "queued",
            NativeOperationRunObservationPhase::Running => "running",
            NativeOperationRunObservationPhase::Completed => "completed",
            NativeOperationRunObservationPhase::Partial => "partial",
            NativeOperationRunObservationPhase::Failed => "failed",
            NativeOperationRunObservationPhase::Cancelled => "cancelled",
            NativeOperationRunObservationPhase::Unknown => "unknown",
        };
        if (matches!(state, "completed" | "partial") && run.accepted_cells.get() == 0)
            || (state == "failed" && run.accepted_cells.get() != 0)
            || (matches!(state, "completed" | "partial" | "failed" | "cancelled")
                && run.original_active_operation.0.is_some())
            || run.kernel_receipts.len() as u64 > run.accepted_cells.get()
        {
            return Err("IPC_LEDGER_RECEIPT_MISMATCH".into());
        }
        let mut ids = std::collections::BTreeSet::new();
        for r in &run.kernel_receipts {
            kernel(r, lookup)?;
            if r.producer.cell_id.0.is_none() || !ids.insert(&r.operation_id) {
                return Err("IPC_LEDGER_RECEIPT_MISMATCH".into());
            }
        }
        value.state = state;
        value.known = matches!(state, "completed" | "partial" | "failed" | "cancelled");
        value.committed = if run.accepted_effects.get() > 0 {
            Some(true)
        } else if value.known {
            Some(false)
        } else {
            None
        };
        value.run = Some(lookup.operation_ref.clone());
        value.receipts = run.kernel_receipts.clone();
        value.hint = format!(
            "Original durable run: {} accepted cell(s), {} accepted checkpoint effect(s). Unresolved children never authorize a fresh-ID replay",
            run.accepted_cells.get(),
            run.accepted_effects.get()
        );
    } else if let Some(claim) = &observation.claim.0 {
        if claim.event.event_sequence.get() == 0 {
            return Err("IPC_LEDGER_RECEIPT_MISMATCH".into());
        }
        value.state = match claim.phase {
            NativeOperationClaimObservationPhase::Queued => "queued",
            NativeOperationClaimObservationPhase::Running => "running",
            NativeOperationClaimObservationPhase::Unknown => "unknown",
        };
        value.hint =
            "Original admission/start exists; no final physical outcome has been proved".into();
    }
    Ok(value)
}
/// Query completion is independent from the queried operation's state, including unknown.
pub(crate) fn envelope(
    lookup: &NativeOperationLookup,
    scope: &ReferenceScope,
    projection: &Projection,
    outputs: &Value,
) -> Result<String, String> {
    let refs = outputs
        .as_object()
        .into_iter()
        .flat_map(|m| m.values())
        .filter_map(|v| v["result_ref"].as_str())
        .collect::<Vec<_>>();
    let value = json!({"ok":true,"status":"completed","request_ref":lookup.operation_id,"operation_ref":null,
        "document":{"id":scope.document,"generation":scope.generation,"revision":scope.revision},
        "data":{"operation_state":projection.state,"outcome_known":projection.known,"effect_committed":projection.committed,
            "transaction_ref":projection.transaction,"run_ref":projection.run,"result_refs":refs,"recovery_hint":projection.hint},
        "error":null,"pagination":{"truncated":false,"next_cursor":null}});
    super::checked_data("get_operation_status_result", value.clone())
        .map_err(|_| "TOOL_RESULT_LIMIT".to_string())?;
    Ok(value.to_string())
}
/// Closed argument failures are data; they do not manufacture an operation outcome.
pub(crate) fn failure(
    intent: &ToolIntent,
    scope: &ReferenceScope,
    code: &str,
) -> Result<String, String> {
    let code = match code {
        "PERMISSION_DENIED" | "NOT_AVAILABLE" => code,
        "STALE_DOCUMENT" => "STALE_SNAPSHOT",
        _ => "INVALID_ARGUMENT",
    };
    let value = json!({"ok":false,"status":"failed","request_ref":intent.operation_id,"operation_ref":null,
        "document":{"id":scope.document,"generation":scope.generation,"revision":scope.revision},"data":null,
        "error":{"code":code,"message":code,"recovery":"none","details":{}},"pagination":{"truncated":false,"next_cursor":null}});
    super::checked_data("get_operation_status_result", value.clone())
        .map_err(|_| "TOOL_RESULT_LIMIT".to_string())?;
    Ok(value.to_string())
}
