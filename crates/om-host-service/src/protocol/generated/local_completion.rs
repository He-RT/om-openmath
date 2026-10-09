//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `LocalCompletion`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalCompletion {
    /// Contract field `record_type`.
    pub record_type: LocalCompletionRecordType,
    /// Contract field `candidate_id`.
    pub candidate_id: String,
    /// Contract field `key`.
    pub key: EditorKey,
    /// Contract field `replacement_span`.
    pub replacement_span: SourceSpan,
    /// Contract field `label`.
    pub label: String,
    /// Contract field `insert_text`.
    pub insert_text: String,
    /// Contract field `insertion_kind`.
    pub insertion_kind: LocalCompletionInsertionKind,
    /// Contract field `detail`.
    pub detail: Nullable<String>,
    /// Contract field `origin`.
    pub origin: LocalCompletionOrigin,
}
impl std::fmt::Debug for LocalCompletion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("LocalCompletion { redacted }")
    }
}
