//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `Operation`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Operation {
    /// Contract field `operation_ref`.
    pub operation_ref: String,
    /// Contract field `task_id`.
    pub task_id: Nullable<String>,
    /// Contract field `kind`.
    pub kind: OperationKind,
    /// Contract field `phase`.
    pub phase: OperationPhase,
    /// Contract field `effect_state`.
    pub effect_state: OperationEffectState,
    /// Contract field `producer`.
    pub producer: Nullable<ProducerBinding>,
    /// Contract field `cancel_requested`.
    pub cancel_requested: bool,
    /// Contract field `result_refs`.
    pub result_refs: Vec<String>,
}
impl std::fmt::Debug for Operation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Operation { redacted }")
    }
}
