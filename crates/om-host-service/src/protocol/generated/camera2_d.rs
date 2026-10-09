//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `Camera2D`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Camera2D {
    /// Contract field `record_type`.
    pub record_type: Camera2DRecordType,
    /// Contract field `result_id`.
    pub result_id: String,
    /// Contract field `producer_id`.
    pub producer_id: String,
    /// Contract field `camera_generation`.
    pub camera_generation: Serial,
    /// Contract field `sample_generation`.
    pub sample_generation: Serial,
    /// Contract field `x_range`.
    pub x_range: Vec<f64>,
    /// Contract field `y_range`.
    pub y_range: Vec<f64>,
    /// Contract field `scale`.
    pub scale: Camera2DScale,
    /// Contract field `parameter_domain_hash`.
    pub parameter_domain_hash: String,
    /// Contract field `changes_math_domain`.
    pub changes_math_domain: bool,
}
impl std::fmt::Debug for Camera2D {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Camera2D { redacted }")
    }
}
