//! Immutable real cell/statement snapshots; projections never run the producer notebook source.
mod geometry;
mod inspect;
use crate::{
    Cell,
    protocol::*,
    value_views::{ScientificOrigin, ValueKind, ValueNature},
};
pub use geometry::{
    GeometryChannel, RetainedGeometryPage, RetainedGeometryQuery, RetainedGeometrySummary,
};
pub use inspect::{RetainedStepEntry, RetainedStepPage};
use om_eval::Evaluator;
use serde::{Deserialize, Serialize};

/// One real result source format; complete source is never an ellipsized display preview.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResultSourceFormat {
    /// Exact Wolfram InputForm.
    InputForm,
    /// Compatible modern source.
    Modern,
    /// Formula source for native typesetting.
    Latex,
}
/// Actual retained output kind, independent of a record's textual status keys.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RetainedOutputKind {
    /// Output suppressed, so it is not visible notebook content.
    Suppressed,
    /// Actual expression/data.
    Expression,
    /// Actual solver presentation.
    Solutions,
    /// Actual already sampled 2D geometry.
    Plot,
    /// Actual scene with an explicit sampled/data-only/unavailable state.
    Scene3d,
    /// Actual local exploration output/context.
    Explore,
    /// Packing failed after actual evaluation.
    Error,
}
/// Identity/provenance of an actual successful statement, including suppressed statements.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RetainedStatementInfo {
    /// Actual successful history ordinal, not a stable global result ID.
    pub out_index: u32,
    /// Producer occurrence identity; always paired with its actual owner cell.
    pub view_id: String,
    /// True original output suppression.
    pub suppressed: bool,
    /// Actual structural value category.
    pub value_kind: ValueKind,
    /// Actual scalar/data representation, not inferred from a field named "verified".
    pub nature: ValueNature,
    /// Actual output production kind.
    pub output_kind: RetainedOutputKind,
    /// True if recorded computational steps exist.
    pub steps_recorded: bool,
    /// Fresh actual callback provenance only.
    pub scientific: Option<ScientificOrigin>,
}
/// Small retained metadata. It carries no complete values, step trees or geometry arrays.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RetainedResultSummary {
    /// Actual owner cell.
    pub cell_id: String,
    /// Original source kind.
    pub cell_kind: CellKind,
    /// Actual terminal status, including errors after partial effects.
    pub status: CellStatus,
    /// Raw original source byte length; complete text is requested explicitly.
    pub source_byte_length: usize,
    /// All original successful records.
    pub statements: Vec<RetainedStatementInfo>,
    /// Actual message count, not a reconstructed diagnostic guarantee.
    pub message_count: usize,
}
/// Actual current occurrence index for freshness checks; complete values are never duplicated.
#[derive(Clone, Debug)]
pub struct CurrentResultCell {
    /// Real owner cell.
    pub cell_id: String,
    /// Actual current cell state, including dependency invalidation.
    pub status: CellStatus,
    /// Actual retained successful output occurrences.
    pub occurrences: Vec<(u32, String)>,
}
/// Actual frozen cell records and readonly definition context. No writable/session/file owner.
pub struct RetainedResult {
    cell: Cell,
    readonly: Evaluator,
}
impl RetainedResult {
    pub(crate) fn capture(cell: &Cell, eval: &Evaluator) -> Result<Self, String> {
        if matches!(cell.status, CellStatus::Running | CellStatus::Queued) {
            return Err("RESULT_NOT_READY".into());
        }
        Ok(Self {
            cell: cell.clone(),
            readonly: eval.fork_readonly(),
        })
    }
    /// Actual raw producer input, including Unicode and incomplete source before an error.
    pub fn input(&self) -> CellInput {
        CellInput {
            id: self.cell.id.clone(),
            kind: self.cell.kind,
            source: self.cell.source.clone(),
            dialect: self.cell.dialect,
        }
    }
    /// Original cell status and record/type facts, without serialization of the mathematical data.
    pub fn summary(&self) -> RetainedResultSummary {
        let mut visible = 0;
        let statements = self
            .cell
            .records
            .iter()
            .map(|record| {
                let item = if record.suppress_output {
                    None
                } else {
                    let item = self.cell.output.as_ref().and_then(|o| o.items.get(visible));
                    visible += 1;
                    item
                };
                let output_kind = if record.suppress_output {
                    RetainedOutputKind::Suppressed
                } else {
                    match item {
                        Some(OutputItem::Expr { .. }) => RetainedOutputKind::Expression,
                        Some(OutputItem::Solutions { .. }) => RetainedOutputKind::Solutions,
                        Some(OutputItem::Plot { .. }) => RetainedOutputKind::Plot,
                        Some(OutputItem::Scene3D { .. }) => RetainedOutputKind::Scene3d,
                        Some(OutputItem::Explore { .. }) => RetainedOutputKind::Explore,
                        _ => RetainedOutputKind::Error,
                    }
                };
                RetainedStatementInfo {
                    out_index: record.out_index,
                    view_id: record.view_id.clone(),
                    suppressed: record.suppress_output,
                    value_kind: crate::output::values::kind(&record.value),
                    nature: crate::output::values::nature(&record.value),
                    output_kind,
                    steps_recorded: record.steps.is_some(),
                    scientific: record.scientific.as_ref().and_then(|o| {
                        om_core::catalog::by_runtime(o.name).map(|d| ScientificOrigin {
                            function_id: d.id.clone(),
                            name: d.modern_name.clone(),
                        })
                    }),
                }
            })
            .collect();
        RetainedResultSummary {
            cell_id: self.cell.id.clone(),
            cell_kind: self.cell.kind,
            status: self.cell.status,
            source_byte_length: self.cell.source.len(),
            statements,
            message_count: self.cell.output.as_ref().map_or(0, |o| o.messages.len()),
        }
    }
    /// Exact actual terminal messages, never fabricated from a generic successful request.
    pub fn messages(&self) -> Vec<Message> {
        self.cell
            .output
            .as_ref()
            .map_or_else(Vec::new, |o| o.messages.clone())
    }
    /// Actual zero-success failure output, including original parser/evaluator byte spans.
    pub fn failure_presentation(&self) -> serde_json::Value {
        serde_json::json!({"kind":"error","status":self.cell.status,"items":self.cell.output.as_ref().map(|o|&o.items),"messages":self.messages()})
    }
    fn record(
        &self,
        out_index: u32,
        view_id: &str,
    ) -> Result<&crate::notebook::StatementRecord, String> {
        self.cell
            .records
            .iter()
            .find(|r| r.out_index == out_index && r.view_id == view_id)
            .ok_or_else(|| "INVALID_RESULT_OCCURRENCE".into())
    }
    fn output(&self, out_index: u32, view_id: &str) -> Result<Option<&OutputItem>, String> {
        let record = self.record(out_index, view_id)?;
        if record.suppress_output {
            return Ok(None);
        }
        let index = self
            .cell
            .records
            .iter()
            .take_while(|r| r.view_id != view_id)
            .filter(|r| !r.suppress_output)
            .count();
        Ok(self.cell.output.as_ref().and_then(|o| o.items.get(index)))
    }
    fn geometry_output(
        &self,
        out_index: u32,
        view_id: &str,
    ) -> Result<Option<&OutputItem>, String> {
        let mut output = self.output(out_index, view_id)?;
        for _ in 0..16 {
            if let Some(OutputItem::Explore { result, .. }) = output {
                output = Some(result.item.as_ref());
            } else {
                return Ok(output);
            }
        }
        Err("INVALID_RETAINED_EXPLORATION_DEPTH".into())
    }
}
