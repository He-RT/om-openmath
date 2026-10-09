//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `SourcePrepareManual`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourcePrepareManual {
    /// Contract field `type`.
    pub r#type: SourcePrepareManualType,
    /// Contract field `operations`.
    pub operations: Vec<NativeSourceOperation>,
}
impl std::fmt::Debug for SourcePrepareManual {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SourcePrepareManual { redacted }")
    }
}
