//! Generated native contract fields; validation/admission remain separate.
use super::*;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Contract enum `InspectResultSourceFormat`.
pub enum InspectResultSourceFormat {
    /// Contract value `input_form`.
    #[serde(rename = "input_form")]
    InputForm,
    /// Contract value `modern`.
    #[serde(rename = "modern")]
    Modern,
    /// Contract value `latex`.
    #[serde(rename = "latex")]
    Latex,
    /// Contract value `cell_source`.
    #[serde(rename = "cell_source")]
    CellSource,
    /// Contract value `statement_input`.
    #[serde(rename = "statement_input")]
    StatementInput,
}
