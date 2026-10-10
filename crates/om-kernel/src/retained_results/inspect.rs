use super::*;
use om_core::{BUILTIN as B, Expr, ExprKind, Interrupt};
fn representation_budget(expr: &Expr, ctx: &Interrupt) -> Result<(), String> {
    let mut pending = vec![expr];
    let mut nodes = 0usize;
    // Formatting expands shared occurrences. Charge the expansion before formatting a DAG.
    while let Some(value) = pending.pop() {
        ctx.tick().map_err(|e| e.to_string())?;
        nodes += 1;
        if nodes > 200000 {
            return Err("RESULT_REPRESENTATION_BUDGET_EXCEEDED".into());
        }
        if let ExprKind::Normal(normal) = value.kind() {
            pending.push(&normal.head);
            pending.extend(normal.args.iter());
        }
    }
    Ok(())
}
/// One actual step with a bounded child route. Children are paged separately, never invented.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RetainedStepEntry {
    /// Actual root/child coordinate within the immutable original record.
    pub path: Vec<u32>,
    /// Actual recorded node, with child list omitted from this page.
    pub step: StepView,
    /// Original number of direct computational children.
    pub child_count: u32,
}
/// One page of real recorded steps.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RetainedStepPage {
    /// Original view occurrence.
    pub view_id: String,
    /// Requested parent path, empty means computational roots.
    pub parent_path: Vec<u32>,
    /// True if steps were recorded, including a genuinely empty record.
    pub recorded: bool,
    /// Actual sibling count at this route.
    pub total: u32,
    /// Actual first sibling offset.
    pub offset: u32,
    /// Actual selected siblings.
    pub entries: Vec<RetainedStepEntry>,
}
impl RetainedResult {
    /// Project exact structural data without evaluating any source or changing original history.
    pub fn value_page(&self, query: &ValueQuery, ctx: &Interrupt) -> Result<ValuePage, String> {
        if query.cell_id != self.cell.id {
            return Err("INVALID_RESULT_OWNER".into());
        }
        let record = self.record(query.out_index, &query.view_id)?;
        crate::output::values::page(record, query, ctx).map_err(|e| e.to_string())
    }
    /// Complete retained value source at a data route, never reevaluated from formatted text.
    pub fn value_source(
        &self,
        out_index: u32,
        view_id: &str,
        path: &[u32],
        format: ResultSourceFormat,
        ctx: &Interrupt,
    ) -> Result<String, String> {
        if path.len() > 32 {
            return Err("INVALID_RESULT_PATH".into());
        }
        let mut value = &self.record(out_index, view_id)?.value;
        for &part in path {
            ctx.tick().map_err(|e| e.to_string())?;
            value = value
                .args()
                .get(part as usize)
                .ok_or("INVALID_RESULT_PATH")?;
        }
        ctx.tick().map_err(|e| e.to_string())?;
        representation_budget(value, ctx)?;
        Ok(match format {
            ResultSourceFormat::InputForm => om_format::input_form(value),
            ResultSourceFormat::Modern => om_format::modern_form(value),
            ResultSourceFormat::Latex => om_format::latex(value),
        })
    }
    /// Original held successful input for this occurrence, not the later contents of its cell.
    pub fn statement_input(
        &self,
        out_index: u32,
        view_id: &str,
        format: ResultSourceFormat,
        ctx: &Interrupt,
    ) -> Result<String, String> {
        ctx.tick().map_err(|e| e.to_string())?;
        let value = &self.record(out_index, view_id)?.input;
        representation_budget(value, ctx)?;
        Ok(match format {
            ResultSourceFormat::InputForm => om_format::input_form(value),
            ResultSourceFormat::Modern => om_format::modern_form(value),
            ResultSourceFormat::Latex => om_format::latex(value),
        })
    }
    /// Numerical view evaluates the original value in a fresh readonly fork; no Out/random mutation.
    pub fn numeric(
        &self,
        out_index: u32,
        view_id: &str,
        path: &[u32],
        digits: u32,
        ctx: &Interrupt,
    ) -> Result<ExpressionView, String> {
        if digits == 0 || digits > 1000 || path.len() > 32 {
            return Err("INVALID_NUMERIC_PROJECTION".into());
        }
        let mut value = &self.record(out_index, view_id)?.value;
        for &part in path {
            ctx.tick().map_err(|e| e.to_string())?;
            value = value
                .args()
                .get(part as usize)
                .ok_or("INVALID_RESULT_PATH")?;
        }
        let mut readonly = self.readonly.fork_readonly();
        readonly.settings.record_steps = false;
        let value = readonly
            .evaluate(
                &Expr::call(B::N, [value.clone(), Expr::int(digits.into())]),
                ctx,
            )
            .map_err(|e| e.to_string())?;
        if let Some(message) = readonly.messages.take().into_iter().find(|m| {
            matches!(
                m.level,
                om_core::MsgLevel::Warning | om_core::MsgLevel::Error
            )
        }) {
            return Err(format!(
                "{}::{}: {}",
                message.symbol, message.tag, message.text
            ));
        }
        representation_budget(&value, ctx)?;
        Ok(crate::output::expression_view(&value))
    }
    /// Explicit readonly expression inspection; assignments, random functions and history writers
    /// remain forbidden by the actual evaluator even when hidden in a user function's delayed RHS.
    pub fn readonly_expression(
        &self,
        source: &str,
        numeric: bool,
        ctx: &Interrupt,
    ) -> Result<ExpressionView, String> {
        if source.len() > 65536 {
            return Err("INSPECTION_BUDGET_EXCEEDED".into());
        }
        let expr = om_parse::parse_input_form(source).map_err(|_| "INVALID_INSPECTION_SOURCE")?;
        let mut work = vec![&expr];
        while let Some(value) = work.pop() {
            ctx.tick().map_err(|e| e.to_string())?;
            if let ExprKind::Normal(n) = value.kind() {
                if n.head.as_symbol().is_some_and(|symbol| {
                    matches!(
                        symbol.name(),
                        "Set"
                            | "SetDelayed"
                            | "Unset"
                            | "Clear"
                            | "ClearAll"
                            | "CompoundExpression"
                            | "SeedRandom"
                    )
                }) {
                    return Err("READONLY_VIOLATION".into());
                }
                work.push(&n.head);
                work.extend(n.args.iter());
            }
        }
        let expr = if numeric {
            Expr::call(B::N, [expr, Expr::int(20)])
        } else {
            expr
        };
        let mut readonly = self.readonly.fork_readonly();
        readonly.settings.record_steps = false;
        let value = readonly.evaluate(&expr, ctx).map_err(|e| e.to_string())?;
        if let Some(message) = readonly.messages.take().into_iter().find(|m| {
            matches!(
                m.level,
                om_core::MsgLevel::Warning | om_core::MsgLevel::Error
            )
        }) {
            return Err(format!(
                "{}::{}: {}",
                message.symbol, message.tag, message.text
            ));
        }
        representation_budget(&value, ctx)?;
        Ok(crate::output::expression_view(&value))
    }
    /// Actual step siblings from retained records; rendering is pure and does not run a solver.
    pub fn steps_page(
        &self,
        out_index: u32,
        view_id: &str,
        parent_path: &[u32],
        offset: u32,
        limit: u32,
        ctx: &Interrupt,
    ) -> Result<RetainedStepPage, String> {
        if parent_path.len() > 128 || limit == 0 || limit > 100 {
            return Err("INVALID_STEPS_PAGE".into());
        }
        let record = self.record(out_index, view_id)?;
        let Some(steps) = record.steps.as_ref() else {
            if !parent_path.is_empty() || offset != 0 {
                return Err("STEPS_NOT_RECORDED".into());
            }
            return Ok(RetainedStepPage {
                view_id: view_id.into(),
                parent_path: Vec::new(),
                recorded: false,
                total: 0,
                offset: 0,
                entries: Vec::new(),
            });
        };
        let mut siblings = steps.root.as_slice();
        for &part in parent_path {
            ctx.tick().map_err(|e| e.to_string())?;
            siblings = siblings
                .get(part as usize)
                .ok_or("INVALID_STEPS_PATH")?
                .children
                .as_slice();
        }
        if offset as usize > siblings.len() {
            return Err("INVALID_STEPS_PAGE".into());
        }
        let mut entries = Vec::new();
        for (index, node) in siblings
            .iter()
            .enumerate()
            .skip(offset as usize)
            .take(limit as usize)
        {
            ctx.tick().map_err(|e| e.to_string())?;
            let selected = om_solve::Step {
                id: node.id.clone(),
                rule_id: node.rule_id,
                kind: node.kind.clone(),
                before: node.before.clone(),
                after: node.after.clone(),
                level: node.level,
                children: Vec::new(),
            };
            let step = crate::output::render_steps(&om_solve::Steps {
                root: vec![selected],
            })
            .root
            .pop()
            .ok_or("INVALID_RECORDED_STEP")?;
            let mut path = parent_path.to_vec();
            path.push(index as u32);
            entries.push(RetainedStepEntry {
                path,
                step,
                child_count: node.children.len() as u32,
            });
        }
        Ok(RetainedStepPage {
            view_id: view_id.into(),
            parent_path: parent_path.to_vec(),
            recorded: true,
            total: siblings.len() as u32,
            offset,
            entries,
        })
    }
}
