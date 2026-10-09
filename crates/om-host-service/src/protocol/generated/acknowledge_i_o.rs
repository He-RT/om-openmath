//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `AcknowledgeIO`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AcknowledgeIO {
    /// Contract field `kind`.
    pub kind: AcknowledgeIOKind,
    /// Contract field `ack`.
    pub ack: IOAck,
}
impl std::fmt::Debug for AcknowledgeIO {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("AcknowledgeIO { redacted }")
    }
}
