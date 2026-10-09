//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `PreviewNewCell`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreviewNewCell {
    /// Contract field `client_key`.
    pub client_key: String,
}
impl std::fmt::Debug for PreviewNewCell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PreviewNewCell { redacted }")
    }
}
