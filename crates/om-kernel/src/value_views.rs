//! Readonly bounded structured pages, with immutable output occurrence identities and full source routes.
use crate::views::ExpressionView;
/// Readonly retained-output projection request, checked against the current producer snapshot.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export_to=concat!(env!("CARGO_MANIFEST_DIR"),"/../../app/src/kernel/generated/"))]
pub struct ValueQuery {
    /// Owner cell.
    pub cell_id: String,
    /// Actual output history index.
    pub out_index: u32,
    /// Opaque producer serial.
    pub view_id: String,
    /// Zero-based data path.
    pub path: Vec<u32>,
    /// First row.
    pub offset: u32,
    /// 1..100 rows.
    pub limit: u32,
    /// First value column.
    pub column_offset: u32,
    /// 1..32 columns.
    pub column_limit: u32,
    /// Explicit complete-source request.
    pub include_source: bool,
}
use serde::{Deserialize, Serialize};
use ts_rs::TS;
/// Data shape; a record never establishes mathematical verification by itself.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export_to=concat!(env!("CARGO_MANIFEST_DIR"),"/../../app/src/kernel/generated/"))]
#[serde(rename_all = "snake_case")]
pub enum ValueKind {
    /// Scalar expression.
    Scalar,
    /// Ordered list.
    List,
    /// Rectangular two-dimensional list.
    Matrix,
    /// Literal keyed data.
    Record,
    /// Validated column/record table.
    Table,
    /// Held fitted numeric source.
    Model,
    /// Finite interpolation data.
    Interpolation,
    /// Finite Taylor data.
    Series,
    /// Quantity with dimensions.
    Quantity,
}
/// Representation of a leaf value, not a proof of a surrounding record's claims.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export_to=concat!(env!("CARGO_MANIFEST_DIR"),"/../../app/src/kernel/generated/"))]
#[serde(rename_all = "snake_case")]
pub enum ValueNature {
    /// Exact integer or rational atom.
    Exact,
    /// Machine atom.
    Machine,
    /// Existing arbitrary-precision atom.
    HighPrecision,
    /// Symbolic expression/data.
    Symbolic,
    /// Text literal.
    Text,
    /// Actual Boolean atom.
    Boolean,
    /// Null atom.
    Null,
}
/// One immutable value occurrence, without executing its source.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export_to=concat!(env!("CARGO_MANIFEST_DIR"),"/../../app/src/kernel/generated/"))]
pub struct ValueEntry {
    /// Producer-generated ID, stable across pagination/column pages of this output snapshot.
    pub id: String,
    /// Internal zero-based expression path, independent of language indexing.
    pub path: Vec<u32>,
    /// Structural shape.
    pub kind: ValueKind,
    /// Representation of this occurrence.
    pub nature: ValueNature,
    /// Actual logical size for containers, zero for atoms.
    pub count: u32,
    /// Complete scalar source; containers load full source only on an explicit request.
    pub source: Option<ExpressionView>,
}
/// A producer-owned immutable row occurrence.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export_to=concat!(env!("CARGO_MANIFEST_DIR"),"/../../app/src/kernel/generated/"))]
pub struct ValueRow {
    /// Stable occurrence identity within the snapshot.
    pub id: String,
    /// Record field name or displayed ordinal.
    pub label: String,
    /// Visible cells in the selected column page.
    pub cells: Vec<ValueEntry>,
}
/// Real fresh callback provenance, absent for arbitrary/cached record literals.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export_to=concat!(env!("CARGO_MANIFEST_DIR"),"/../../app/src/kernel/generated/"))]
pub struct ScientificOrigin {
    /// Stable callable identity.
    pub function_id: String,
    /// Preferred actual callback name.
    pub name: String,
}
/// A bounded view of an immutable actual output; no sampling/evaluation happens here.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export_to=concat!(env!("CARGO_MANIFEST_DIR"),"/../../app/src/kernel/generated/"))]
pub struct ValuePage {
    /// Opaque decimal serial; JavaScript must not round it to a number.
    pub view_id: String,
    /// Requested data subtree.
    pub path: Vec<u32>,
    /// Shape of this subtree.
    pub kind: ValueKind,
    /// Total logical rows/fields/items.
    pub row_count: u32,
    /// Total logical value columns.
    pub column_count: u32,
    /// First displayed row.
    pub offset: u32,
    /// First displayed column.
    pub column_offset: u32,
    /// Actual visible column labels.
    pub columns: Vec<String>,
    /// Actual bounded row page.
    pub rows: Vec<ValueRow>,
    /// Full source only for scalar/opaque details or explicitly requested container source.
    pub source: Option<ExpressionView>,
    /// Fresh producer provenance, not inferred from Boolean/status keys.
    pub origin: Option<ScientificOrigin>,
}
