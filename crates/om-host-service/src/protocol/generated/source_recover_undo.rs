//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `SourceRecoverUndo`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceRecoverUndo {
    /// Contract field `type`.
    pub r#type: SourceRecoverUndoType,
    /// Contract field `receipt`.
    pub receipt: NativeDurableSourceReceipt,
}
impl std::fmt::Debug for SourceRecoverUndo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SourceRecoverUndo { redacted }")
    }
}
