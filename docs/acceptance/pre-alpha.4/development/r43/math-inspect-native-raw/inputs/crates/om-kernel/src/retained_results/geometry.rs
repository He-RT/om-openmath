use super::*;
use om_core::Interrupt;
use serde_json::{Value, json};
/// Closed raw geometry channels. All data comes from the original kernel sampling result.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GeometryChannel {
    /// Mesh vertex positions.
    MeshPositions,
    /// Mesh normals.
    MeshNormals,
    /// Mesh colors.
    MeshColors,
    /// Original global vertex indices, no frontend re-triangulation.
    MeshTriangles,
    /// Three-dimensional line positions.
    LinePositions,
    /// Three-dimensional line colors.
    LineColors,
    /// Original point markers.
    ScenePoints,
    /// Original plain labels.
    SceneLabels,
    /// Original legacy 2D curve segment samples.
    CurvePoints,
    /// Actual sampled region/density/histogram tiles.
    PlotTiles,
    /// Original vector samples/endpoints.
    PlotArrows,
    /// Original unconnected 2D points.
    PlotPoints,
    /// Actual composed path samples.
    PathPoints,
    /// Original filled polygon boundaries.
    PolygonPoints,
    /// Original sized markers.
    PlotMarkers,
    /// Original mathematical text labels.
    PlotLabels,
}
/// One bounded actual geometry coordinate request.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RetainedGeometryQuery {
    /// Exact original channel.
    pub channel: GeometryChannel,
    /// Object index within an immutable output.
    pub object_index: u32,
    /// Discontinuous legacy curve segment index.
    pub segment_index: u32,
    /// First original item.
    pub offset: u32,
    /// 1..1024 original items.
    pub limit: u32,
}
/// Small real geometry counts/bounds. Data-only/unavailable is distinct from sampled geometry.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RetainedGeometrySummary {
    /// Actual output dimension/family, not guessed from source spelling.
    pub kind: String,
    /// True only when real geometry was produced and retained.
    pub available: bool,
    /// Small structured counts/ranges/styles, never point or triangle arrays.
    pub metadata: Value,
    /// Explicit absence/unsupported host reason.
    pub unavailable: Option<String>,
}
/// One raw original geometry slice, still associated with its producer occurrence.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RetainedGeometryPage {
    /// Original occurrence.
    pub view_id: String,
    /// Exact channel requested.
    pub channel: GeometryChannel,
    /// Mesh/line/curve/path/polygon ordinal inside this immutable output.
    pub object_index: u32,
    /// Segment ordinal inside a discontinuous 2D curve.
    pub segment_index: u32,
    /// Actual complete channel length.
    pub total: usize,
    /// Exact offset into the original raw channel.
    pub offset: u32,
    /// Original selected raw coordinates/indices/colors, without any function evaluation.
    pub values: Value,
}
fn page<T: Serialize>(items: &[T], offset: u32, limit: u32) -> Result<(usize, Value), String> {
    if offset as usize > items.len() {
        return Err("INVALID_GEOMETRY_OFFSET".into());
    }
    let end = (offset as usize)
        .checked_add(limit as usize)
        .ok_or("GEOMETRY_BUDGET_EXCEEDED")?
        .min(items.len());
    Ok((
        items.len(),
        serde_json::to_value(&items[offset as usize..end]).map_err(|_| "INVALID_GEOMETRY_DATA")?,
    ))
}
impl RetainedResult {
    /// Read original geometry counts/availability; never sample source because a view is opened.
    pub fn geometry_summary(
        &self,
        out_index: u32,
        view_id: &str,
        ctx: &Interrupt,
    ) -> Result<RetainedGeometrySummary, String> {
        ctx.tick().map_err(|e| e.to_string())?;
        match self.geometry_output(out_index, view_id)? {
            Some(OutputItem::Scene3D {
                data: Some(data), ..
            }) => Ok(RetainedGeometrySummary {
                kind: "scene3d".into(),
                available: true,
                metadata: json!({"bounds":data.bounds,"sampled":data.sampled,"skipped":data.skipped,"meshes":data.meshes.iter().map(|m|json!({"vertices":m.positions.len(),"normals":m.normals.len(),"colors":m.colors.len(),"triangles":m.triangles.len(),"label":m.label})).collect::<Vec<_>>(),"lines":data.lines.iter().map(|l|json!({"positions":l.positions.len(),"colors":l.colors.len(),"width":l.width})).collect::<Vec<_>>(),"points":data.points.len(),"labels":data.labels.len()}),
                unavailable: None,
            }),
            Some(OutputItem::Scene3D {
                data: None,
                unavailable,
                ..
            }) => Ok(RetainedGeometrySummary {
                kind: "scene3d".into(),
                available: false,
                metadata: json!({}),
                unavailable: Some(
                    unavailable
                        .clone()
                        .unwrap_or_else(|| "Geometry has not been sampled".into()),
                ),
            }),
            Some(OutputItem::Plot { data, .. }) => {
                let geometry = data.geometry.as_ref();
                Ok(RetainedGeometrySummary {
                    kind: "plot2d".into(),
                    available: true,
                    metadata: json!({"x_range":data.x_range,"y_range":data.y_range,"scale":data.scale,"curves":data.curves.iter().map(|c|json!({"label":c.label,"segments":c.segments.iter().map(Vec::len).collect::<Vec<_>>()})).collect::<Vec<_>>(),"tiles":geometry.map_or(0,|g|g.tiles.len()),"arrows":geometry.map_or(0,|g|g.arrows.len()),"points":geometry.map_or(0,|g|g.points.len()),"markers":geometry.map_or(0,|g|g.markers.len()),"labels":geometry.map_or(0,|g|g.labels.len()),"paths":geometry.map(|g|g.paths.iter().map(|p|json!({"points":p.points.len(),"color":p.color,"opacity":p.opacity,"width":p.width})).collect::<Vec<_>>()),"polygons":geometry.map(|g|g.polygons.iter().map(|p|json!({"points":p.points.len(),"color":p.color,"opacity":p.opacity})).collect::<Vec<_>>()),"skipped":geometry.map_or(0,|g|g.skipped),"color_range":geometry.and_then(|g|g.color_range)}),
                    unavailable: None,
                })
            }
            Some(OutputItem::Solutions { plot: Some(_), .. }) => Ok(RetainedGeometrySummary {
                kind: "plot2d".into(),
                available: false,
                metadata: json!({}),
                unavailable: Some(
                    "Automatic plot request exists; geometry has not been sampled".into(),
                ),
            }),
            _ => Ok(RetainedGeometrySummary {
                kind: "none".into(),
                available: false,
                metadata: json!({}),
                unavailable: Some("This output has no retained geometry".into()),
            }),
        }
    }
    /// Bounded original geometry data route. Invalid owner/view/channel/object never samples anew.
    pub fn geometry_page(
        &self,
        out_index: u32,
        view_id: &str,
        query: &RetainedGeometryQuery,
        ctx: &Interrupt,
    ) -> Result<RetainedGeometryPage, String> {
        let RetainedGeometryQuery {
            channel,
            object_index,
            segment_index,
            offset,
            limit,
        } = *query;
        if limit == 0 || limit > 1024 {
            return Err("GEOMETRY_BUDGET_EXCEEDED".into());
        }
        ctx.tick().map_err(|e| e.to_string())?;
        let (total, values) = match self.geometry_output(out_index, view_id)? {
            Some(OutputItem::Scene3D {
                data: Some(data), ..
            }) => {
                if segment_index != 0 {
                    return Err("INVALID_GEOMETRY_SEGMENT".into());
                }
                match channel {
                    GeometryChannel::MeshPositions => page(
                        &data
                            .meshes
                            .get(object_index as usize)
                            .ok_or("INVALID_GEOMETRY_OBJECT")?
                            .positions,
                        offset,
                        limit,
                    )?,
                    GeometryChannel::MeshNormals => page(
                        &data
                            .meshes
                            .get(object_index as usize)
                            .ok_or("INVALID_GEOMETRY_OBJECT")?
                            .normals,
                        offset,
                        limit,
                    )?,
                    GeometryChannel::MeshColors => page(
                        &data
                            .meshes
                            .get(object_index as usize)
                            .ok_or("INVALID_GEOMETRY_OBJECT")?
                            .colors,
                        offset,
                        limit,
                    )?,
                    GeometryChannel::MeshTriangles => page(
                        &data
                            .meshes
                            .get(object_index as usize)
                            .ok_or("INVALID_GEOMETRY_OBJECT")?
                            .triangles,
                        offset,
                        limit,
                    )?,
                    GeometryChannel::LinePositions => page(
                        &data
                            .lines
                            .get(object_index as usize)
                            .ok_or("INVALID_GEOMETRY_OBJECT")?
                            .positions,
                        offset,
                        limit,
                    )?,
                    GeometryChannel::LineColors => page(
                        &data
                            .lines
                            .get(object_index as usize)
                            .ok_or("INVALID_GEOMETRY_OBJECT")?
                            .colors,
                        offset,
                        limit,
                    )?,
                    GeometryChannel::ScenePoints if object_index == 0 => {
                        page(&data.points, offset, limit)?
                    }
                    GeometryChannel::SceneLabels if object_index == 0 => {
                        page(&data.labels, offset, limit)?
                    }
                    _ => return Err("INVALID_GEOMETRY_CHANNEL".into()),
                }
            }
            Some(OutputItem::Plot { data, .. }) => {
                if channel == GeometryChannel::CurvePoints {
                    page(
                        data.curves
                            .get(object_index as usize)
                            .ok_or("INVALID_GEOMETRY_OBJECT")?
                            .segments
                            .get(segment_index as usize)
                            .ok_or("INVALID_GEOMETRY_SEGMENT")?,
                        offset,
                        limit,
                    )?
                } else {
                    if segment_index != 0 {
                        return Err("INVALID_GEOMETRY_SEGMENT".into());
                    }
                    let geometry = data.geometry.as_ref().ok_or("GEOMETRY_NOT_AVAILABLE")?;
                    match channel {
                        GeometryChannel::PlotTiles if object_index == 0 => {
                            page(&geometry.tiles, offset, limit)?
                        }
                        GeometryChannel::PlotArrows if object_index == 0 => {
                            page(&geometry.arrows, offset, limit)?
                        }
                        GeometryChannel::PlotPoints if object_index == 0 => {
                            page(&geometry.points, offset, limit)?
                        }
                        GeometryChannel::PlotMarkers if object_index == 0 => {
                            page(&geometry.markers, offset, limit)?
                        }
                        GeometryChannel::PlotLabels if object_index == 0 => {
                            page(&geometry.labels, offset, limit)?
                        }
                        GeometryChannel::PathPoints => page(
                            &geometry
                                .paths
                                .get(object_index as usize)
                                .ok_or("INVALID_GEOMETRY_OBJECT")?
                                .points,
                            offset,
                            limit,
                        )?,
                        GeometryChannel::PolygonPoints => page(
                            &geometry
                                .polygons
                                .get(object_index as usize)
                                .ok_or("INVALID_GEOMETRY_OBJECT")?
                                .points,
                            offset,
                            limit,
                        )?,
                        _ => return Err("INVALID_GEOMETRY_CHANNEL".into()),
                    }
                }
            }
            _ => return Err("GEOMETRY_NOT_AVAILABLE".into()),
        };
        Ok(RetainedGeometryPage {
            view_id: view_id.into(),
            channel,
            object_index,
            segment_index,
            total,
            offset,
            values,
        })
    }
}
