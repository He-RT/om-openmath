//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `KernelState`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KernelState {
    /// Contract field `type`.
    pub r#type: KernelStateType,
}
impl std::fmt::Debug for KernelState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("KernelState { redacted }")
    }
}
