//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `AnalyzeEditor`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnalyzeEditor {
    /// Contract field `kind`.
    pub kind: AnalyzeEditorKind,
    /// Contract field `key`.
    pub key: EditorKey,
    /// Contract field `source`.
    pub source: String,
    /// Contract field `analysis_kind`.
    pub analysis_kind: AnalyzeEditorAnalysisKind,
    /// Contract field `non_evaluating`.
    pub non_evaluating: bool,
}
impl std::fmt::Debug for AnalyzeEditor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("AnalyzeEditor { redacted }")
    }
}
