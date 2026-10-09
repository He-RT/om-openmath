//! Generated native contract fields; validation/admission remain separate.
use super::*;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Contract enum `Camera2DScale`.
pub enum Camera2DScale {
    /// Contract value `linear`.
    #[serde(rename = "linear")]
    Linear,
    /// Contract value `log_x`.
    #[serde(rename = "log_x")]
    LogX,
    /// Contract value `log_y`.
    #[serde(rename = "log_y")]
    LogY,
    /// Contract value `log_log`.
    #[serde(rename = "log_log")]
    LogLog,
}
