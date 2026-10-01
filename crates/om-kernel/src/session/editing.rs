//! Source edits preserve document order and keep live definitions separate.
use super::Session;
use crate::{Cell, protocol::*};

impl Session {
    pub(super) fn upsert_cell(&mut self, input: CellInput) -> (Response, Vec<Event>) {
        if input.id.is_empty() {
            return (
                self.error(
                    "err.empty_cell_id",
                    "单元格 ID 不能为空",
                    "Cell IDs must not be empty",
                ),
                vec![],
            );
        }
        let mut changed = Default::default();
        let i = if let Some(i) = self.notebook.cells.iter().position(|c| c.id == input.id) {
            let cell = &self.notebook.cells[i];
            if cell.kind == input.kind
                && cell.source == input.source
                && cell.dialect == input.dialect
            {
                return (Response::Ok, vec![]);
            }
            changed = self.cell_symbols(i);
            if cell.kind == CellKind::Math && input.kind != CellKind::Math {
                self.release_owned(&input.id);
            }
            let cell = &mut self.notebook.cells[i];
            cell.source = input.source;
            cell.dialect = input.dialect;
            cell.kind = input.kind;
            if input.kind != CellKind::Math {
                cell.output = None;
                cell.records.clear();
                cell.exec_count = None;
            }
            i
        } else {
            self.notebook.cells.push(Cell::from_input(input));
            self.notebook.cells.len() - 1
        };
        self.analyze_cell(i);
        changed.extend(self.cell_symbols(i));
        let status = if self.notebook.cells[i].kind == CellKind::Text {
            CellStatus::Done
        } else {
            CellStatus::Stale
        };
        self.notebook.cells[i].status = status;
        let mut events = vec![Event::CellStatus {
            cell_id: self.notebook.cells[i].id.clone(),
            status,
        }];
        if self.config.general.reactive {
            let affected = self.affected(changed);
            self.stale(&affected, Some(i), &mut events);
        }
        (Response::Ok, events)
    }

    pub(super) fn delete_cell(&mut self, id: CellId) -> (Response, Vec<Event>) {
        let Some(i) = self.notebook.cells.iter().position(|c| c.id == id) else {
            return (self.missing_cell(&id), vec![]);
        };
        let mut changed = self.cell_symbols(i);
        changed.extend(self.release_owned(&id));
        self.notebook.cells.remove(i);
        if self.config.general.reactive && self.config.general.auto_run_dependents {
            self.interrupt
                .store(false, std::sync::atomic::Ordering::Relaxed);
        }
        let mut events = vec![];
        self.cascade(changed, None, true, &mut events);
        (Response::Ok, events)
    }

    pub(super) fn move_cell(&mut self, id: CellId, to_index: u32) -> Response {
        let Some(i) = self.notebook.cells.iter().position(|c| c.id == id) else {
            return self.missing_cell(&id);
        };
        let Ok(to) = usize::try_from(to_index) else {
            return self.invalid_position(to_index);
        };
        if to >= self.notebook.cells.len() {
            return self.invalid_position(to_index);
        }
        if i != to {
            let cell = self.notebook.cells.remove(i);
            self.notebook.cells.insert(to, cell);
        }
        Response::Ok
    }

    fn missing_cell(&self, id: &str) -> Response {
        self.error(
            "err.cell_not_found",
            &format!("找不到单元格：{id}"),
            &format!("Cell not found: {id}"),
        )
    }
    fn invalid_position(&self, to: u32) -> Response {
        self.error(
            "err.cell_position",
            &format!("无效的单元格位置：{to}"),
            &format!("Invalid cell position: {to}"),
        )
    }
}
