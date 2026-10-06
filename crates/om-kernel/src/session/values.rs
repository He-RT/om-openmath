//! Retained-output paging checks source ownership, freshness and snapshot identity before projecting data.
use super::Session;
use crate::protocol::*;
use om_core::Interrupt;
impl Session {
    pub(super) fn inspect_value(&self, query: ValueQuery) -> Response {
        let invalid = || Response::Error {
            message: "结果已更新、已过期或分页路径无效".into(),
        };
        let Some(cell) = self
            .notebook
            .cells
            .iter()
            .find(|c| c.id == query.cell_id && c.status == CellStatus::Done)
        else {
            return invalid();
        };
        let Some(record) = cell
            .records
            .iter()
            .find(|r| r.out_index == query.out_index && r.view_id == query.view_id)
        else {
            return invalid();
        };
        let start = self.clock.as_ref().map(|c| c.now_ms());
        let ctx = Interrupt {
            clock: self.clock.clone(),
            deadline_ms: start
                .map(|ms| ms + self.config.general.eval_timeout_ms.min(5000) as f64)
                .filter(|ms| ms.is_finite()),
            ..Interrupt::default()
        };
        match crate::output::values::page(record, &query, &ctx) {
            Ok(page) => Response::ValuePage { page },
            Err(e) => Response::Error {
                message: e.to_string(),
            },
        }
    }
}
