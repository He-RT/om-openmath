//! Generated native contract fields; validation/admission remain separate.
use super::*;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Contract enum `NativeRunDescriptorRunScope`.
pub enum NativeRunDescriptorRunScope {
    /// Contract value `selected`.
    #[serde(rename = "selected")]
    Selected,
    /// Contract value `required`.
    #[serde(rename = "required")]
    Required,
    /// Contract value `affected`.
    #[serde(rename = "affected")]
    Affected,
}
