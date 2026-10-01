//! One real execution path for direct requests, reactive cascades and RunAll.
use super::Session;
use crate::{Cell, protocol::*};
use om_core::Symbol;
use std::{collections::BTreeSet, sync::atomic::Ordering};

impl Session {
    pub(super) fn release_owned(&mut self, id: &str) -> BTreeSet<Symbol> {
        let owned: BTreeSet<_> = self
            .owners
            .iter()
            .filter(|(_, owner)| owner.as_str() == id)
            .map(|(&s, _)| s)
            .collect();
        for &symbol in &owned {
            self.eval.defs.clear(symbol);
            self.owners.remove(&symbol);
        }
        self.eval.defs.take_changed_symbols();
        owned
    }

    pub(super) fn cell_error(&mut self, index: usize, key: &str, text: String) {
        let cell = &mut self.notebook.cells[index];
        cell.status = CellStatus::Error;
        cell.output = Some(CellOutput {
            items: vec![OutputItem::Error {
                message: text.clone(),
                span: None,
            }],
            messages: vec![Message {
                symbol: "Kernel".into(),
                tag: key.into(),
                text,
                level: MsgLevel::Error,
            }],
            timing_ms: 0.0,
        });
        // A graph error does not undo a real prior evaluation or its solver provenance.
        if key != "err.cycle" {
            cell.exec_count = None;
            cell.records.clear();
        }
    }

    fn notify_status(&mut self, index: usize, status: CellStatus, events: &mut Vec<Event>) {
        let cell = &mut self.notebook.cells[index];
        cell.status = status;
        events.push(Event::CellStatus {
            cell_id: cell.id.clone(),
            status,
        });
    }

    fn notify_output(&self, index: usize, events: &mut Vec<Event>) {
        let cell = &self.notebook.cells[index];
        if let Some(output) = &cell.output {
            events.push(Event::CellOutput {
                cell_id: cell.id.clone(),
                output: output.clone(),
            });
        }
    }

    pub(super) fn stale(
        &mut self,
        selected: &BTreeSet<usize>,
        skip: Option<usize>,
        events: &mut Vec<Event>,
    ) {
        for &i in selected {
            if Some(i) != skip {
                self.notify_status(i, CellStatus::Stale, events);
            }
        }
    }

    pub(super) fn cascade(
        &mut self,
        changed: BTreeSet<Symbol>,
        origin: Option<usize>,
        allow_run: bool,
        events: &mut Vec<Event>,
    ) -> Vec<CellId> {
        if !self.config.general.reactive {
            return vec![];
        }
        let mut selected = self.affected(changed);
        if let Some(i) = origin {
            selected.insert(i);
        }
        let plan = self.plan(&selected);
        for &i in &plan.cycles {
            let text = self.localized(
                "err.cycle: 单元格之间存在循环依赖",
                "err.cycle: Cyclic cell dependencies",
            );
            self.cell_error(i, "err.cycle", text);
            self.notify_status(i, CellStatus::Error, events);
            self.notify_output(i, events);
        }
        self.stale(&plan.blocked, origin, events);
        let mut reran = vec![];
        let mut failed = plan.cycles.clone();
        failed.extend(&plan.blocked);
        for i in plan.order {
            if Some(i) == origin {
                if self.notebook.cells[i].status != CellStatus::Done {
                    failed.insert(i);
                }
                continue;
            }
            let blocked = failed.iter().any(|&from| plan.edges[from].contains(&i));
            if !allow_run
                || !self.config.general.auto_run_dependents
                || blocked
                || !self.prerequisites_ready(i)
            {
                self.notify_status(i, CellStatus::Stale, events);
                failed.insert(i);
                continue;
            }
            self.notify_status(i, CellStatus::Queued, events);
            self.notify_status(i, CellStatus::Running, events);
            self.run_cell(i);
            reran.push(self.notebook.cells[i].id.clone());
            let status = self.notebook.cells[i].status;
            if status != CellStatus::Done {
                failed.insert(i);
            }
            self.notify_status(i, status, events);
            self.notify_output(i, events);
        }
        reran
    }

    fn prerequisites_ready(&self, index: usize) -> bool {
        self.notebook.cells[index].uses.iter().all(|symbol| {
            if let Some(owner) = self.owners.get(symbol) {
                self.notebook
                    .cells
                    .iter()
                    .find(|c| &c.id == owner)
                    .is_none_or(|c| c.status == CellStatus::Done)
            } else {
                self.notebook.cells.iter().enumerate().all(|(i, c)| {
                    i == index
                        || c.kind != CellKind::Math
                        || !c.defines.contains(symbol)
                        || c.status == CellStatus::Done
                })
            }
        })
    }

    pub(super) fn evaluate_cell(
        &mut self,
        cell_id: CellId,
        source: String,
        dialect: Dialect,
    ) -> (Response, Vec<Event>) {
        if cell_id.is_empty() {
            return (
                self.error(
                    "err.empty_cell_id",
                    "单元格 ID 不能为空",
                    "Cell IDs must not be empty",
                ),
                vec![],
            );
        }
        let index = if let Some(i) = self.notebook.cells.iter().position(|c| c.id == cell_id) {
            if self.notebook.cells[i].kind != CellKind::Math {
                return (
                    self.error(
                        "err.non_math_cell",
                        "只有数学单元格可以执行 CAS 求值",
                        "Only Math cells can run CAS evaluation",
                    ),
                    vec![],
                );
            }
            i
        } else {
            self.notebook.cells.push(Cell::from_input(CellInput {
                id: cell_id.clone(),
                kind: CellKind::Math,
                source: String::new(),
                dialect,
            }));
            self.notebook.cells.len() - 1
        };
        self.notebook.cells[index].source = source;
        self.notebook.cells[index].dialect = dialect;
        self.interrupt.store(false, Ordering::Relaxed);
        let run = self.run_cell(index);
        let allow_run = self.notebook.cells[index].status == CellStatus::Done;
        let mut events = vec![];
        let reran = if run.rejected {
            vec![]
        } else {
            self.cascade(run.changed, Some(index), allow_run, &mut events)
        };
        let output = self.notebook.cells[index]
            .output
            .clone()
            .expect("invariant: attempted execution stores output");
        (
            Response::Evaluated {
                cell_id,
                output,
                reran,
            },
            events,
        )
    }

    pub(super) fn run_all(&mut self) -> (Response, Vec<Event>) {
        self.interrupt.store(false, Ordering::Relaxed);
        for i in 0..self.notebook.cells.len() {
            self.analyze_cell(i);
        }
        let selected: BTreeSet<_> = self
            .notebook
            .cells
            .iter()
            .enumerate()
            .filter(|(_, c)| c.kind == CellKind::Math)
            .map(|(i, _)| i)
            .collect();
        let plan = self.plan(&selected);
        let mut failed = BTreeSet::new();
        let mut events = vec![];
        for i in selected {
            if self.config.general.reactive && plan.cycles.contains(&i) {
                let text = self.localized(
                    "err.cycle: 单元格之间存在循环依赖",
                    "err.cycle: Cyclic cell dependencies",
                );
                self.cell_error(i, "err.cycle", text);
                self.notify_status(i, CellStatus::Error, &mut events);
                self.notify_output(i, &mut events);
                failed.insert(i);
            } else if self.config.general.reactive
                && (plan.blocked.contains(&i)
                    || failed.iter().any(|&from| plan.edges[from].contains(&i)))
            {
                self.notify_status(i, CellStatus::Stale, &mut events);
                failed.insert(i);
            } else {
                self.notify_status(i, CellStatus::Queued, &mut events);
                self.notify_status(i, CellStatus::Running, &mut events);
                self.run_cell(i);
                let status = self.notebook.cells[i].status;
                if status != CellStatus::Done {
                    failed.insert(i);
                }
                self.notify_status(i, status, &mut events);
                self.notify_output(i, &mut events);
            }
        }
        (Response::Ok, events)
    }
}
