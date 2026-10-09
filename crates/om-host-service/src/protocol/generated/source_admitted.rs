//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `SourceAdmitted`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceAdmitted {
    /// Contract field `type`.
    pub r#type: SourceAdmittedType,
    /// Contract field `operation_id`.
    pub operation_id: String,
    /// Contract field `admission`.
    pub admission: NativeSourceAdmission,
}
impl std::fmt::Debug for SourceAdmitted {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SourceAdmitted { redacted }")
    }
}
