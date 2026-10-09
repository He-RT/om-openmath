//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `NativeSourceCommit`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeSourceCommit {
    /// Contract field `protocol_version`.
    pub protocol_version: u32,
    /// Contract field `generation`.
    pub generation: Serial,
    /// Contract field `commit`.
    pub commit: DocumentCommit,
    /// Contract field `before`.
    pub before: NativeSourceSnapshot,
    /// Contract field `after`.
    pub after: NativeSourceSnapshot,
    /// Contract field `calculation_change`.
    pub calculation_change: Nullable<NativeCalculationChange>,
}
impl std::fmt::Debug for NativeSourceCommit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NativeSourceCommit { redacted }")
    }
}
