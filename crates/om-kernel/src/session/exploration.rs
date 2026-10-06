//! Snapshot identity checks precede all readonly exploration. Detached workers have no notebook credentials.
use super::Session;
use crate::{notebook::StatementRecord, protocol::*};
use om_core::Interrupt;
use std::{collections::BTreeMap, sync::atomic::Ordering};
impl Session {
    fn explore_record(&self, q: &ExploreQuery) -> Option<&StatementRecord> {
        self.notebook
            .cells
            .iter()
            .find(|c| c.id == q.cell_id && c.status == CellStatus::Done)?
            .records
            .iter()
            .find(|r| {
                r.out_index == q.out_index && r.view_id == q.view_id && r.exploration.is_some()
            })
    }
    fn explore_budget(&self) -> Interrupt {
        self.interrupt.store(false, Ordering::Relaxed);
        let now = self.clock.as_ref().map(|c| c.now_ms());
        Interrupt {
            flag: self.interrupt.clone(),
            clock: self.clock.clone(),
            deadline_ms: now
                .map(|t| t + self.config.general.eval_timeout_ms as f64)
                .filter(|t| t.is_finite()),
            ..Default::default()
        }
    }
    pub(super) fn explore_context(&self, q: ExploreQuery) -> Response {
        let Some(record) = self.explore_record(&q) else {
            return Response::Error {
                message: "参数探索输出已过期或已替换".into(),
            };
        };
        match crate::explore::context(record, &self.explore_budget()) {
            Ok(context) => Response::ExploreContext {
                view_id: q.view_id,
                context,
            },
            Err(e) => Response::Error {
                message: self.plot_error(&e).1,
            },
        }
    }
    pub(super) fn sample_explore(
        &self,
        q: ExploreQuery,
        values: BTreeMap<String, f64>,
        revision: u32,
    ) -> Response {
        let Some(record) = self.explore_record(&q) else {
            return Response::Error {
                message: "参数探索输出已过期或已替换".into(),
            };
        };
        match crate::explore::compute(
            record.exploration.as_ref().unwrap(),
            &q.view_id,
            q.out_index,
            values,
            revision,
            &self.explore_budget(),
        ) {
            Ok(result) => Response::Explored { result },
            Err(e) => Response::Error {
                message: self.plot_error(&e).1,
            },
        }
    }
    pub(super) fn inspect_explore_expression(
        &self,
        context: String,
        values: BTreeMap<String, f64>,
        source: String,
        numeric: bool,
    ) -> Response {
        match crate::explore::detached_expression(
            &context,
            values,
            &source,
            numeric,
            &self.explore_budget(),
        ) {
            Ok(value) => Response::Expression { value },
            Err(e) => Response::Error {
                message: self.plot_error(&e).1,
            },
        }
    }
    pub(super) fn sample_explore_plot(
        &self,
        context: String,
        values: BTreeMap<String, f64>,
        request: PlotRequest,
    ) -> Response {
        match crate::explore::detached_plot(&context, values, request, &self.explore_budget()) {
            Ok(data) => Response::Plot { data },
            Err(e) => Response::Error {
                message: self.plot_error(&e).1,
            },
        }
    }
    pub(super) fn run_explore_context(
        &self,
        context: String,
        values: BTreeMap<String, f64>,
        revision: u32,
    ) -> Response {
        match crate::explore::detached(&context, values, revision, &self.explore_budget()) {
            Ok(result) => Response::Explored { result },
            Err(e) => Response::Error {
                message: self.plot_error(&e).1,
            },
        }
    }
}
