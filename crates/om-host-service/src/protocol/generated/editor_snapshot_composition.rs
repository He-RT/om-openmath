//! Generated native contract fields; validation/admission remain separate.
use super::*;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Contract enum `EditorSnapshotComposition`.
pub enum EditorSnapshotComposition {
    /// Contract value `none`.
    #[serde(rename = "none")]
    None,
    /// Contract value `marked`.
    #[serde(rename = "marked")]
    Marked,
}
