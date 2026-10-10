use super::*;
use om_kernel::{
    protocol::{Dialect, ExpressionView, Request, Response, ValuePage, ValueQuery},
    retained_results::{
        ResultSourceFormat, RetainedGeometryPage, RetainedGeometryQuery, RetainedGeometrySummary,
        RetainedStepPage,
    },
};
use om_num::ctx::Interrupt;
impl ResolvedResult {
    fn occurrence(&self) -> Result<(u32, &str), ResultError> {
        Ok((
            self.occurrence.out_index,
            self.occurrence
                .view_id
                .as_deref()
                .ok_or(ResultError::Invalid)?,
        ))
    }
    /// Original producer source text, including incomplete Unicode input, not the current cell text.
    pub fn cell_source(&self) -> String {
        self.record.value.input().source
    }
    /// Page the original value with the exact retained cell/out/view identities.
    pub fn page(&self, query: &ValueQuery, ctx: &Interrupt) -> Result<ValuePage, ResultError> {
        let (out, view) = self.occurrence()?;
        if query.cell_id != self.occurrence.cell_id
            || query.out_index != out
            || query.view_id != view
        {
            return Err(ResultError::Invalid);
        }
        self.record
            .value
            .value_page(query, ctx)
            .map_err(ResultError::Projection)
    }
    /// Explicit full value source; host transport may send it as bounded byte fragments.
    pub fn source(
        &self,
        path: &[u32],
        format: ResultSourceFormat,
        ctx: &Interrupt,
    ) -> Result<String, ResultError> {
        let (out, view) = self.occurrence()?;
        self.record
            .value
            .value_source(out, view, path, format, ctx)
            .map_err(ResultError::Projection)
    }
    /// Numeric projection against the original readonly context/value, never formatted-source replay.
    pub fn numeric(
        &self,
        path: &[u32],
        digits: u32,
        ctx: &Interrupt,
    ) -> Result<ExpressionView, ResultError> {
        let (out, view) = self.occurrence()?;
        self.record
            .value
            .numeric(out, view, path, digits, ctx)
            .map_err(ResultError::Projection)
    }
    /// Explicit readonly expression inspection. Actual diagnostics/seed/write rejection are retained.
    pub fn readonly_expression(
        &self,
        source: &str,
        numeric: bool,
        ctx: &Interrupt,
    ) -> Result<ExpressionView, ResultError> {
        self.record
            .value
            .readonly_expression(source, numeric, ctx)
            .map_err(ResultError::Projection)
    }
    /// Real recorded step siblings; absent steps remain explicit rather than synthesized.
    pub fn steps(
        &self,
        parent: &[u32],
        offset: u32,
        limit: u32,
        ctx: &Interrupt,
    ) -> Result<RetainedStepPage, ResultError> {
        let (out, view) = self.occurrence()?;
        self.record
            .value
            .steps_page(out, view, parent, offset, limit, ctx)
            .map_err(ResultError::Projection)
    }
    /// Original immutable geometry availability/counts, with no resampling or renderer claim.
    pub fn geometry(&self, ctx: &Interrupt) -> Result<RetainedGeometrySummary, ResultError> {
        let (out, view) = self.occurrence()?;
        self.record
            .value
            .geometry_summary(out, view, ctx)
            .map_err(ResultError::Projection)
    }
    /// Actual original coordinates/indices/colors at a bounded data route.
    pub fn geometry_page(
        &self,
        query: &RetainedGeometryQuery,
        ctx: &Interrupt,
    ) -> Result<RetainedGeometryPage, ResultError> {
        let (out, view) = self.occurrence()?;
        self.record
            .value
            .geometry_page(out, view, query, ctx)
            .map_err(ResultError::Projection)
    }
    /// Isolated scratch with original accepted definitions. Restore remains an unaccepted writable
    /// work object, not a promotion of the readonly projection, and it never reaches a main owner.
    pub fn scratch(
        &self,
        source: String,
        dialect: Dialect,
        ctx: &Interrupt,
    ) -> Result<Response, ResultError> {
        if source.len() > 65536 {
            return Err(ResultError::Budget);
        }
        let state = &self.record.kernel;
        let mut work = state
            .restore(
                CheckpointRestore {
                    binding: state.binding(),
                    source: state.source(),
                    general: state.general(),
                    clock: ctx.clock.clone(),
                    cancel: ctx.flag.clone(),
                },
                CheckpointLimits::default(),
                ctx,
            )
            .map_err(|e| ResultError::Projection(e.to_string()))?;
        // A scratch expression is outside the notebook dependency graph, so it never cascades
        // into old producer cells and never inherits their execution policy.
        work.session_mut().config.general.reactive = false;
        work.session_mut().config.general.eval_timeout_ms =
            state.general().eval_timeout_ms.min(5000);
        Ok(work
            .session_mut()
            .handle(Request::Evaluate {
                cell_id: "isolated-result-scratch".into(),
                source,
                dialect,
            })
            .0)
    }
}
