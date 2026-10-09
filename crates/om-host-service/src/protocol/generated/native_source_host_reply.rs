//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `NativeSourceHostReply`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeSourceHostReply {
    /// Contract field `protocol_version`.
    pub protocol_version: u32,
    /// Contract field `kind`.
    pub kind: NativeSourceHostReplyKind,
    /// Contract field `document_generation`.
    pub document_generation: Serial,
    /// Contract field `snapshot`.
    pub snapshot: Nullable<NativeSourceSnapshot>,
    /// Contract field `plan`.
    pub plan: Nullable<NativeSourceCommit>,
    /// Contract field `preview`.
    pub preview: Nullable<serde_json::Value>,
    /// Contract field `reference`.
    pub reference: Nullable<String>,
    /// Contract field `operation`.
    pub operation: Nullable<NativeCommitState>,
    /// Contract field `owner_time_ms`.
    pub owner_time_ms: Serial,
}
impl std::fmt::Debug for NativeSourceHostReply {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NativeSourceHostReply { redacted }")
    }
}
