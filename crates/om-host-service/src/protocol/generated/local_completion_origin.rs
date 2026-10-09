//! Generated native contract fields; validation/admission remain separate.
use super::*;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Contract enum `LocalCompletionOrigin`.
pub enum LocalCompletionOrigin {
    /// Contract value `builtin`.
    #[serde(rename = "builtin")]
    Builtin,
    /// Contract value `user_definition`.
    #[serde(rename = "user_definition")]
    UserDefinition,
    /// Contract value `parameter`.
    #[serde(rename = "parameter")]
    Parameter,
    /// Contract value `greek_shortcut`.
    #[serde(rename = "greek_shortcut")]
    GreekShortcut,
}
