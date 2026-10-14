//! Data-only explicit execution planning from the actual parsed notebook and live write owners.
use super::Session;
use crate::protocol::{CellKind, CellStatus};
use om_num::ctx::{Abort, Interrupt};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

fn tick(ctx: &Interrupt) -> Result<(), String> {
    ctx.tick().map_err(|e| {
        match e {
            Abort::Interrupted => "CANCELLED",
            Abort::Timeout => "TIMEOUT",
            Abort::Budget => "LIMIT_EXCEEDED",
        }
        .into()
    })
}

/// Requested extension of an explicit target set; this is not a host permission grant.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CellExecutionScope {
    /// Run exactly the selected cells; external unavailable prerequisites remain explicit.
    Selected,
    /// Add only the prerequisites that have not genuinely completed in this session.
    #[default]
    Required,
    /// Also rerun current dependents of the targets, then add their unavailable prerequisites.
    Affected,
}
/// A bounded frozen ordering. It contains no value, execution receipt or authority to accept state.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CellExecutionPlan {
    /// Exact original targets in the caller's order.
    pub requested_cell_ids: Vec<String>,
    /// Actual executable topological order; true cycles and their blocked descendants are separate.
    pub ordered_cell_ids: Vec<String>,
    /// Actual selected/expanded dependency edges (producer -> reader), in notebook order.
    pub edges: Vec<(String, String)>,
    /// Real cycle participants, distinct from blocked descendants.
    pub cycle_cell_ids: Vec<String>,
    /// Cells downstream of a cycle within this plan.
    pub blocked_cell_ids: Vec<String>,
    /// Outside prerequisites not completed; selected mode never silently runs them.
    pub unavailable_prerequisite_cell_ids: Vec<String>,
}
impl Session {
    /// Plan on an isolated reconciled working session off the source/UI owner. This reads the
    /// exact same live ownership/readiness facts as run_current_cell; no source is evaluated,
    /// definitions/random/Out/status are mutated, or notebook-wide RunAll request is issued.
    /// allowed_cell_ids is supplied by the native task owner, never by model arguments.
    pub fn plan_cell_execution(
        &self,
        ids: &[String],
        scope: CellExecutionScope,
        allowed_cell_ids: &BTreeSet<String>,
        ctx: &Interrupt,
    ) -> Result<CellExecutionPlan, String> {
        if ids.is_empty()
            || ids.len() > 50
            || ids.iter().collect::<BTreeSet<_>>().len() != ids.len()
        {
            return Err("INVALID_ARGUMENT".into());
        }
        // Index current parsed source once. Actual live-owner precedence is exactly the same as
        // prerequisite_cells; every potentially large traversal is interruptible and charged.
        let mut positions = BTreeMap::new();
        let mut writers = BTreeMap::<_, Vec<usize>>::new();
        let mut readers = BTreeMap::<_, Vec<usize>>::new();
        for (i, cell) in self.notebook.cells.iter().enumerate() {
            tick(ctx)?;
            positions.insert(cell.id.as_str(), i);
            if cell.kind == CellKind::Math {
                for &symbol in &cell.defines {
                    tick(ctx)?;
                    writers.entry(symbol).or_default().push(i);
                }
                for &symbol in &cell.uses {
                    tick(ctx)?;
                    readers.entry(symbol).or_default().push(i);
                }
            }
        }
        let prerequisites = |i: usize| -> Result<BTreeSet<usize>, String> {
            let mut required = BTreeSet::new();
            for symbol in &self.notebook.cells[i].uses {
                tick(ctx)?;
                if let Some(owner) = self.owners.get(symbol) {
                    if let Some(&index) = positions.get(owner.as_str()) {
                        required.insert(index);
                    }
                } else {
                    for &index in writers.get(symbol).into_iter().flatten() {
                        tick(ctx)?;
                        if index != i {
                            required.insert(index);
                        }
                    }
                }
            }
            Ok(required)
        };
        let mut selected = BTreeSet::new();
        for id in ids {
            tick(ctx)?;
            let i = self
                .notebook
                .cells
                .iter()
                .position(|c| &c.id == id)
                .ok_or("INVALID_CELL_REFERENCE")?;
            if self.notebook.cells[i].kind != CellKind::Math {
                return Err("INVALID_CELL_KIND".into());
            }
            selected.insert(i);
        }
        if scope == CellExecutionScope::Affected {
            // Same actual uses + current/owned writes as Session::affected. Index readers once
            // so a long chain can be cancelled and capped before a quadratic whole-file scan.
            let mut pending = selected
                .iter()
                .flat_map(|&i| self.cell_symbols(i))
                .collect::<Vec<_>>();
            let mut seen = BTreeSet::new();
            while let Some(symbol) = pending.pop() {
                tick(ctx)?;
                if !seen.insert(symbol) {
                    continue;
                }
                for &i in readers.get(&symbol).into_iter().flatten() {
                    tick(ctx)?;
                    if selected.insert(i) {
                        pending.extend(self.cell_symbols(i));
                    }
                    if selected.len() > 1000 {
                        return Err("LIMIT_EXCEEDED".into());
                    }
                }
            }
        }
        if scope != CellExecutionScope::Selected {
            loop {
                tick(ctx)?;
                let mut needed = BTreeSet::new();
                for &i in &selected {
                    tick(ctx)?;
                    needed.extend(
                        prerequisites(i)?.into_iter().filter(|&j| {
                            j != i && self.notebook.cells[j].status != CellStatus::Done
                        }),
                    );
                }
                let old = selected.len();
                selected.extend(needed);
                if selected.len() > 1000 {
                    return Err("LIMIT_EXCEEDED".into());
                }
                if selected.len() == old {
                    break;
                }
            }
        }
        if selected.len() > 1000 {
            return Err("LIMIT_EXCEEDED".into());
        }
        if selected
            .iter()
            .any(|&i| !allowed_cell_ids.contains(&self.notebook.cells[i].id))
        {
            return Err("PERMISSION_DENIED".into());
        }
        let mut unavailable = BTreeSet::new();
        let mut plan = self.plan(&selected);
        for &to in &selected {
            tick(ctx)?;
            // A callable may perform a global write owned by its actual caller, even when the
            // caller's source declares no binding. Preserve that real prerequisite edge.
            for from in prerequisites(to)? {
                if from == to {
                    continue;
                }
                if selected.contains(&from) {
                    plan.edges[from].insert(to);
                } else if self.notebook.cells[from].status != CellStatus::Done {
                    unavailable.insert(from);
                }
            }
        }
        let plan = Self::order_plan(&selected, plan.edges);
        let names = |set: &BTreeSet<usize>| {
            set.iter()
                .map(|&i| self.notebook.cells[i].id.clone())
                .collect()
        };
        let edges = selected
            .iter()
            .flat_map(|&from| {
                plan.edges[from].iter().map(move |&to| {
                    (
                        self.notebook.cells[from].id.clone(),
                        self.notebook.cells[to].id.clone(),
                    )
                })
            })
            .collect();
        Ok(CellExecutionPlan {
            requested_cell_ids: ids.to_vec(),
            ordered_cell_ids: plan
                .order
                .iter()
                .map(|&i| self.notebook.cells[i].id.clone())
                .collect(),
            edges,
            cycle_cell_ids: names(&plan.cycles),
            blocked_cell_ids: names(&plan.blocked),
            unavailable_prerequisite_cell_ids: names(&unavailable),
        })
    }
}
