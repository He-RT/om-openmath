//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `SourceOpen`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceOpen {
    /// Contract field `type`.
    pub r#type: SourceOpenType,
    /// Contract field `snapshot`.
    pub snapshot: NativeSourceSnapshot,
    /// Contract field `store_id`.
    pub store_id: String,
    /// Contract field `calculation`.
    pub calculation: Nullable<NativeCalculationSettings>,
    /// Contract field `config_revision`.
    pub config_revision: Serial,
}
impl std::fmt::Debug for SourceOpen {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SourceOpen { redacted }")
    }
}
