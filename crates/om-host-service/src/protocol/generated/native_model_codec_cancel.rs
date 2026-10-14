//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `NativeModelCodecCancel`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeModelCodecCancel {
    /// Contract field `type`.
    pub r#type: NativeModelCodecCancelType,
    /// Contract field `runtime_instance_id`.
    pub runtime_instance_id: String,
    /// Contract field `codec_ref`.
    pub codec_ref: String,
}
impl std::fmt::Debug for NativeModelCodecCancel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NativeModelCodecCancel { redacted }")
    }
}
