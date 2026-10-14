use super::*;
use om_kernel::{
    protocol::{Message, ValuePage},
    retained_results::{
        RetainedGeometryPage, RetainedGeometryQuery, RetainedGeometrySummary, RetainedResult,
        RetainedResultSummary, RetainedStepPage,
    },
};
/// Shared projection only; origin-specific provenance never becomes a main receipt.
pub(super) struct InspectionView<'a> {
    value: &'a RetainedResult,
    binding: Value,
    pub origin: &'static str,
    pub current: bool,
    pub cell_id: String,
    pub out_index: u32,
    pub view_id: Option<String>,
}
impl<'a> InspectionView<'a> {
    pub fn notebook(result: &'a ResolvedResult) -> Result<Self, &'static str> {
        let b = result.binding();
        Ok(Self {
            value: result.retained(),
            binding: serde_json::to_value(b).map_err(|_| "NOT_AVAILABLE")?,
            origin: "notebook",
            current: b.acceptance == ResultBindingAcceptance::Accepted
                && b.freshness != ResultBindingFreshness::Stale,
            cell_id: b.cell_id.clone(),
            out_index: u32::try_from(b.out_index.get()).map_err(|_| "NOT_AVAILABLE")?,
            view_id: b.view_id.0.clone(),
        })
    }
    pub fn scratch(result: &'a super::super::scratch::ResolvedScratch) -> Self {
        let summary = result.summary();
        let last = summary.statements.last();
        Self {
            value: result.retained(),
            binding: result.binding(),
            origin: "scratch",
            current: result.is_current(),
            cell_id: summary.cell_id,
            out_index: last.map_or(0, |s| s.out_index),
            view_id: last.map(|s| s.view_id.clone()),
        }
    }
    pub fn binding(&self) -> &Value {
        &self.binding
    }
    pub fn summary(&self) -> RetainedResultSummary {
        self.value.summary()
    }
    pub fn messages(&self) -> Vec<Message> {
        self.value.messages()
    }
    fn occurrence(&self) -> Result<(u32, &str), ResultError> {
        Ok((
            self.out_index,
            self.view_id.as_deref().ok_or(ResultError::Invalid)?,
        ))
    }
    pub fn presentation(
        &self,
        offset: u32,
        limit: u32,
        ctx: &Interrupt,
    ) -> Result<Value, ResultError> {
        if self.out_index == 0 && self.view_id.is_none() {
            return if offset == 0 && (1..=32).contains(&limit) {
                Ok(self.value.failure_presentation())
            } else {
                Err(ResultError::Invalid)
            };
        }
        let (o, v) = self.occurrence()?;
        self.value
            .presentation_page(o, v, offset, limit, ctx)
            .map_err(ResultError::Projection)
    }
    pub fn page(&self, query: &ValueQuery, ctx: &Interrupt) -> Result<ValuePage, ResultError> {
        let (o, v) = self.occurrence()?;
        if query.cell_id != self.cell_id || query.out_index != o || query.view_id != v {
            return Err(ResultError::Invalid);
        }
        self.value
            .value_page(query, ctx)
            .map_err(ResultError::Projection)
    }
    pub fn steps(
        &self,
        parent: &[u32],
        offset: u32,
        limit: u32,
        ctx: &Interrupt,
    ) -> Result<RetainedStepPage, ResultError> {
        let (o, v) = self.occurrence()?;
        self.value
            .steps_page(o, v, parent, offset, limit, ctx)
            .map_err(ResultError::Projection)
    }
    pub fn geometry(&self, ctx: &Interrupt) -> Result<RetainedGeometrySummary, ResultError> {
        let (o, v) = self.occurrence()?;
        self.value
            .geometry_summary(o, v, ctx)
            .map_err(ResultError::Projection)
    }
    pub fn geometry_page(
        &self,
        q: &RetainedGeometryQuery,
        ctx: &Interrupt,
    ) -> Result<RetainedGeometryPage, ResultError> {
        let (o, v) = self.occurrence()?;
        self.value
            .geometry_page(o, v, q, ctx)
            .map_err(ResultError::Projection)
    }
}
