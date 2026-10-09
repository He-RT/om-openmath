//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `GhostCandidate`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GhostCandidate {
    /// Contract field `record_type`.
    pub record_type: GhostCandidateRecordType,
    /// Contract field `candidate_id`.
    pub candidate_id: String,
    /// Contract field `request_id`.
    pub request_id: String,
    /// Contract field `key`.
    pub key: EditorKey,
    /// Contract field `anchor_utf8`.
    pub anchor_utf8: u32,
    /// Contract field `model_request_snapshot_id`.
    pub model_request_snapshot_id: String,
    /// Contract field `suggestion_text`.
    pub suggestion_text: String,
    /// Contract field `phase`.
    pub phase: GhostCandidatePhase,
    /// Contract field `enters_source_before_accept`.
    pub enters_source_before_accept: bool,
}
impl std::fmt::Debug for GhostCandidate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("GhostCandidate { redacted }")
    }
}
