//! Generated native contract fields; validation/admission remain separate.
use super::*;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Contract enum `NativeSourceHostReplyKind`.
pub enum NativeSourceHostReplyKind {
    /// Contract value `opened`.
    #[serde(rename = "opened")]
    Opened,
    /// Contract value `source`.
    #[serde(rename = "source")]
    Source,
    /// Contract value `snapshot_ref`.
    #[serde(rename = "snapshot_ref")]
    SnapshotRef,
    /// Contract value `preview`.
    #[serde(rename = "preview")]
    Preview,
    /// Contract value `commit_plan`.
    #[serde(rename = "commit_plan")]
    CommitPlan,
    /// Contract value `state`.
    #[serde(rename = "state")]
    State,
    /// Contract value `editor_updated`.
    #[serde(rename = "editor_updated")]
    EditorUpdated,
}
