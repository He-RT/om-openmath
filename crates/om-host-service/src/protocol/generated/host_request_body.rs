//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Typed native union `HostRequestBody`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum HostRequestBody {
    /// Variant `ReadHostState`.
    ReadHostState(ReadHostState),
    /// Variant `ReadHostCapabilities`.
    ReadHostCapabilities(ReadHostCapabilities),
    /// Variant `ReadFunctionCatalog`.
    ReadFunctionCatalog(ReadFunctionCatalog),
    /// Variant `AnalyzeEditor`.
    AnalyzeEditor(AnalyzeEditor),
    /// Variant `EvaluateScratch`.
    EvaluateScratch(EvaluateScratch),
    /// Variant `AcknowledgeIO`.
    AcknowledgeIO(AcknowledgeIO),
    /// Variant `ReadOperationStatus`.
    ReadOperationStatus(ReadOperationStatus),
}
