//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `InspectResultExpression`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InspectResultExpression {
    /// Contract field `kind`.
    pub kind: InspectResultExpressionKind,
    /// Contract field `source`.
    pub source: String,
    /// Contract field `numeric`.
    pub numeric: bool,
}
impl std::fmt::Debug for InspectResultExpression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("InspectResultExpression { redacted }")
    }
}
