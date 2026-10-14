//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `NativeModelCodecHeaders`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeModelCodecHeaders {
    /// Contract field `type`.
    pub r#type: NativeModelCodecHeadersType,
    /// Contract field `runtime_instance_id`.
    pub runtime_instance_id: String,
    /// Contract field `codec_ref`.
    pub codec_ref: String,
    /// Contract field `status`.
    pub status: u32,
}
impl std::fmt::Debug for NativeModelCodecHeaders {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NativeModelCodecHeaders { redacted }")
    }
}
