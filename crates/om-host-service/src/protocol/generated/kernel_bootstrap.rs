//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `KernelBootstrap`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KernelBootstrap {
    /// Contract field `type`.
    pub r#type: KernelBootstrapType,
    /// Contract field `operation_id`.
    pub operation_id: String,
    /// Contract field `expected_source_hash`.
    pub expected_source_hash: String,
}
impl std::fmt::Debug for KernelBootstrap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("KernelBootstrap { redacted }")
    }
}
