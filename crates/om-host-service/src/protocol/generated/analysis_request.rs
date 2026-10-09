//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `AnalysisRequest`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnalysisRequest {
    /// Contract field `record_type`.
    pub record_type: AnalysisRequestRecordType,
    /// Contract field `request_id`.
    pub request_id: String,
    /// Contract field `key`.
    pub key: EditorKey,
    /// Contract field `kind`.
    pub kind: AnalysisRequestKind,
    /// Contract field `non_evaluating`.
    pub non_evaluating: bool,
}
impl std::fmt::Debug for AnalysisRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("AnalysisRequest { redacted }")
    }
}
