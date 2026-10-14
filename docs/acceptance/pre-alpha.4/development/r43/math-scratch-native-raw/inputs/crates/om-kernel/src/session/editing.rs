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
            self.reset_interrupt();
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

impl Session {
    /// Replace a committed source file in one non-evaluating step. This is a kernel-owner API,
    /// not a new external request: the native coordinator calls it after durable acceptance.
    /// Old owned definitions are removed together; no old DeleteCell cascade can run midway.
    pub fn apply_source_file_without_evaluation(
        &mut self,
        file: NotebookFile,
    ) -> Result<crate::source::SourceInvalidation, String> {
        let before = self.notebook.to_file();
        let known = self
            .eval
            .defs
            .known_functions()
            .into_iter()
            .map(|s| s.name().to_owned())
            .collect::<Vec<_>>();
        let owned = self
            .owners
            .iter()
            .map(|(symbol, cell_id)| crate::source::SourceOwnership {
                symbol: symbol.name().to_owned(),
                cell_id: cell_id.clone(),
            })
            .collect::<Vec<_>>();
        let plan = crate::source::assess_source_change(
            &before,
            &file,
            self.config.general.dialect,
            self.config.general.constants,
            &known,
            &owned,
            false,
        )?;
        for owner in &plan.retire_owner_cells {
            self.release_owned(owner);
        }
        let mut previous = std::mem::take(&mut self.notebook.cells)
            .into_iter()
            .map(|c| (c.id.clone(), c))
            .collect::<std::collections::BTreeMap<_, _>>();
        let affected = plan
            .affected_cells
            .iter()
            .collect::<std::collections::BTreeSet<_>>();
        let mut cells = Vec::with_capacity(file.cells.len());
        for input in file.cells {
            let mut cell = if let Some(mut cell) = previous.remove(&input.id) {
                if cell.kind != input.kind && input.kind != CellKind::Math {
                    cell.output = None;
                    cell.records.clear();
                    cell.exec_count = None;
                }
                cell.kind = input.kind;
                cell.source = input.source;
                cell.dialect = input.dialect;
                cell
            } else {
                Cell::from_input(input)
            };
            if affected.contains(&cell.id) {
                cell.status = CellStatus::Stale;
            }
            if cell.kind == CellKind::Text {
                cell.status = CellStatus::Done;
            }
            if let Some(facts) = plan.analysis.cells.iter().find(|a| a.cell_id == cell.id) {
                cell.defines = facts
                    .defines
                    .iter()
                    .map(|n| om_core::Symbol::intern(n))
                    .collect();
                cell.uses = facts
                    .uses
                    .iter()
                    .map(|n| om_core::Symbol::intern(n))
                    .collect();
            }
            cells.push(cell);
        }
        self.notebook.title = file.title;
        self.notebook.cells = cells;
        Ok(plan)
    }
}

impl Session {
    /// Apply already accepted calculation settings without running cells. UI locale is independent;
    /// semantic settings retire old owned values and mark affected outputs stale.
    pub fn apply_calculation_settings_without_evaluation(
        &mut self,
        settings: crate::config::GeneralConfig,
    ) -> Result<crate::source::SourceInvalidation, String> {
        let changed = crate::source::calculation_settings_changed(&self.config.general, &settings);
        let file = self.notebook.to_file();
        let known = self
            .eval
            .defs
            .known_functions()
            .into_iter()
            .map(|s| s.name().to_owned())
            .collect::<Vec<_>>();
        let owned = self
            .owners
            .iter()
            .map(|(s, id)| crate::source::SourceOwnership {
                symbol: s.name().to_owned(),
                cell_id: id.clone(),
            })
            .collect::<Vec<_>>();
        let plan = crate::source::assess_source_change(
            &file,
            &file,
            settings.dialect,
            settings.constants,
            &known,
            &owned,
            changed,
        )?;
        for owner in &plan.retire_owner_cells {
            self.release_owned(owner);
        }
        self.config.general = settings;
        for cell in &mut self.notebook.cells {
            if plan.affected_cells.contains(&cell.id) {
                cell.status = CellStatus::Stale;
            }
            if let Some(facts) = plan.analysis.cells.iter().find(|f| f.cell_id == cell.id) {
                cell.defines = facts
                    .defines
                    .iter()
                    .map(|n| om_core::Symbol::intern(n))
                    .collect();
                cell.uses = facts
                    .uses
                    .iter()
                    .map(|n| om_core::Symbol::intern(n))
                    .collect();
            }
        }
        Ok(plan)
    }
}
