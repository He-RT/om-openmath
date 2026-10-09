//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `NativeSourceFile`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeSourceFile {
    /// Contract field `version`.
    pub version: u32,
    /// Contract field `title`.
    pub title: String,
    /// Contract field `cells`.
    pub cells: Vec<NativeSourceCell>,
}
impl std::fmt::Debug for NativeSourceFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NativeSourceFile { redacted }")
    }
}
