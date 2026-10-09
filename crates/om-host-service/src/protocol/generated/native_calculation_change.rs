//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `NativeCalculationChange`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeCalculationChange {
    /// Contract field `before`.
    pub before: NativeCalculationSettings,
    /// Contract field `after`.
    pub after: NativeCalculationSettings,
}
impl std::fmt::Debug for NativeCalculationChange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NativeCalculationChange { redacted }")
    }
}
