//! Agent preview uses the exact read-issued snapshot and the existing frozen-plan owner.
use crate::{
    document::{
        commit::{CommitError, DocumentCommitController},
        preview::PreviewError,
    },
    references::{ReferenceError, ReferenceScope},
};
use serde_json::Value;

/// Inspect original ACKed arguments without issuing a replacement read token or widening exposure.
/// A successful source inspection is still noncommittable; a patch only freezes the original plan.
pub fn preview(
    bytes: &[u8],
    controller: &mut DocumentCommitController,
    scope: &ReferenceScope,
    now: u64,
) -> Result<Value, CommitError> {
    let arguments = om_agent_contract::json::strict_json_bounded(bytes, 262144)
        .map_err(|_| CommitError::Preview(PreviewError::InvalidArgument))?;
    super::checked_data("preview_source_args", arguments)
        .map_err(|_| CommitError::Preview(PreviewError::InvalidArgument))?;
    let actual = controller.preview(bytes, scope, now, "host-agent-preview-time".into())?;
    let data = serde_json::to_value(actual)
        .map_err(|_| CommitError::Preview(PreviewError::LimitExceeded))?;
    super::checked_data("preview_source_data", data)
        .map_err(|_| CommitError::Preview(PreviewError::LimitExceeded))
}

/// Closed tool diagnostics preserve the failed authority check without pretending an edit ran.
pub(crate) fn error(error: CommitError) -> (&'static str, &'static str) {
    match error {
        CommitError::Busy => ("EDITING_BUSY", "retry_read"),
        CommitError::Invalid => ("PREVIEW_MISMATCH", "reprepare"),
        CommitError::Cancelled => ("CANCELLED", "none"),
        CommitError::UndoConflict => ("UNDO_CONFLICT", "none"),
        CommitError::Preview(error) => match error {
            PreviewError::InvalidArgument => ("INVALID_ARGUMENT", "none"),
            PreviewError::PermissionDenied
            | PreviewError::Reference(ReferenceError::PermissionDenied) => {
                ("PERMISSION_DENIED", "none")
            }
            PreviewError::StaleSnapshot
            | PreviewError::SourceHashMismatch
            | PreviewError::Reference(
                ReferenceError::StaleScope | ReferenceError::Expired | ReferenceError::Revoked,
            ) => ("STALE_SNAPSHOT", "refresh_snapshot"),
            PreviewError::IncompleteSource => ("INVALID_PATCH", "refresh_snapshot"),
            PreviewError::InvalidPatch | PreviewError::NonuniqueReplacement => {
                ("INVALID_PATCH", "none")
            }
            PreviewError::LimitExceeded | PreviewError::Reference(ReferenceError::Budget) => {
                ("LIMIT_EXCEEDED", "reduce_request")
            }
            PreviewError::Reference(ReferenceError::Invalid | ReferenceError::WrongKind) => {
                ("INVALID_REFERENCE", "refresh_snapshot")
            }
        },
    }
}
