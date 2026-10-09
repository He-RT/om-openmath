//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `ReadHostCapabilities`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReadHostCapabilities {
    /// Contract field `kind`.
    pub kind: ReadHostCapabilitiesKind,
}
impl std::fmt::Debug for ReadHostCapabilities {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ReadHostCapabilities { redacted }")
    }
}
