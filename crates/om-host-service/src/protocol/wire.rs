//! Versioned native request decode; valid shapes still require actual owner admission.
use super::{
    generated::{HostInit, RequestEnvelope},
    validation,
};
use serde_json::Value;
use std::sync::OnceLock;

/// Native control protocol version, independent of ABI and application version.
pub const PROTOCOL_VERSION: u32 = 1;
/// Actual implemented modern language feature set version.
pub const LANGUAGE_FEATURE_VERSION: u32 = 1;
static SCHEMA: OnceLock<Value> = OnceLock::new();
/// Checked-in self-contained wire schema. It never retrieves external references.
pub fn schema() -> &'static Value {
    SCHEMA.get_or_init(|| {
        serde_json::from_str(include_str!(
            "../../../../docs/design/native-host-wire.schema.json"
        ))
        .expect("invariant: checked-in native host wire schema must be valid")
    })
}
/// Decode initialization without creating a service or granting a task permission.
pub fn decode_init(bytes: &[u8]) -> Result<HostInit, String> {
    validation::decode(bytes, &schema()["$defs"]["HostInit"], schema())
}
/// Decode a submission before separate runtime/generation/scope checks.
pub fn decode_request(bytes: &[u8]) -> Result<RequestEnvelope, String> {
    validation::decode(bytes, &schema()["$defs"]["RequestEnvelope"], schema())
}

/// Verify owner-provided runtime/document facts after decode. This is not a task grant.
pub fn check_owner_binding(
    request: &RequestEnvelope,
    runtime_instance_id: &str,
    current_document: Option<&super::generated::DocumentBinding>,
) -> Result<(), &'static str> {
    if request.runtime_instance_id != runtime_instance_id {
        return Err("STALE_RUNTIME");
    }
    if let Some(requested) = &request.document_binding.0 {
        let Some(current) = current_document else {
            return Err("STALE_DOCUMENT");
        };
        if requested.document_id != current.document_id
            || requested.generation != current.generation
            || requested.document_revision != current.document_revision
            || requested.execution_epoch != current.execution_epoch
        {
            return Err("STALE_DOCUMENT");
        }
    }
    Ok(())
}
