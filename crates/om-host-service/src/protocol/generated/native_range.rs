//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `NativeRange`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeRange {
    /// Contract field `location`.
    pub location: Serial,
    /// Contract field `length`.
    pub length: Serial,
}
impl std::fmt::Debug for NativeRange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NativeRange { redacted }")
    }
}
