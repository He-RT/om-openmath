//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `NativeSourceCell`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeSourceCell {
    /// Contract field `id`.
    pub id: String,
    /// Contract field `kind`.
    pub kind: NativeCellKind,
    /// Contract field `source`.
    pub source: String,
    /// Contract field `dialect`.
    pub dialect: Dialect,
}
impl std::fmt::Debug for NativeSourceCell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NativeSourceCell { redacted }")
    }
}
