//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `InvocationBinding`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InvocationBinding {
    /// Contract field `task_id`.
    pub task_id: String,
    /// Contract field `task_generation`.
    pub task_generation: Serial,
    /// Contract field `grant_ref`.
    pub grant_ref: String,
}
impl std::fmt::Debug for InvocationBinding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("InvocationBinding { redacted }")
    }
}
