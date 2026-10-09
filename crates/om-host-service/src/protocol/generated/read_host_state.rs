//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `ReadHostState`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReadHostState {
    /// Contract field `kind`.
    pub kind: ReadHostStateKind,
}
impl std::fmt::Debug for ReadHostState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ReadHostState { redacted }")
    }
}
