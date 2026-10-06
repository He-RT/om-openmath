//! Typed pure artifact requests; actual filesystem persistence remains with each host.
use serde::{Deserialize, Serialize};
use ts_rs::TS;
/// Two-dimensional vector or raster export.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, TS)]
#[ts(export_to=concat!(env!("CARGO_MANIFEST_DIR"),"/../../app/src/kernel/generated/"))]
#[serde(rename_all = "snake_case")]
pub enum PlotExportFormat {
    /// Self-contained vector geometry and original metadata.
    Svg,
    /// Lossless pixels with UTF-8 metadata.
    Png,
}
/// Pure data export, never a source evaluator.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, TS)]
#[ts(export_to=concat!(env!("CARGO_MANIFEST_DIR"),"/../../app/src/kernel/generated/"))]
#[serde(rename_all = "snake_case")]
pub enum DataExportFormat {
    /// Ordered tabular fields.
    Csv,
    /// Pure structured values.
    Json,
}
/// Actual already-sampled mathematical data and display options, without duplicated sample requests.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export_to=concat!(env!("CARGO_MANIFEST_DIR"),"/../../app/src/kernel/generated/"))]
pub struct PlotFigure {
    /// Actual kernel geometry in the current successful viewport.
    pub data: crate::views::PlotData,
    /// Horizontal mathematical coordinate name (x for a parametric curve).
    pub axis_x: String,
    /// Vertical mathematical coordinate name.
    pub axis_y: String,
    /// User-visible figure title.
    pub title: String,
    /// Checked fixed curve/arrow/point color, when specified.
    pub color: Option<String>,
    /// The region grid is a midpoint sampling approximation.
    pub region: bool,
    /// Actual successful sampling/slider parameters, kept in figure metadata.
    #[serde(default)]
    pub parameters: std::collections::BTreeMap<String, f64>,
    /// 320..2048 logical pixels.
    pub width: u32,
    /// 240..2048 logical pixels; total pixels <=2,000,000.
    pub height: u32,
}
/// Concrete bytes, not a claim that the host has saved a file.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export_to=concat!(env!("CARGO_MANIFEST_DIR"),"/../../app/src/kernel/generated/"))]
pub struct Artifact {
    /// Actual data MIME type.
    pub mime: String,
    /// Checked file extension.
    pub extension: String,
    /// RFC4648 base64 preserving exact bytes across JSON/Swift/JS.
    pub base64: String,
    /// Actual raw byte length, checked by hosts before saving.
    pub byte_len: u32,
}
