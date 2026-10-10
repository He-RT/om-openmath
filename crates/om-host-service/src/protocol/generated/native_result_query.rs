//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Typed native union `NativeResultQuery`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum NativeResultQuery {
    /// Variant `InspectResultSummary`.
    InspectResultSummary(InspectResultSummary),
    /// Variant `InspectResultPage`.
    InspectResultPage(InspectResultPage),
    /// Variant `InspectResultSource`.
    InspectResultSource(InspectResultSource),
    /// Variant `InspectResultNumeric`.
    InspectResultNumeric(InspectResultNumeric),
    /// Variant `InspectResultSteps`.
    InspectResultSteps(InspectResultSteps),
    /// Variant `InspectResultGeometry`.
    InspectResultGeometry(InspectResultGeometry),
    /// Variant `InspectResultGeometryPage`.
    InspectResultGeometryPage(InspectResultGeometryPage),
    /// Variant `InspectResultExpression`.
    InspectResultExpression(InspectResultExpression),
    /// Variant `InspectResultScratch`.
    InspectResultScratch(InspectResultScratch),
    /// Variant `InspectResultPresentation`.
    InspectResultPresentation(InspectResultPresentation),
}
