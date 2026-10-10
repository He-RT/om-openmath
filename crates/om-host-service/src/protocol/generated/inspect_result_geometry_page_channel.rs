//! Generated native contract fields; validation/admission remain separate.
use super::*;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Contract enum `InspectResultGeometryPageChannel`.
pub enum InspectResultGeometryPageChannel {
    /// Contract value `mesh_positions`.
    #[serde(rename = "mesh_positions")]
    MeshPositions,
    /// Contract value `mesh_normals`.
    #[serde(rename = "mesh_normals")]
    MeshNormals,
    /// Contract value `mesh_colors`.
    #[serde(rename = "mesh_colors")]
    MeshColors,
    /// Contract value `mesh_triangles`.
    #[serde(rename = "mesh_triangles")]
    MeshTriangles,
    /// Contract value `line_positions`.
    #[serde(rename = "line_positions")]
    LinePositions,
    /// Contract value `line_colors`.
    #[serde(rename = "line_colors")]
    LineColors,
    /// Contract value `scene_points`.
    #[serde(rename = "scene_points")]
    ScenePoints,
    /// Contract value `scene_labels`.
    #[serde(rename = "scene_labels")]
    SceneLabels,
    /// Contract value `curve_points`.
    #[serde(rename = "curve_points")]
    CurvePoints,
    /// Contract value `plot_tiles`.
    #[serde(rename = "plot_tiles")]
    PlotTiles,
    /// Contract value `plot_arrows`.
    #[serde(rename = "plot_arrows")]
    PlotArrows,
    /// Contract value `plot_points`.
    #[serde(rename = "plot_points")]
    PlotPoints,
    /// Contract value `path_points`.
    #[serde(rename = "path_points")]
    PathPoints,
    /// Contract value `polygon_points`.
    #[serde(rename = "polygon_points")]
    PolygonPoints,
    /// Contract value `plot_markers`.
    #[serde(rename = "plot_markers")]
    PlotMarkers,
    /// Contract value `plot_labels`.
    #[serde(rename = "plot_labels")]
    PlotLabels,
}
