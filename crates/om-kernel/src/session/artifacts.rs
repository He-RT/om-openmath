//! Artifact production checks retained value identities; geometry exports never execute a mathematical source.
use super::Session;
use crate::protocol::*;
use om_core::Interrupt;
use std::cell::Cell;
impl Session {
    fn artifact_budget(&self) -> Interrupt {
        self.reset_interrupt();
        let now = self.clock.as_ref().map(|clock| clock.now_ms());
        Interrupt {
            flag: self.interrupt.clone(),
            clock: self.clock.clone(),
            deadline_ms: now
                .map(|t| t + self.config.general.eval_timeout_ms.min(10000) as f64)
                .filter(|t| t.is_finite()),
            steps_left: Cell::new(16 * 1024 * 1024),
        }
    }
    pub(super) fn export_scene3d(&self, data: Scene3DData, title: String) -> Response {
        match crate::artifact::export_scene(&data, &title, &self.artifact_budget()) {
            Ok(artifact) => Response::Artifact { artifact },
            Err(e) => Response::Error {
                message: e.to_string(),
            },
        }
    }
    pub(super) fn export_plot(&self, figure: PlotFigure, format: PlotExportFormat) -> Response {
        match crate::artifact::export_plot(&figure, format, &self.artifact_budget()) {
            Ok(artifact) => Response::Artifact { artifact },
            Err(e) => Response::Error {
                message: e.to_string(),
            },
        }
    }
    pub(super) fn export_value(&self, q: ValueQuery, format: DataExportFormat) -> Response {
        let invalid = || Response::Error {
            message: "数据导出快照已过期、路径或范围无效".into(),
        };
        if q.path.len() > 32 {
            return invalid();
        }
        let Some(cell) = self
            .notebook
            .cells
            .iter()
            .find(|c| c.id == q.cell_id && c.status == CellStatus::Done)
        else {
            return invalid();
        };
        let Some(record) = cell
            .records
            .iter()
            .find(|r| r.out_index == q.out_index && r.view_id == q.view_id)
        else {
            return invalid();
        };
        let ctx = self.artifact_budget();
        let mut value = &record.value;
        for i in &q.path {
            if let Err(e) = ctx.tick() {
                return Response::Error {
                    message: e.to_string(),
                };
            }
            let Some(next) = value.args().get(*i as usize) else {
                return invalid();
            };
            value = next;
        }
        match crate::artifact::export_data(value, format, &ctx) {
            Ok(artifact) => Response::Artifact { artifact },
            Err(e) => Response::Error {
                message: e.to_string(),
            },
        }
    }
    pub(super) fn export_value_token(&self, token: String, format: DataExportFormat) -> Response {
        let ctx = self.artifact_budget();
        let value = match crate::explore::token_value(&token, &ctx) {
            Ok(value) => value,
            Err(e) => {
                return Response::Error {
                    message: e.to_string(),
                };
            }
        };
        match crate::artifact::export_data(&value, format, &ctx) {
            Ok(artifact) => Response::Artifact { artifact },
            Err(e) => Response::Error {
                message: e.to_string(),
            },
        }
    }
}
