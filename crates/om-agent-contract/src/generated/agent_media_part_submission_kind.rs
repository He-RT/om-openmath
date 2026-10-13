//! Generated native contract fields; validation/admission remain separate.
use super::*;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Contract enum `AgentMediaPartSubmissionKind`.
pub enum AgentMediaPartSubmissionKind {
    /// Contract value `image`.
    #[serde(rename = "image")]
    Image,
    /// Contract value `audio`.
    #[serde(rename = "audio")]
    Audio,
    /// Contract value `video`.
    #[serde(rename = "video")]
    Video,
    /// Contract value `file`.
    #[serde(rename = "file")]
    File,
}
