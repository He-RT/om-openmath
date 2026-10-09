//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `NativePreviewArgs`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativePreviewArgs {
    /// Contract field `snapshot_ref`.
    pub snapshot_ref: String,
    /// Contract field `input`.
    pub input: NativePreviewInput,
}
impl std::fmt::Debug for NativePreviewArgs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NativePreviewArgs { redacted }")
    }
}
