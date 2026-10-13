//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `EvaluateScratch`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvaluateScratch {
    /// Contract field `kind`.
    pub kind: EvaluateScratchKind,
    /// Contract field `source`.
    pub source: String,
    /// Contract field `dialect`.
    pub dialect: Dialect,
    /// Contract field `definition_snapshot_ref`.
    pub definition_snapshot_ref: Nullable<String>,
    /// Contract field `use_notebook_definitions`.
    pub use_notebook_definitions: bool,
    /// Contract field `timeout_ms`.
    pub timeout_ms: u32,
}
impl std::fmt::Debug for EvaluateScratch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("EvaluateScratch { redacted }")
    }
}
