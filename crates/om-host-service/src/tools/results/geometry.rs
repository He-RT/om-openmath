use super::*;
use om_kernel::retained_results::{GeometryChannel as C, RetainedGeometryQuery};

pub(super) fn page(
    result: &ResolvedResult,
    index: usize,
    offset: u32,
    limit: u32,
    ctx: &Interrupt,
) -> Result<(Value, Option<Position>, &'static str), &'static str> {
    let summary = result.geometry(ctx).map_err(projection)?;
    let mut routes = Vec::new();
    let mut add =
        |channel, object: usize, segment: usize, count: u64| -> Result<(), &'static str> {
            tick(ctx)?;
            if count > 0 {
                if routes.len() >= 10000 {
                    return Err("LIMIT_EXCEEDED");
                }
                routes.push(RetainedGeometryQuery {
                    channel,
                    object_index: u32::try_from(object).map_err(|_| "LIMIT_EXCEEDED")?,
                    segment_index: u32::try_from(segment).map_err(|_| "LIMIT_EXCEEDED")?,
                    offset: 0,
                    limit,
                });
            }
            Ok(())
        };
    let m = &summary.metadata;
    if summary.available {
        match summary.kind.as_str() {
            "scene3d" => {
                for (i, mesh) in m["meshes"]
                    .as_array()
                    .ok_or("NOT_AVAILABLE")?
                    .iter()
                    .enumerate()
                {
                    for (field, channel) in [
                        ("vertices", C::MeshPositions),
                        ("normals", C::MeshNormals),
                        ("colors", C::MeshColors),
                        ("triangles", C::MeshTriangles),
                    ] {
                        add(channel, i, 0, mesh[field].as_u64().ok_or("NOT_AVAILABLE")?)?;
                    }
                }
                for (i, line) in m["lines"]
                    .as_array()
                    .ok_or("NOT_AVAILABLE")?
                    .iter()
                    .enumerate()
                {
                    for (field, channel) in
                        [("positions", C::LinePositions), ("colors", C::LineColors)]
                    {
                        add(channel, i, 0, line[field].as_u64().ok_or("NOT_AVAILABLE")?)?;
                    }
                }
                add(
                    C::ScenePoints,
                    0,
                    0,
                    m["points"].as_u64().ok_or("NOT_AVAILABLE")?,
                )?;
                add(
                    C::SceneLabels,
                    0,
                    0,
                    m["labels"].as_u64().ok_or("NOT_AVAILABLE")?,
                )?;
            }
            "plot2d" => {
                for (i, curve) in m["curves"]
                    .as_array()
                    .ok_or("NOT_AVAILABLE")?
                    .iter()
                    .enumerate()
                {
                    for (j, count) in curve["segments"]
                        .as_array()
                        .ok_or("NOT_AVAILABLE")?
                        .iter()
                        .enumerate()
                    {
                        add(C::CurvePoints, i, j, count.as_u64().ok_or("NOT_AVAILABLE")?)?;
                    }
                }
                for (field, channel) in [
                    ("tiles", C::PlotTiles),
                    ("arrows", C::PlotArrows),
                    ("points", C::PlotPoints),
                    ("markers", C::PlotMarkers),
                    ("labels", C::PlotLabels),
                ] {
                    add(channel, 0, 0, m[field].as_u64().ok_or("NOT_AVAILABLE")?)?;
                }
                for (field, channel) in [("paths", C::PathPoints), ("polygons", C::PolygonPoints)] {
                    if let Some(items) = m[field].as_array() {
                        for (i, item) in items.iter().enumerate() {
                            add(
                                channel,
                                i,
                                0,
                                item["points"].as_u64().ok_or("NOT_AVAILABLE")?,
                            )?;
                        }
                    }
                }
            }
            _ => return Err("NOT_AVAILABLE"),
        }
    }
    if routes.is_empty() {
        if index != 0 || offset != 0 {
            return Err("INVALID_REFERENCE");
        }
        return Ok((
            json!({"binding":result.binding(),"summary":summary,"page":null}),
            None,
            if summary.available {
                "data_only"
            } else {
                "unavailable"
            },
        ));
    }
    let mut query = routes.get(index).ok_or("INVALID_REFERENCE")?.clone();
    query.offset = offset;
    let data = result.geometry_page(&query, ctx).map_err(projection)?;
    let count = data.values.as_array().ok_or("NOT_AVAILABLE")?.len();
    let end = (offset as usize)
        .checked_add(count)
        .ok_or("LIMIT_EXCEEDED")?;
    let next = if end < data.total {
        if count == 0 {
            return Err("NOT_AVAILABLE");
        }
        Some(Position::Geometry {
            route: index,
            offset: u32::try_from(end).map_err(|_| "LIMIT_EXCEEDED")?,
        })
    } else if index + 1 < routes.len() {
        Some(Position::Geometry {
            route: index + 1,
            offset: 0,
        })
    } else {
        None
    };
    Ok((
        json!({"binding":result.binding(),"summary":summary,"page":data}),
        next,
        "data_only",
    ))
}
