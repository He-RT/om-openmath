//! Three-dimensional sampled geometry is separate from the legacy two-dimensional protocol.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use ts_rs::TS;
/// Mathematical scene sampling family.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export_to=concat!(env!("CARGO_MANIFEST_DIR"),"/../../app/src/kernel/generated/"))]
#[serde(rename_all = "snake_case")]
pub enum SceneKind {
    /// z=f(x,y).
    Surface,
    /// One parameter curve or two parameter surface, with three coordinate expressions.
    Parametric,
    /// A scalar zero set in three actual coordinates.
    Implicit,
    /// Composed mathematical primitives and sampled child scenes.
    Scene,
}
/// Held portable mathematical request; the renderer never interprets these source expressions.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export_to=concat!(env!("CARGO_MANIFEST_DIR"),"/../../app/src/kernel/generated/"))]
pub struct Scene3DRequest {
    /// Actual mathematical family.
    pub kind: SceneKind,
    /// Wolfram raw source coordinate/scalar expressions.
    pub expressions: Vec<String>,
    /// Explicit immutable coordinate/parameter domains.
    pub axes: Vec<crate::plot_views::PlotAxis>,
    /// Uniform subdivisions per parameter or coordinate.
    pub mesh_points: u32,
    /// Fixed color string or a readonly fn(position, ...parameters) source.
    pub color: Option<String>,
    /// Finite local parameter substitutions; never written to the notebook.
    #[serde(default)]
    pub parameters: BTreeMap<String, f64>,
}
/// One real indexed surface mesh.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export_to=concat!(env!("CARGO_MANIFEST_DIR"),"/../../app/src/kernel/generated/"))]
pub struct SceneMesh {
    /// Original finite world-space coordinates.
    pub positions: Vec<[f64; 3]>,
    /// Unit normals computed from actual finite triangles.
    pub normals: Vec<[f64; 3]>,
    /// Actual kernel-produced sRGB/alpha components in [0,1].
    pub colors: Vec<[f64; 4]>,
    /// Actual counterclockwise vertex indices; no frontend triangulation.
    pub triangles: Vec<[u32; 3]>,
    /// Mathematical source description.
    pub label: String,
}
/// One continuous sampled curve or scene polyline; discontinuities are separate objects.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export_to=concat!(env!("CARGO_MANIFEST_DIR"),"/../../app/src/kernel/generated/"))]
pub struct SceneLine {
    /// Original finite coordinates, in order.
    pub positions: Vec<[f64; 3]>,
    /// Actual point colors.
    pub colors: Vec<[f64; 4]>,
    /// Logical pixel line width, subject to actual renderer support.
    pub width: f64,
}
/// A real point marker (including a constant parametric locus).
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export_to=concat!(env!("CARGO_MANIFEST_DIR"),"/../../app/src/kernel/generated/"))]
pub struct ScenePoint {
    /// Actual coordinate.
    pub position: [f64; 3],
    /// Actual color.
    pub color: [f64; 4],
    /// Logical pixel marker radius.
    pub radius: f64,
}
/// Genuine mesh/curve output; sampling approximations are never described as certified boundaries.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export_to=concat!(env!("CARGO_MANIFEST_DIR"),"/../../app/src/kernel/generated/"))]
pub struct Scene3DData {
    /// Actual surface geometry.
    pub meshes: Vec<SceneMesh>,
    /// Actual sampled continuous curves.
    pub lines: Vec<SceneLine>,
    /// Actual point loci/markers.
    pub points: Vec<ScenePoint>,
    /// Finite original-coordinate bounding corners.
    pub bounds: ([f64; 3], [f64; 3]),
    /// Real undefined/discontinuous samples or omitted grid cells.
    pub skipped: u32,
    /// True: finite sampling is not an exact/complete surface certificate.
    pub sampled: bool,
    /// Real text at mathematical positions, not a remote image or HTML instruction.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[ts(optional, as = "Option<Vec<SceneLabel>>")]
    pub labels: Vec<SceneLabel>,
}

/// Real mathematical label in original coordinates.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export_to=concat!(env!("CARGO_MANIFEST_DIR"),"/../../app/src/kernel/generated/"))]
pub struct SceneLabel {
    /// Exact sampled label position in the world frame.
    pub position: [f64; 3],
    /// Plain Unicode text, never executed as source or markup.
    pub text: String,
    /// Actual fixed RGBA.
    pub color: [f64; 4],
    /// Display-only pixel offset from the mathematical anchor.
    pub offset: (f64, f64),
}
