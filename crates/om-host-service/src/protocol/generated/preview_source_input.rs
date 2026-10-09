//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `PreviewSourceInput`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreviewSourceInput {
    /// Contract field `kind`.
    pub kind: PreviewSourceInputKind,
    /// Contract field `source`.
    pub source: String,
    /// Contract field `dialect`.
    pub dialect: Dialect,
}
impl std::fmt::Debug for PreviewSourceInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PreviewSourceInput { redacted }")
    }
}
