//! Generated native contract fields; validation/admission remain separate.
use super::*;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Contract enum `IOAckOutcome`.
pub enum IOAckOutcome {
    /// Contract value `durable`.
    #[serde(rename = "durable")]
    Durable,
    /// Contract value `rejected`.
    #[serde(rename = "rejected")]
    Rejected,
    /// Contract value `unknown`.
    #[serde(rename = "unknown")]
    Unknown,
}
