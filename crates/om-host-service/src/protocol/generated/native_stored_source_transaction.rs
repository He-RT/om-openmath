//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `NativeStoredSourceTransaction`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeStoredSourceTransaction {
    /// Contract field `store_id`.
    pub store_id: String,
    /// Contract field `transaction_id`.
    pub transaction_id: String,
    /// Contract field `plan`.
    pub plan: NativeSourceCommit,
    /// Contract field `receipt`.
    pub receipt: NativeDurableSourceReceipt,
}
impl std::fmt::Debug for NativeStoredSourceTransaction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NativeStoredSourceTransaction { redacted }")
    }
}
