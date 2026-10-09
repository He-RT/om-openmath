//! Generated native contract fields; validation/admission remain separate.
use super::*;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Contract enum `OperationReceiptDetailsRetention`.
pub enum OperationReceiptDetailsRetention {
    /// Contract value `full`.
    #[serde(rename = "full")]
    Full,
    /// Contract value `tombstone`.
    #[serde(rename = "tombstone")]
    Tombstone,
}
