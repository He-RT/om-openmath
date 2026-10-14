//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `NativeModelCodecFeed`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeModelCodecFeed {
    /// Contract field `type`.
    pub r#type: NativeModelCodecFeedType,
    /// Contract field `runtime_instance_id`.
    pub runtime_instance_id: String,
    /// Contract field `codec_ref`.
    pub codec_ref: String,
    /// Contract field `bytes`.
    pub bytes: Vec<u32>,
}
impl std::fmt::Debug for NativeModelCodecFeed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NativeModelCodecFeed { redacted }")
    }
}
