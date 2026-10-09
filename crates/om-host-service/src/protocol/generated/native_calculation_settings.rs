//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `NativeCalculationSettings`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeCalculationSettings {
    /// Contract field `dialect`.
    pub dialect: NativeCalculationSettingsDialect,
    /// Contract field `constants`.
    pub constants: NativeCalculationSettingsConstants,
    /// Contract field `reactive`.
    pub reactive: bool,
    /// Contract field `auto_run_dependents`.
    pub auto_run_dependents: bool,
    /// Contract field `show_steps`.
    pub show_steps: bool,
    /// Contract field `auto_plot`.
    pub auto_plot: bool,
    /// Contract field `eval_timeout_ms`.
    pub eval_timeout_ms: Serial,
}
impl std::fmt::Debug for NativeCalculationSettings {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NativeCalculationSettings { redacted }")
    }
}
