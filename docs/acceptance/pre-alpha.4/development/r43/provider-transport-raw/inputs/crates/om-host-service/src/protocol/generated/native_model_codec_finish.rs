//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `NativeModelCodecFinish`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeModelCodecFinish {
    /// Contract field `type`.
    pub r#type: NativeModelCodecFinishType,
    /// Contract field `runtime_instance_id`.
    pub runtime_instance_id: String,
    /// Contract field `codec_ref`.
    pub codec_ref: String,
}
impl std::fmt::Debug for NativeModelCodecFinish {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NativeModelCodecFinish { redacted }")
    }
}
