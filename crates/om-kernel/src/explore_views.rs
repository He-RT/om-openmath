//! Snapshot-bound parameter exploration, separate from document edits and agent sessions.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use ts_rs::TS;
/// An actual checked machine-precision parameter control.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export_to=concat!(env!("CARGO_MANIFEST_DIR"),"/../../app/src/kernel/generated/"))]
pub struct ExploreControl {
    /// Local symbol spelling.
    pub name: String,
    /// Finite increasing slider range.
    pub range: (f64, f64),
    /// Actual initial value.
    pub initial: f64,
}
/// Identity of a retained immutable Explore output.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export_to=concat!(env!("CARGO_MANIFEST_DIR"),"/../../app/src/kernel/generated/"))]
pub struct ExploreQuery {
    /// Current document cell.
    pub cell_id: String,
    /// Retained output index.
    pub out_index: u32,
    /// Opaque producer snapshot ID.
    pub view_id: String,
}
/// Genuine result of one local computation; revision is echoed, never treated as permission.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export_to=concat!(env!("CARGO_MANIFEST_DIR"),"/../../app/src/kernel/generated/"))]
pub struct ExploreResult {
    /// Producer whose immutable state was used.
    pub view_id: String,
    /// Client interaction generation.
    pub revision: u32,
    /// Actual supplied local parameters.
    pub values: BTreeMap<String, f64>,
    /// Actual mathematical output, including real kernel geometry if requested.
    pub item: Box<crate::views::OutputItem>,
    /// Actual readonly evaluator messages.
    pub messages: Vec<crate::wire::Message>,
    /// Actual evaluator/sampling elapsed time when a host clock exists; zero otherwise.
    pub timing_ms: f64,
    /// Immutable raw expression graph for explicit pure data export; omitted for other results.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub value_token: Option<String>,
}
