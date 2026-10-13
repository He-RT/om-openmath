//! Generated native contract fields; validation/admission remain separate.
use super::*;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Contract enum `AgentCancelPayloadReason`.
pub enum AgentCancelPayloadReason {
    /// Contract value `user_stop`.
    #[serde(rename = "user_stop")]
    UserStop,
    /// Contract value `document_closed`.
    #[serde(rename = "document_closed")]
    DocumentClosed,
    /// Contract value `application_closed`.
    #[serde(rename = "application_closed")]
    ApplicationClosed,
    /// Contract value `superseded`.
    #[serde(rename = "superseded")]
    Superseded,
}
