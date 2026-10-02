//! Restart definition execution uses the existing actual static dependency planner.
use super::Session;
use crate::protocol::*;
use std::collections::BTreeSet;
impl Session {
    fn definitions(&self) -> BTreeSet<usize> {
        self.notebook
            .cells
            .iter()
            .enumerate()
            .filter(|(_, c)| c.kind == CellKind::Math && !c.defines.is_empty())
            .map(|(i, _)| i)
            .collect()
    }
    pub(super) fn notebook_state(&self) -> Response {
        let plan = self.plan(&self.definitions());
        let names = |symbols: &BTreeSet<om_core::Symbol>| {
            let mut names = symbols
                .iter()
                .map(|s| s.name().into())
                .collect::<Vec<String>>();
            names.sort();
            names
        };
        Response::NotebookState {
            state: NotebookState {
                file: self.notebook.to_file(),
                cells: self
                    .notebook
                    .cells
                    .iter()
                    .map(|c| CellState {
                        id: c.id.clone(),
                        status: c.status,
                        defines: names(&c.defines),
                        uses: names(&c.uses),
                    })
                    .collect(),
                definition_order: plan
                    .order
                    .iter()
                    .map(|&i| self.notebook.cells[i].id.clone())
                    .collect(),
                cycles: plan
                    .cycles
                    .union(&plan.blocked)
                    .map(|&i| self.notebook.cells[i].id.clone())
                    .collect(),
            },
        }
    }
    pub(super) fn restore_definitions(&mut self) -> (Response, Vec<Event>) {
        let mut events = self.cancel_all_llm();
        self.eval = om_eval::Evaluator::new();
        self.owners.clear();
        self.apply_settings();
        self.interrupt
            .store(false, std::sync::atomic::Ordering::Relaxed);
        for c in &mut self.notebook.cells {
            c.records.clear();
            c.output = None;
            c.exec_count = None;
            c.status = if c.kind == CellKind::Text {
                CellStatus::Done
            } else {
                CellStatus::Stale
            };
        }
        for i in 0..self.notebook.cells.len() {
            self.analyze_cell(i);
        }
        let selected = self.definitions();
        let plan = self.plan(&selected);
        let mut failed = BTreeSet::new();
        for &i in plan.cycles.union(&plan.blocked) {
            self.cell_error(
                i,
                "err.cycle",
                self.localized(
                    "定义恢复存在循环依赖",
                    "Definition recovery has cyclic dependencies",
                ),
            );
            self.notify_status(i, CellStatus::Error, &mut events);
            self.notify_output(i, &mut events);
            failed.insert(i);
        }
        for i in plan.order {
            if failed.iter().any(|&from| plan.edges[from].contains(&i)) {
                failed.insert(i);
                continue;
            }
            self.notify_status(i, CellStatus::Running, &mut events);
            self.run_cell(i);
            let status = self.notebook.cells[i].status;
            if status != CellStatus::Done {
                failed.insert(i);
            }
            self.notify_status(i, status, &mut events);
            self.notify_output(i, &mut events);
        }
        for c in &self.notebook.cells {
            if c.status == CellStatus::Stale {
                events.push(Event::CellStatus {
                    cell_id: c.id.clone(),
                    status: c.status,
                });
            }
        }
        events.push(Event::KernelRestarted {
            message: self.localized(
                "计算已中断，内核已重启",
                "Computation interrupted; kernel restarted",
            ),
        });
        (self.notebook_state(), events)
    }
}
