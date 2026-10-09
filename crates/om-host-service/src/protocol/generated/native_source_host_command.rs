//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Typed native union `NativeSourceHostCommand`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum NativeSourceHostCommand {
    /// Variant `SourceOpen`.
    SourceOpen(SourceOpen),
    /// Variant `SourceRead`.
    SourceRead(SourceRead),
    /// Variant `SourceSnapshotRef`.
    SourceSnapshotRef(SourceSnapshotRef),
    /// Variant `SourcePreview`.
    SourcePreview(SourcePreview),
    /// Variant `SourceBegin`.
    SourceBegin(SourceBegin),
    /// Variant `SourceAdmitted`.
    SourceAdmitted(SourceAdmitted),
    /// Variant `SourceFenced`.
    SourceFenced(SourceFenced),
    /// Variant `SourceBarrier`.
    SourceBarrier(SourceBarrier),
    /// Variant `SourceUnknown`.
    SourceUnknown(SourceUnknown),
    /// Variant `SourceCompleted`.
    SourceCompleted(SourceCompleted),
    /// Variant `SourceSettled`.
    SourceSettled(SourceSettled),
    /// Variant `SourceStatus`.
    SourceStatus(SourceStatus),
    /// Variant `SourceEditorChanged`.
    SourceEditorChanged(SourceEditorChanged),
    /// Variant `SourcePrepareManual`.
    SourcePrepareManual(SourcePrepareManual),
}
