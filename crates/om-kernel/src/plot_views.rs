//! Additive two-dimensional options and actual sampled geometry; no third coordinate in legacy curves.
use serde::{Deserialize, Serialize};
use ts_rs::TS;
/// Display axis transform, distinct from a mathematical parameter domain.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export_to=concat!(env!("CARGO_MANIFEST_DIR"),"/../../app/src/kernel/generated/"))]
#[serde(rename_all = "snake_case")]
pub enum PlotScale {
    /// Ordinary axes.
    #[default]
    Linear,
    /// Positive horizontal logarithmic axis.
    LogX,
    /// Positive vertical logarithmic axis.
    LogY,
    /// Both positive logarithmic axes.
    LogLog,
}
impl PlotScale {
    pub(crate) fn log_x(self) -> bool {
        matches!(self, Self::LogX | Self::LogLog)
    }
    pub(crate) fn log_y(self) -> bool {
        matches!(self, Self::LogY | Self::LogLog)
    }
}
/// Explicit sampling coordinate and fixed mathematical domain.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export_to=concat!(env!("CARGO_MANIFEST_DIR"),"/../../app/src/kernel/generated/"))]
pub struct PlotAxis {
    /// User variable.
    pub name: String,
    /// Mathematical sampling interval.
    pub range: (f64, f64),
}
/// Supported data rendering shape.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export_to=concat!(env!("CARGO_MANIFEST_DIR"),"/../../app/src/kernel/generated/"))]
#[serde(rename_all = "snake_case")]
pub enum DataPlotStyle {
    /// Unconnected data points.
    #[default]
    Scatter,
    /// Ordered input polyline.
    Line,
    /// Rectangular value grid.
    Heatmap,
}
/// Additional real options; absent on legacy requests.
#[derive(Clone, Debug, Default, Serialize, Deserialize, TS)]
#[ts(export_to=concat!(env!("CARGO_MANIFEST_DIR"),"/../../app/src/kernel/generated/"))]
pub struct PlotOptions2D {
    /// Mathematical coordinate domains, required for parameterized sampling.
    pub axes: Vec<PlotAxis>,
    /// Axis transform.
    pub scale: PlotScale,
    /// Data presentation shape.
    pub style: DataPlotStyle,
    /// Checked finite points/grid values captured by the kernel; never evaluated by the renderer.
    pub samples: Vec<Vec<f64>>,
    /// Vector field arrows or streamlines.
    pub stream: bool,
    /// Actual positive bin count.
    pub bins: u32,
    /// Actual requested scalar contour levels (empty means zero).
    pub levels: Vec<f64>,
    /// Checked fixed two-dimensional color, if explicitly requested.
    pub color: Option<String>,
}
/// One finite colored sampled rectangle (region/density/heatmap/histogram).
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export_to=concat!(env!("CARGO_MANIFEST_DIR"),"/../../app/src/kernel/generated/"))]
pub struct PlotTile {
    /// Physical lower/upper corners.
    pub bounds: ((f64, f64), (f64, f64)),
    /// Actual sampled value/count.
    pub value: f64,
    /// Kernel-generated sRGB color.
    pub color: String,
}
/// Actual vector sample and kernel-derived visible arrow endpoints.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export_to=concat!(env!("CARGO_MANIFEST_DIR"),"/../../app/src/kernel/generated/"))]
pub struct PlotArrow {
    /// Actual sample coordinate.
    pub start: (f64, f64),
    /// Visible endpoint after documented common grid normalization.
    pub end: (f64, f64),
    /// Original vector value before visual normalization.
    pub value: (f64, f64),
}
/// All geometry comes from the numerical kernel, not frontend expressions.
#[derive(Clone, Debug, Default, Serialize, Deserialize, TS)]
#[ts(export_to=concat!(env!("CARGO_MANIFEST_DIR"),"/../../app/src/kernel/generated/"))]
pub struct PlotGeometry2D {
    /// Sampled area/value cells.
    pub tiles: Vec<PlotTile>,
    /// Vector samples.
    pub arrows: Vec<PlotArrow>,
    /// Unconnected data samples.
    pub points: Vec<(f64, f64)>,
    /// Nonfinite/undefined samples skipped, explicitly counted.
    pub skipped: u32,
    /// Actual finite color value range when applicable.
    pub color_range: Option<(f64, f64)>,
}
