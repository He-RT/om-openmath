//! Owned session work candidates preserve real math/source/results, never host IO or model jobs.
use super::Session;
use crate::{
    KernelConfig,
    config::GeneralConfig,
    protocol::{CellKind, CellStatus, NotebookFile},
};
use std::sync::{Arc, atomic::AtomicBool};
/// Unaccepted independent session. Its private evaluator is writable; stored Explore forks stay readonly.
pub struct WorkingSession {
    pub(super) candidate: Session,
}
/// Actual end-of-cell facts; output/evidence remain in the candidate, pending host acceptance.
#[derive(Clone, Debug)]
pub struct CellBoundary {
    /// Stable selected cell ID.
    pub cell_id: String,
    /// Real status after the original parser/evaluator, including errors after partial effects.
    pub status: CellStatus,
    /// Successful statements retained before a terminal error/cancellation, not an effect count.
    pub successful_statements: usize,
    /// Last actual history index if a statement succeeded.
    pub out_index: Option<u32>,
}
impl WorkingSession {
    /// Apply current committed source/settings without calling Evaluate/RunAll/DeleteCell cascade.
    /// A skipped/reverted mathematical epoch retires all owned definitions conservatively.
    pub fn reconcile(
        &mut self,
        source: NotebookFile,
        general: GeneralConfig,
        retire_all: bool,
    ) -> Result<(), String> {
        self.candidate
            .apply_source_file_without_evaluation(source)?;
        self.candidate
            .apply_calculation_settings_without_evaluation(general)?;
        if retire_all {
            let owners = self
                .candidate
                .owners
                .values()
                .cloned()
                .collect::<std::collections::BTreeSet<_>>();
            for owner in owners {
                self.candidate.release_owned(&owner);
            }
            for cell in &mut self.candidate.notebook.cells {
                if cell.kind == CellKind::Math {
                    cell.status = CellStatus::Stale;
                }
            }
        }
        Ok(())
    }
    /// Execute exactly one current Math cell. Prerequisites must already be ready; no auto cascade.
    /// Errors keep actual statement/definition effects in this unaccepted working state.
    pub fn run_current_cell(&mut self, cell_id: &str) -> Result<CellBoundary, String> {
        let index = self
            .candidate
            .notebook
            .cells
            .iter()
            .position(|c| c.id == cell_id)
            .ok_or("INVALID_CELL_REFERENCE")?;
        if self.candidate.notebook.cells[index].kind != CellKind::Math {
            return Err("INVALID_CELL_KIND".into());
        }
        let names = self
            .candidate
            .eval
            .defs
            .known_functions()
            .iter()
            .map(|s| s.name().to_owned())
            .collect::<Vec<_>>();
        let analysis = crate::source::analyze_source(
            &self.candidate.notebook.to_file(),
            self.candidate.config.general.dialect,
            self.candidate.config.general.constants,
            &names,
        )?;
        if self.candidate.config.general.reactive
            && (analysis
                .cycles
                .iter()
                .chain(&analysis.blocked)
                .any(|id| id == cell_id)
                || analysis
                    .conflicts
                    .iter()
                    .any(|c| c.cell_ids.iter().any(|id| id == cell_id))
                || !self.candidate.prerequisites_ready(index))
        {
            return Err("DEPENDENCY_NOT_READY".into());
        }
        self.candidate.notebook.cells[index].status = CellStatus::Running;
        self.candidate.run_cell(index);
        let cell = &self.candidate.notebook.cells[index];
        Ok(CellBoundary {
            cell_id: cell.id.clone(),
            status: cell.status,
            successful_statements: cell.records.len(),
            out_index: cell.exec_count,
        })
    }
    /// Execute against the candidate; none of its definition/result mutations affect the parent.
    pub fn session_mut(&mut self) -> &mut Session {
        &mut self.candidate
    }
    /// Consume the candidate after host source/epoch/cancel and durable acceptance checks.
    /// This move itself certifies neither document authority nor persisted checkpoint state.
    pub fn into_session(self) -> Session {
        self.candidate
    }
}
impl Session {
    /// Read actual definition ownership for trusted non-evaluating source coordination.
    pub fn execution_context(&self) -> crate::source::SourceExecutionContext {
        let mut known_functions = self
            .eval
            .defs
            .known_functions()
            .iter()
            .map(|s| s.name().to_owned())
            .collect::<Vec<_>>();
        known_functions.sort();
        let mut owned = self
            .owners
            .iter()
            .map(|(symbol, cell_id)| crate::source::SourceOwnership {
                symbol: symbol.name().to_owned(),
                cell_id: cell_id.clone(),
            })
            .collect::<Vec<_>>();
        owned.sort_by(|a, b| a.symbol.cmp(&b.symbol));
        crate::source::SourceExecutionContext {
            known_functions,
            owned,
        }
    }
    /// Fork the actual idle working state with a fresh operation cancel token and no IO/AI owner.
    /// Definitions are cloned, never reexecuted from notebook text; record provenance is retained.
    pub fn fork_working_session(
        &self,
        cancel: Arc<AtomicBool>,
    ) -> Result<WorkingSession, om_eval::EvalError> {
        if Arc::ptr_eq(&cancel, &self.interrupt) {
            return Err(om_eval::EvalError::Other(
                "候选取消标志必须独立于主会话".into(),
            ));
        }
        if self
            .notebook
            .cells
            .iter()
            .any(|cell| cell.status == CellStatus::Running)
        {
            return Err(om_eval::EvalError::Other("工作候选需要空闲文档状态".into()));
        }
        let evaluator = self.eval.fork_working_stage()?.into_evaluator();
        let mut config = KernelConfig {
            general: self.config.general.clone(),
            ..KernelConfig::default()
        };
        config.llm.enabled = false;
        config.llm.profiles.clear();
        config.llm.translate.clear();
        config.llm.explain.clear();
        config.llm.chat.clear();
        config.llm.complete.clear();
        config.llm.fix.clear();
        let mut candidate = Session::with_cancel_token(config, self.clock.clone(), cancel);
        candidate.eval = evaluator;
        candidate.notebook = self.notebook.clone();
        candidate.owners = self.owners.clone();
        candidate.output_serial = self.output_serial;
        candidate.host_platform = self.host_platform;
        candidate.system_language = self.system_language;
        Ok(WorkingSession { candidate })
    }
}
